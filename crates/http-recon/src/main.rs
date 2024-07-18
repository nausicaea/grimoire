use std::{
    collections::HashMap,
    fmt::Display,
    net::{AddrParseError, IpAddr},
    str::FromStr,
    time::Duration,
};

use clap::Parser;
use cookie::Cookie;
use futures::StreamExt;
use grimoire::{create_recon_db_pool, Fqdn, ParseFqdnError};
use itertools::Itertools;
use reqwest::{header::HeaderMap, redirect::Policy, Proxy, Url};
use reqwest_leaky_bucket::leaky_bucket::RateLimiter;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use sqlx::{query, query_scalar, PgPool};
use thiserror::Error;
use tokio_util::codec::{LinesCodec, LinesCodecError};
use tracing::{debug, error, info};
use tracing_subscriber::EnvFilter;

const MAX_HEADER_BUFFER_SIZE: usize = 1024 * 64;

/// Perform mass HTTP(s) connection attempts in order to reconnoiter an entire domain
#[derive(Debug, Parser)]
struct Args {
    /// The IPv4 or IPv6 address or the host name of the recon database service
    #[arg(long, default_value = "localhost", env = "RECON_DB_HOST")]
    recon_db_host: String,
    /// The username used for authenticating with the recon database service
    #[arg(long, default_value = "recon", env = "RECON_DB_USERNAME")]
    recon_db_username: String,
    /// The password used for authenticating with the recon database service
    #[arg(long, env = "RECON_DB_PASSWORD")]
    recon_db_password: Option<String>,
    /// The database to connect to when using the recon database service
    #[arg(long, default_value = "recon", env = "RECON_DB_DATABASE")]
    recon_db_database: String,
    /// If enabled, store the results in the recon database
    #[arg(short, long)]
    store_results: bool,
    /// If enabled, run queries again even if the result is known. Ignored when results are not
    /// stored in the recon database
    #[arg(long)]
    query_known_results: bool,
    /// Optionally proxy the HTTP(s) requests
    #[arg(short, long, env = "PROXY")]
    proxy: Option<String>,
    /// Define the user agent header used during HTTP(s) requests
    #[arg(
        short,
        long,
        env = "USER_AGENT",
        default_value = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36"
    )]
    user_agent: String,
    /// Define the total request timeout in seconds
    #[arg(short, long, default_value_t = 10_u64)]
    timeout_secs: u64,
    /// Define the number of requests performed per minute
    #[arg(short, long, default_value_t = 60_usize)]
    requests_per_minute: usize,
    /// Define the maximum number of requests that can be accumulated
    #[arg(short, long, default_value_t = 600_usize)]
    request_max_budget: usize,
    /// When connecting to HTTPS services, accept invalid certificates
    #[arg(short, long, default_value_t = true)]
    accept_invalid_certs: bool,
}

#[tracing::instrument]
async fn is_http_recon_result_in_db(pg_pool: &PgPool, fqdn: &Fqdn) -> bool {
    let recon_db_entry_count = query_scalar!(
        r#"SELECT COUNT(*) FROM "http-recon" WHERE "fqdn" = $1"#,
        fqdn.to_string(),
    )
    .fetch_one(pg_pool)
    .await
    .map(|c| c.map(|c| c as usize).unwrap_or(0_usize))
    .unwrap_or(0_usize);

    recon_db_entry_count != 0
}

#[tracing::instrument]
async fn is_https_recon_result_in_db(pg_pool: &PgPool, fqdn: &Fqdn) -> bool {
    let recon_db_entry_count = query_scalar!(
        r#"SELECT COUNT(*) FROM "https-recon" WHERE "fqdn" = $1"#,
        fqdn.to_string(),
    )
    .fetch_one(pg_pool)
    .await
    .map(|c| c.map(|c| c as usize).unwrap_or(0_usize))
    .unwrap_or(0_usize);

    recon_db_entry_count != 0
}

