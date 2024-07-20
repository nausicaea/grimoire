use std::{
    collections::HashMap,
    fmt::Display,
    net::{AddrParseError, IpAddr},
    pin::pin,
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use clap::Parser;
use cookie::Cookie;
use futures::{FutureExt, StreamExt};
use grimoire::{create_recon_db_pool, Fqdn, ParseFqdnError};
use itertools::Itertools;
use reqwest::{header::HeaderMap, redirect::Policy, Proxy, Url};
use reqwest_leaky_bucket::leaky_bucket::RateLimiter;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use sqlx::{query, query_as, query_scalar, PgPool};
use thiserror::Error;
use tokio::io::stdin;
use tokio_util::codec::{FramedRead, LinesCodec, LinesCodecError};
use tracing::{debug, error, info, warn};
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
    enable_db_storage: bool,
    /// If enabled, run queries again even if the result is known. Ignored when results are not
    /// stored in the recon database
    #[arg(long)]
    query_known_fqdns: bool,
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
    /// Disable output to stdout
    #[arg(short, long)]
    quiet: bool,
}

#[derive(Debug, Default)]
struct CountPair {
    http_count: Option<i64>,
    https_count: Option<i64>,
}

#[tracing::instrument(skip(pg_pool))]
async fn is_fqdn_in_http_recon_db(pg_pool: &PgPool, fqdn: &Fqdn) -> (bool, bool) {
    let counts = query_as!(
        CountPair,
        r#"
        SELECT
            (SELECT COUNT(*) FROM "http-recon" WHERE "fqdn" = $1) AS http_count,
            (SELECT COUNT(*) FROM "https-recon" WHERE "fqdn" = $1) AS https_count;
        "#,
        fqdn.to_string(),
    )
    .fetch_one(pg_pool)
    .await
    .unwrap_or_default();

    (
        counts.http_count.unwrap_or(0) != 0,
        counts.https_count.unwrap_or(0) != 0,
    )
}

#[tracing::instrument(skip(pg_pool, headers))]
async fn submit_http_recon_results(
    pg_pool: &PgPool,
    fqdn: &Fqdn,
    url: &Url,
    response_status: u16,
    headers: Option<&AnonymizedHttpHeaders>,
) -> anyhow::Result<()> {
    let recon_db_entry_count = query_scalar!(
        r#"SELECT COUNT(*) FROM "http-recon" WHERE "fqdn" = $1"#,
        fqdn.to_string(),
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
        r#"INSERT INTO "http-recon" (id, fqdn, url, "response-status", headers, domain) VALUES (DEFAULT, $1, $2, $3, $4, $5)"#,
        fqdn.to_string(),
        url.to_string(),
        response_status as i32,
        headers
            .and_then(|h| serde_json::to_value(h).map_err(|e| error!("{}", e)).ok())
            .unwrap_or(serde_json::json!({})),
        fqdn.domain(),
    )
    .execute(pg_pool)
    .await?;

    Ok(())
}

#[tracing::instrument(skip(pg_pool, headers))]
async fn submit_https_recon_results(
    pg_pool: &PgPool,
    fqdn: &Fqdn,
    url: &Url,
    response_status: u16,
    headers: Option<&AnonymizedHttpHeaders>,
) -> anyhow::Result<()> {
    let recon_db_entry_count = query_scalar!(
        r#"SELECT COUNT(*) FROM "https-recon" WHERE "fqdn" = $1"#,
        fqdn.to_string(),
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
        r#"INSERT INTO "https-recon" (id, fqdn, url, "response-status", headers, domain) VALUES (DEFAULT, $1, $2, $3, $4, $5)"#,
        fqdn.to_string(),
        url.to_string(),
        response_status as i32,
        headers
            .and_then(|h| serde_json::to_value(h).map_err(|e| error!("{}", e)).ok())
            .unwrap_or(serde_json::json!({})),
        fqdn.domain(),
    )
    .execute(pg_pool)
    .await?;

    Ok(())
}

