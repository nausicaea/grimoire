use anyhow::anyhow;
use std::{net::SocketAddr, str::FromStr};

use clap::Parser;
use futures::StreamExt;
use grimoire::{Fqdn, IpAddrOrFqdn};
use hickory_resolver::{
    config::{Protocol, ResolverConfig, ResolverOpts},
    AsyncResolver,
};
use tokio_util::codec::LinesCodec;
use tracing::{debug, warn};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(version, name = "dns-recon", about, long_about = None)]
struct Args {
    /// The port used by the DNS resolver to connect to the DNS server
    #[arg(short = 'p', long, env = "DNS_PORT", default_value_t = 53)]
    dns_port: u16,
    /// The IP address or fully qualified domain name of the DNS server
    #[arg(env = "DNS_SERVER")]
    dns_server: IpAddrOrFqdn,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    debug!("Parsing command line arguments");
    let args = Args::parse();

    debug!("Creating a stream from Stdin, decoded as lines, and parsed as FQDNs");
    debug!("Lines that don't parse as FQDNs are silently ignored");
    let mut fqdn_stream = tokio_util::codec::FramedRead::new(tokio::io::stdin(), LinesCodec::new())
        .map(|line| {
            line.map(|line| {
                Fqdn::from_str(&line)
                    .map_err(|e| {
                        warn!("{}, got '{}'", &e, &line);
                        e
                    })
                    .ok()
            })
            .transpose()
        });

    let dns_server = match &args.dns_server {
        IpAddrOrFqdn::IpAddr(dns_addr) => *dns_addr,
        IpAddrOrFqdn::Fqdn(dns_fqdn) => {
            debug!("Resolving the DNS IP address");
            let resolver = AsyncResolver::tokio_from_system_conf()?;
            resolver
                .lookup_ip(format!("{}.", &dns_fqdn))
                .await?
                .iter()
                .next()
                .ok_or_else(|| anyhow!("no IP address found for {}", &dns_fqdn))?
        }
    };

    debug!("Creating the resolver configuration");
    let mut resolver_config = ResolverConfig::new();
    resolver_config.add_name_server(hickory_resolver::config::NameServerConfig {
        socket_addr: SocketAddr::new(dns_server, args.dns_port),
        protocol: Protocol::Udp,
        tls_dns_name: None,
        trust_negative_responses: false,
        bind_addr: None,
    });

    debug!("Creating the resolver");
    let resolver = AsyncResolver::tokio(resolver_config, ResolverOpts::default());

    debug!("Performing the DNS query for the input");
    while let Some(maybe_fqdn) = fqdn_stream.next().await {
        let Some(result_fqdn) = maybe_fqdn else {
            continue;
        };

        let fqdn = result_fqdn?;
        if let Ok(lookup_ip) = resolver.lookup_ip(format!("{}.", &fqdn)).await {
            let ips = lookup_ip
                .iter()
                .map(|ip_addr| ip_addr.to_string())
                .collect::<Vec<_>>()
                .join(" ");
            println!("{} {}", &fqdn, ips);
        }
    }

    Ok(())
}