#[tracing::instrument]
async fn submit_http_recon_results(
    pg_pool: &PgPool,
    fqdn: &Fqdn,
    url: &Url,
    response_status: u16,
    headers: &AnonymizedHttpHeaders,
) -> anyhow::Result<()> {
    let fqdn = fqdn.to_string();

    let recon_db_entry_count = query_scalar!(
        r#"SELECT COUNT(*) FROM "http-recon" WHERE "fqdn" = $1"#,
        &fqdn
    )
    .fetch_one(pg_pool)
    .await?
    .map(|c| c as usize)
    .unwrap_or(0_usize);

    if recon_db_entry_count > 0 {
        info!("'{fqdn}' already exists in the recon database");
        return Ok(());
    }

    query!(
        r#"INSERT INTO "http-recon" VALUES (DEFAULT, $1, $2, $3, $4, $5)"#,
        &fqdn,
        url.to_string(),
        response_status as i32,
        serde_json::to_value(headers)?,
        url.domain(),
    )
    .execute(pg_pool)
    .await?;

    Ok(())
}

#[tracing::instrument]
async fn submit_https_recon_results(
    pg_pool: &PgPool,
    fqdn: &Fqdn,
    url: &Url,
    response_status: u16,
    headers: &AnonymizedHttpHeaders,
) -> anyhow::Result<()> {
    let fqdn = fqdn.to_string();

    let recon_db_entry_count = query_scalar!(
        r#"SELECT COUNT(*) FROM "https-recon" WHERE "fqdn" = $1"#,
        &fqdn
    )
    .fetch_one(pg_pool)
    .await?
    .map(|c| c as usize)
    .unwrap_or(0_usize);

    if recon_db_entry_count > 0 {
        info!("'{fqdn}' already exists in the recon database");
        return Ok(());
    }

    query!(
        r#"INSERT INTO "https-recon" VALUES (DEFAULT, $1, $2, $3, $4)"#,
        &fqdn,
        url.to_string(),
        response_status as i32,
        serde_json::to_value(headers)?,
    )
    .execute(pg_pool)
    .await?;

    Ok(())
}

#[tracing::instrument]
async fn recon_http(
    pg_pool: &Option<PgPool>,
    client: &ClientWithMiddleware,
    fqdn: &Fqdn,
    ip: &IpAddr,
    query_known_results: bool,
) -> anyhow::Result<()> {
    if let Some(recon_pg_pool) = pg_pool {
        if !query_known_results && is_http_recon_result_in_db(recon_pg_pool, fqdn).await {
            return Ok(());
        }
    }

    let http_request = client
        .head(format!("http://{ip}"))
        .header(reqwest::header::HOST, fqdn.to_string())
        .build()?;

    if let Ok(http_response) = client.execute(http_request).await {
        let request_url = http_response.url();
        let response_status = http_response.status().as_u16();
        let headers = AnonymizedHttpHeaders::from(http_response.headers());
        println!("{fqdn} {ip} {request_url} {response_status} {headers}",);

        if let Some(recon_pg_pool) = pg_pool {
            submit_http_recon_results(recon_pg_pool, fqdn, request_url, response_status, &headers)
                .await?;
        }
    }

    Ok(())
}

#[tracing::instrument]
async fn recon_https(
    pg_pool: &Option<PgPool>,
    client: &ClientWithMiddleware,
    fqdn: &Fqdn,
    ip: &IpAddr,
    query_known_results: bool,
) -> anyhow::Result<()> {
    if let Some(recon_pg_pool) = pg_pool {
        if !query_known_results && is_https_recon_result_in_db(recon_pg_pool, fqdn).await {
            return Ok(());
        }
    }

    let https_request = client
        .head(format!("https://{ip}"))
        .header(reqwest::header::HOST, fqdn.to_string())
        .build()?;

    if let Ok(https_response) = client.execute(https_request).await {
        let request_url = https_response.url();
        let response_status = https_response.status().as_u16();
        let headers = AnonymizedHttpHeaders::from(https_response.headers());
        println!("{fqdn} {ip} {request_url} {response_status} {headers}",);

        if let Some(recon_pg_pool) = pg_pool {
            submit_https_recon_results(recon_pg_pool, fqdn, request_url, response_status, &headers)
                .await?;
        }
    }

    Ok(())
}

#[derive(Debug, serde::Serialize)]
#[serde(transparent)]
struct AnonymizedHttpHeaders(HashMap<String, Vec<String>>);

