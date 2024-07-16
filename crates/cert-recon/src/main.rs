use std::time::Duration;

use clap::Parser;
use futures::StreamExt;
use grimoire::{Fqdn, IpAddrOrFqdn};
use log::LevelFilter;
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    raw_sql, ConnectOptions, Row,
};
use tracing::debug;
use tracing_subscriber::EnvFilter;

/// Queries certificate transparency logs for subdomains of a domain
#[derive(Debug, Parser)]
#[command(version, name = "dns-recon", about, long_about = None)]
struct Args {
    /// The IPv4 or IPv6 address the certificate transparency log (CT) service
    #[arg(long, default_value = "crt.sh", env = "CT_HOST")]
    ct_host: IpAddrOrFqdn,
    /// The username used for the PostgreSQL connection to the CT service
    #[arg(long, default_value = "guest", env = "CT_USERNAME")]
    ct_username: String,
    /// The PostgreSQL database to connect to when using the CT service
    #[arg(long, default_value = "certwatch", env = "CT_DATABASE")]
    ct_database: String,
    /// The domain to query
    domain: Fqdn,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    debug!("Parsing command line arguments");
    let args = Args::parse();

    debug!("Defining PostgreSQL connection settings");
    let ct_pg_connect_opts = PgConnectOptions::new_without_pgpass()
        .log_slow_statements(LevelFilter::Debug, Duration::from_secs(10))
        .ssl_mode(sqlx::postgres::PgSslMode::Require)
        .statement_cache_capacity(0)
        .host(&args.ct_host.to_string())
        .username(&args.ct_username)
        .database(&args.ct_database);

    debug!("Creating the PostgreSQL connection pool");
    let ct_pg_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy_with(ct_pg_connect_opts);

    debug!("Creating the SQL query");
    let raw_query = format!(
        r#"
        SELECT DISTINCT cai.NAME_VALUE
        FROM certificate_and_identities AS cai
        WHERE
            plainto_tsquery('certwatch', '{0}') @@ identities(cai.certificate)
            AND (cai.NAME_TYPE = '2.5.4.3' OR cai.NAME_TYPE LIKE 'san:%')
            AND cai.NAME_VALUE LIKE '%.{0}'
    "#,
        &args.domain
    );

    debug!("Fetching SQL query results");
    let mut result_stream = raw_sql(&raw_query).fetch(&ct_pg_pool);

    debug!("Printing SQL query results");
    while let Some(row) = result_stream.next().await {
        let row = row?;
        println!("{}", &row.get::<&str, _>(0));
    }

    Ok(())
}
