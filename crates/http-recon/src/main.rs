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
use grimoire::{Fqdn, ParseFqdnError};
use itertools::Itertools;
use reqwest::{header::HeaderMap, redirect::Policy, ClientBuilder, Proxy};
use thiserror::Error;
use tokio_util::codec::{LinesCodec, LinesCodecError};
use tracing::{debug, error};
use tracing_subscriber::EnvFilter;

const MAX_HEADER_BUFFER_SIZE: usize = 1024 * 64;

#[derive(Debug, Parser)]
struct Args {
    #[arg(short, long, env = "PROXY")]
    proxy: Option<String>,
    #[arg(
        short,
        long,
        env = "USER_AGENT",
        default_value = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36"
    )]
    user_agent: String,
    #[arg(short, long, default_value_t = 10_u64)]
    timeout_secs: u64,
    #[arg(short, long, default_value_t = true)]
    accept_invalid_certs: bool,
}

#[derive(Debug)]
struct AnonymizedHttpHeader(HashMap<String, Vec<String>>);

impl<'a> From<&'a HeaderMap> for AnonymizedHttpHeader {
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

impl Display for AnonymizedHttpHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut output_buf = [0_u8; MAX_HEADER_BUFFER_SIZE];
        let map_str = serde_json::to_string(&self.0).map_err(|e| {
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

    let client = if let Some(proxy) = args.proxy {
        ClientBuilder::default().proxy(Proxy::all(proxy)?)
    } else {
        ClientBuilder::default()
    }
    .danger_accept_invalid_certs(args.accept_invalid_certs)
    .user_agent(&args.user_agent)
    .redirect(Policy::none())
    .timeout(Duration::from_secs(args.timeout_secs))
    .build()?;

    while let Some(target) = target_stream.next().await {
        let (target_fqdn, target_ip) = target?;
        let target_fqdn = target_fqdn.to_string();

        let http_request = client.head(format!("http://{target_fqdn}")).build()?;

        if let Ok(http_response) = client.execute(http_request).await {
            println!(
                "{target_fqdn} {target_ip} {} {} {}",
                http_response.url(),
                http_response.status().as_u16(),
                AnonymizedHttpHeader::from(http_response.headers())
            );
        }

        let https_request = client.head(format!("https://{target_fqdn}")).build()?;

        if let Ok(https_response) = client.execute(https_request).await {
            println!(
                "{target_fqdn} {target_ip} {} {} {}",
                https_response.url(),
                https_response.status().as_u16(),
                AnonymizedHttpHeader::from(https_response.headers())
            );
        }
    }

    Ok(())
}