impl<'a> From<&'a HeaderMap> for AnonymizedHttpHeaders {
    fn from(value: &'a HeaderMap) -> Self {
        let mut map = HashMap::default();
        let groups = value.iter().chunk_by(|(header, _)| *header);
        for (header, group) in groups.into_iter() {
            map.insert(
                header.to_string(),
                group
                    .map(|(_, value)| {
                        let utf8_value = String::from_utf8_lossy(value.as_bytes());
                        if header == reqwest::header::SET_COOKIE {
                            let mut cookie =
                                Cookie::parse(utf8_value).expect("when parsing a cookie");
                            cookie.set_value("");
                            cookie.to_string()
                        } else {
                            utf8_value.to_string()
                        }
                    })
                    .collect(),
            );
        }

        Self(map)
    }
}

impl Display for AnonymizedHttpHeaders {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut output_buf = [0_u8; MAX_HEADER_BUFFER_SIZE];
        let map_str = serde_json::to_string(&self).map_err(|e| {
            error!("serializing the header map to JSON: {}", e);
            std::fmt::Error
        })?;
        let mut encoder = match base64ct::Encoder::<base64ct::Base64>::new(&mut output_buf) {
            Ok(encoder) => encoder,
            Err(e) => {
                error!("creating the base64 encoder: {}", e);
                return write!(f, "...");
            }
        };
        if let Err(e) = encoder.encode(map_str.as_bytes()) {
            error!("encoding the header map as base64-encoded JSON: {}", e);
            return write!(f, "...");
        }
        let encoded_string = match encoder.finish() {
            Ok(encoded_string) => encoded_string,
            Err(e) => {
                error!("finishing the encoder job: {}", e);
                return write!(f, "...");
            }
        };
        write!(f, "{}", encoded_string)
    }
}

#[derive(Debug, Error)]
enum Error {
    #[error("Cannot split each line of the input exactly once with a whitespace")]
    InputSplit,
    #[error(transparent)]
    Codec(#[from] LinesCodecError),
    #[error(transparent)]
    Fqdn(#[from] ParseFqdnError),
    #[error(transparent)]
    IpAddr(#[from] AddrParseError),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    debug!("Parsing command line arguments");
    let args = Args::parse();

    let recon_pg_pool = if args.store_results {
        debug!("Establishing a connection to the recon database");
        Some(
            create_recon_db_pool(
                &args.recon_db_host,
                &args.recon_db_username,
                args.recon_db_password.as_deref(),
                &args.recon_db_database,
            )
            .await?,
        )
    } else {
        None
    };

    debug!("Creating a stream from Stdin, decoded as lines, and parsed as pairs FQDNs and IPs");
    let mut target_stream =
        tokio_util::codec::FramedRead::new(tokio::io::stdin(), LinesCodec::new()).map(
            |line_result| {
                line_result.map_err(Error::from).and_then(|line| {
                    line.split_once(' ')
                        .ok_or(Error::InputSplit)
                        .and_then(|(fqdn_str, ip_str)| {
                            let fqdn = Fqdn::from_str(fqdn_str)?;
                            let ip_addr = IpAddr::from_str(ip_str)?;

                            Ok((fqdn, ip_addr))
                        })
                })
            },
        );

    debug!("Creating the rate limiter");
    let limiter = RateLimiter::builder()
        .initial(0)
        .refill(args.requests_per_minute)
        .interval(Duration::from_secs(60))
        .max(args.request_max_budget)
        .build();

    debug!("Creating the reqwest HTTP client");
    let client = if let Some(proxy) = args.proxy {
        reqwest::ClientBuilder::default().proxy(Proxy::all(proxy)?)
    } else {
        reqwest::ClientBuilder::default()
    }
    .danger_accept_invalid_certs(args.accept_invalid_certs)
    .user_agent(&args.user_agent)
    .redirect(Policy::none())
    .timeout(Duration::from_secs(args.timeout_secs))
    .build()?;

    debug!("Wrapping the HTTP client to enable rate limiting");
    let client = ClientBuilder::new(client)
        .with(reqwest_leaky_bucket::rate_limit_all(limiter))
        .build();

    debug!("Starting HTTP(s) recon");
    while let Some(target) = target_stream.next().await {
        let (fqdn, ip) = target?;
        recon_http(
            &recon_pg_pool,
            &client,
            &fqdn,
            &ip,
            args.query_known_results,
        )
        .await?;
        recon_https(
            &recon_pg_pool,
            &client,
            &fqdn,
            &ip,
            args.query_known_results,
        )
        .await?;
    }

    Ok(())
}
