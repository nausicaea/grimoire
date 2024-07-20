use anyhow::anyhow;
use itertools::Itertools;
use sqlx::{query, query_scalar, types::ipnetwork::IpNetwork, PgPool};
use std::{
    net::{IpAddr, SocketAddr},
    pin::pin,
    str::FromStr,
    sync::Arc,
};
use tokio::io::stdin;

use clap::Parser;
use futures::{FutureExt, StreamExt};
use grimoire::{create_recon_db_pool, Fqdn, IpAddrOrFqdn};
use hickory_resolver::{
    config::{Protocol, ResolverConfig, ResolverOpts},
    error::ResolveErrorKind,
    AsyncResolver,
};
use tokio_util::codec::{FramedRead, LinesCodec};
use tracing::{debug, error, info, warn};
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
    enable_db_storage: bool,
    /// If enabled, run queries again even if the result is known. Ignored when the recon database
    /// integration is disabled
    #[arg(long)]
    query_known_fqdns: bool,
    /// The port used by the DNS resolver to connect to the DNS server
    #[arg(short = 'p', long, env = "DNS_PORT", default_value_t = 53)]
    dns_port: u16,
    /// Disable output to stdout
    #[arg(short, long)]
    quiet: bool,
    /// The IP address or fully qualified domain name of the DNS server
    #[arg(env = "DNS_SERVER")]
    dns_server: IpAddrOrFqdn,
}

#[tracing::instrument(skip(pg_pool))]
async fn is_fqdn_in_dns_recon_db(pg_pool: &PgPool, fqdn: &Fqdn) -> bool {
    query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM "dns-recon" WHERE fqdn = $1)"#,
        fqdn.to_string(),
    )
    .fetch_one(pg_pool)
    .await
    .map(|c| c.unwrap_or(false))
    .unwrap_or(false)
}

#[tracing::instrument(skip(pg_pool, ips))]
async fn submit_dns_recon_results(
    pg_pool: &PgPool,
    fqdn: &Fqdn,
    ips: &[IpAddr],
) -> anyhow::Result<()> {
    let mut ip_networks = Vec::new();
    for ip in ips {
        ip_networks.push(IpNetwork::new(*ip, 32)?);
    }

    query!(
        r#"
        INSERT INTO "dns-recon" (id, fqdn, ips, domain) 
        VALUES (DEFAULT, $1, $2, $3)
        ON CONFLICT ON CONSTRAINT "dns-recon_pkey" DO 
        UPDATE SET ips = (SELECT ARRAY(SELECT DISTINCT UNNEST("dns-recon".ips || EXCLUDED.ips)))
        "#,
        fqdn.to_string(),
        &ip_networks,
        fqdn.domain(),
    )
    .execute(pg_pool)
    .await
    .map_err(|e| {
        error!("'{fqdn}': {}", e);
        e
    })?;

    Ok(())
}

#[tracing::instrument(skip(pg_pool, query_known_results))]
async fn skip_known_fqdn(
    pg_pool: Option<Arc<PgPool>>,
    fqdn: Arc<Fqdn>,
    query_known_results: bool,
) -> bool {
    if let Some(pg_pool) = pg_pool {
        return query_known_results || !is_fqdn_in_dns_recon_db(&pg_pool, &fqdn).await;
    }
    true
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

    let dns_server = match &args.dns_server {
        IpAddrOrFqdn::IpAddr(dns_addr) => *dns_addr,
        IpAddrOrFqdn::Fqdn(dns_fqdn) => {
            debug!("Resolving the DNS server IP address");
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

    debug!("Creating a stream from Stdin, decoded as lines, and parsed as FQDNs");
    info!("Lines that don't parse as FQDNs are silently ignored");
    let query_known_fqdns = args.query_known_fqdns;
    let mut data_stream = pin!(FramedRead::new(stdin(), LinesCodec::new())
        .filter_map(|line_result| async move { line_result.map_err(|e| warn!("{e}")).ok() })
        .filter_map(|line| async move {
            Fqdn::from_str(&line)
                .map(Arc::new)
                .map_err(|e| warn!("{e}"))
                .ok()
        })
        .filter(|fqdn| skip_known_fqdn(recon_pg_pool.clone(), fqdn.clone(), query_known_fqdns))
        .flat_map(|fqdn| Box::pin(resolver.lookup_ip(format!("{}.", fqdn)).into_stream())));

    debug!("Performing the DNS query for the input");
    while let Some(lookup_result) = data_stream.next().await {
        match lookup_result {
            Ok(lookup_ip) => {
                let fqdn = Fqdn::from(lookup_ip.query().name());
                let ips: Vec<_> = lookup_ip.iter().collect();

                if !args.quiet {
                    println!("{} {}", &fqdn, ips.iter().join(" "));
                }

                if let Some(recon_pg_pool) = &recon_pg_pool {
                    submit_dns_recon_results(recon_pg_pool, &fqdn, &ips).await?;
                }
            }
            Err(e) => match e.kind() {
                ResolveErrorKind::NoRecordsFound { query, .. } => {
                    let fqdn = Fqdn::from(query.name());

                    debug!("Error resolving the FQDN '{}': {}", &fqdn, e);
                    if let Some(recon_pg_pool) = &recon_pg_pool {
                        submit_dns_recon_results(recon_pg_pool, &fqdn, &[]).await?;
                    }
                }
                _ => {
                    return Err(e.into());
                }
            },
        }
    }

    Ok(())
}
