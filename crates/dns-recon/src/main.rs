use anyhow::anyhow;
use itertools::Itertools;
use sqlx::{query, query_scalar, types::ipnetwork::IpNetwork, PgPool};
use std::{
    net::{IpAddr, SocketAddr},
    pin::pin,
    str::FromStr,
};
use tokio::io::stdin;

use clap::Parser;
use futures::StreamExt;
use grimoire::{create_recon_db_pool, Fqdn, IpAddrOrFqdn};
use hickory_resolver::{
    config::{Protocol, ResolverConfig, ResolverOpts},
    AsyncResolver,
};
use tokio_util::codec::{FramedRead, LinesCodec};
use tracing::{debug, info};
use tracing_subscriber::EnvFilter;

/// Performs mass DNS resolution using the selected DNS server
#[derive(Debug, Parser)]
#[command(version, name = "dns-recon", about, long_about = None)]
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
    /// The port used by the DNS resolver to connect to the DNS server
    #[arg(short = 'p', long, env = "DNS_PORT", default_value_t = 53)]
    dns_port: u16,
    /// The IP address or fully qualified domain name of the DNS server
    #[arg(env = "DNS_SERVER")]
    dns_server: IpAddrOrFqdn,
}

#[tracing::instrument]
async fn submit_dns_recon_results(
    pg_pool: &PgPool,
    fqdn: &Fqdn,
    ips: &[IpAddr],
) -> anyhow::Result<()> {
    let fqdn = fqdn.to_string();

    let recon_db_entry_count = query_scalar!(
        r#"SELECT COUNT(*) FROM "dns-recon" WHERE "fqdn" = $1"#,
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

    let mut ip_networks = Vec::new();
    for ip in ips {
        ip_networks.push(IpNetwork::new(*ip, 32)?);
    }

    query!(
        r#"INSERT INTO "dns-recon" VALUES (DEFAULT, $1, $2)"#,
        &fqdn,
        &ip_networks
    )
    .execute(pg_pool)
    .await?;

    Ok(())
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

    debug!("Creating a stream from Stdin, decoded as lines, and parsed as FQDNs");
    info!("Lines that don't parse as FQDNs are silently ignored");
    let mut fqdn_stream = pin!(FramedRead::new(stdin(), LinesCodec::new())
        .filter_map(|line| async move { line.map(|line| Fqdn::from_str(&line).ok()).transpose() }));

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
    while let Some(fqdn) = fqdn_stream.next().await {
        let fqdn = fqdn?;
        if let Ok(lookup_ip) = resolver.lookup_ip(format!("{}.", &fqdn)).await {
            let ips: Vec<_> = lookup_ip.iter().collect();

            println!("{} {}", &fqdn, ips.iter().join(" "));

            if let Some(recon_pg_pool) = &recon_pg_pool {
                submit_dns_recon_results(recon_pg_pool, &fqdn, &ips).await?;
            }
        }
    }

    Ok(())
}