#[tracing::instrument(skip(pg_pool, client))]
async fn recon_http(
    pg_pool: Option<Arc<PgPool>>,
    client: Arc<ClientWithMiddleware>,
    fqdn: Arc<Fqdn>,
    ip: Arc<IpAddr>,
    query_known_fqdns: bool,
    quiet: bool,
) -> anyhow::Result<()> {
    let (skip_http_recon, skip_https_recon) = if let Some(recon_pg_pool) = &pg_pool {
        is_fqdn_in_http_recon_db(recon_pg_pool, &fqdn).await
    } else {
        (false, false)
    };

    if query_known_fqdns || !skip_http_recon {
        let url = Url::parse(&format!("http://{ip}"))?;
        let request = client
            .head(url.clone())
            .header(reqwest::header::HOST, fqdn.to_string())
            .build()?;

        match client.execute(request).await {
            Ok(response) => {
                let response_status = response.status().as_u16();
                let headers = AnonymizedHttpHeaders::from(response.headers());

                if !quiet {
                    println!("{fqdn} {ip} {url} {response_status} {headers}");
                }

                if let Some(recon_pg_pool) = &pg_pool {
                    submit_http_recon_results(
                        recon_pg_pool,
                        &fqdn,
                        &url,
                        response_status,
                        Some(&headers),
                    )
                    .await?;
                }
            }
            Err(e) => {
                debug!("Error when sending a request to '{}': {}", &url, e);
                if let Some(recon_pg_pool) = &pg_pool {
                    submit_http_recon_results(recon_pg_pool, &fqdn, &url, 0, None).await?;
                }
            }
        }
    }

    if query_known_fqdns || !skip_https_recon {
        let url = Url::parse(&format!("https://{ip}"))?;
        let request = client
            .head(url.clone())
            .header(reqwest::header::HOST, fqdn.to_string())
            .build()?;

        match client.execute(request).await {
            Ok(response) => {
                let response_status = response.status().as_u16();
                let headers = AnonymizedHttpHeaders::from(response.headers());

                if !quiet {
                    println!("{fqdn} {ip} {url} {response_status} {headers}");
                }

                if let Some(recon_pg_pool) = &pg_pool {
                    submit_https_recon_results(
                        recon_pg_pool,
                        &fqdn,
                        &url,
                        response_status,
                        Some(&headers),
                    )
                    .await?;
                }
            }
            Err(e) => {
                debug!("Error when sending a request to '{}': {}", &url, e);
                if let Some(recon_pg_pool) = &pg_pool {
                    submit_https_recon_results(recon_pg_pool, &fqdn, &url, 0, None).await?;
                }
            }
        }
    }
    Ok(())
}

#[derive(Debug, serde::Serialize)]
#[serde(transparent)]
struct AnonymizedHttpHeaders(HashMap<String, Vec<String>>);

impl<'a> From<&'a HeaderMap> for AnonymizedHttpHeaders {
    #[tracing::instrument(skip_all)]
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
    #[tracing::instrument(skip_all)]
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

    let recon_pg_pool = if args.enable_db_storage {
        debug!("Establishing a connection to the recon database");
        Some(Arc::new(
            create_recon_db_pool(
                &args.recon_db_host,
                &args.recon_db_username,
                args.recon_db_password.as_deref(),
                &args.recon_db_database,
            )
            .await?,
        ))
    } else {
        None
    };

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
    let client = Arc::new(
        ClientBuilder::new(client)
            .with(reqwest_leaky_bucket::rate_limit_all(limiter))
            .build(),
    );

    debug!("Creating a stream from Stdin, decoded as lines, and parsed as pairs FQDNs and IPs");
    info!("Lines that don't parse as pairs of FQDN and IP address are silently ignored");
    let query_known_fqdns = args.query_known_fqdns;
    let mut data_stream = pin!(FramedRead::new(stdin(), LinesCodec::new())
        .filter_map(|line_result| async move { line_result.map_err(|e| warn!("{e}")).ok() })
        .filter_map(|line| async move {
            line.split_once(' ')
                .ok_or(Error::InputSplit)
                .and_then(|(fqdn_str, ip_addr_str)| {
                    let fqdn = Arc::new(Fqdn::from_str(fqdn_str)?);
                    let ip_addr = Arc::new(IpAddr::from_str(ip_addr_str)?);

                    Ok((fqdn, ip_addr))
                })
                .map_err(|e| warn!("{e}"))
                .ok()
        })
        .flat_map_unordered(None, |(fqdn, ip_addr)| {
            Box::pin(
                recon_http(
                    recon_pg_pool.clone(),
                    client.clone(),
                    fqdn.clone(),
                    ip_addr.clone(),
                    query_known_fqdns,
                    args.quiet,
                )
                .into_stream(),
            )
        }));

    debug!("Starting HTTP(s) recon");
    while let Some(http_recon_result) = data_stream.next().await {
        http_recon_result?;
    }

    Ok(())
}
