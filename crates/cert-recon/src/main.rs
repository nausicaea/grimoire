use std::time::Duration;

use clap::Parser;
use futures::StreamExt;
use grimoire::{create_recon_db_pool, Fqdn, IpAddrOrFqdn};
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    query, query_scalar, raw_sql, ConnectOptions, PgPool, Row,
};
use tracing::{debug, info};
use tracing_subscriber::EnvFilter;

/// Queries certificate transparency logs for subdomains of a domain
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
    /// The IPv4 or IPv6 address or the FQDN of the certificate transparency log (CT) service
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

#[tracing::instrument]
async fn submit_cert_recon_results(
    pg_pool: &PgPool,
    domain: &str,
    cert_name: &str,
) -> Result<(), sqlx::Error> {
    let recon_db_entry_count = query_scalar!(
        r#"SELECT COUNT(*) FROM "cert-recon" WHERE "cert-name" = $1"#,
        cert_name
    )
    .fetch_one(pg_pool)
    .await?
    .map(|c| c as usize)
    .unwrap_or(0_usize);

    if recon_db_entry_count > 0 {
        info!("'{cert_name}' already exists in the recon database");
        return Ok(());
    }

    query!(
        r#"INSERT INTO "cert-recon" VALUES (DEFAULT, $1, $2)"#,
        domain,
        cert_name
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

    debug!("Defining PostgreSQL connection settings for Certwatch");
    let ct_pg_connect_opts = PgConnectOptions::new_without_pgpass()
        .log_slow_statements(log::LevelFilter::Debug, Duration::from_secs(10))
        .ssl_mode(sqlx::postgres::PgSslMode::Require)
        .statement_cache_capacity(0)
        .host(&args.ct_host.to_string())
        .username(&args.ct_username)
        .database(&args.ct_database);

    debug!("Creating the PostgreSQL connection pool for Certwatch");
    let ct_pg_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy_with(ct_pg_connect_opts);

    debug!("Creating the SQL query for Certwatch");
    let domain = args.domain.to_string();
    let raw_query = format!(
        r#"
        SELECT DISTINCT cai.NAME_VALUE
        FROM certificate_and_identities AS cai
        WHERE
            plainto_tsquery('certwatch', '{0}') @@ identities(cai.certificate)
            AND (cai.NAME_TYPE = '2.5.4.3' OR cai.NAME_TYPE LIKE 'san:%')
            AND cai.NAME_VALUE LIKE '%.{0}'
    "#,
        &domain
    );

    debug!("Fetching SQL query results");
    let mut result_stream = raw_sql(&raw_query).fetch(&ct_pg_pool);

    debug!("Evaluating SQL query results");
    while let Some(row) = result_stream.next().await {
        let row = row?;
        let cert_name_or_san = row.get::<&str, _>(0);
        println!("{}", &cert_name_or_san);

        if let Some(recon_pg_pool) = &recon_pg_pool {
            submit_cert_recon_results(recon_pg_pool, &domain, cert_name_or_san).await?;
        }
    }

    Ok(())
}
