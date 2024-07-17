use std::time::Duration;

use clap::Parser;
use futures::StreamExt;
use grimoire::{Fqdn, IpAddrOrFqdn};
use sqlx::{
    migrate::Migrator,
    postgres::{PgConnectOptions, PgPoolOptions},
    query, query_scalar, raw_sql, ConnectOptions, Row,
};
use tracing::{debug, info};
use tracing_subscriber::EnvFilter;

static MIGRATOR: Migrator = sqlx::migrate!("../../migrations");

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
    #[arg(long, default_value = "localhost", env = "RECON_DB_HOST")]
    recon_db_host: String,
    #[arg(long, default_value = "recon", env = "RECON_DB_USERNAME")]
    recon_db_username: String,
    #[arg(long, env = "RECON_DB_PASSWORD")]
    recon_db_password: Option<String>,
    #[arg(long, default_value = "recon", env = "RECON_DB_DATABASE")]
    recon_db_database: String,
    #[arg(short, long)]
    store_results: bool,
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

    let recon_pg_pool = if args.store_results {
        debug!("Establishing a connection to the recon database");
        let recon_pg_connect_ops = if let Some(recon_db_password) = &args.recon_db_password {
            PgConnectOptions::new().password(recon_db_password)
        } else {
            PgConnectOptions::new()
        }
        .host(&args.recon_db_host)
        .username(&args.recon_db_username)
        .database(&args.recon_db_database);

        let recon_pg_pool = PgPoolOptions::new().connect_lazy_with(recon_pg_connect_ops);

        MIGRATOR.run(&recon_pg_pool).await?;

        Some(recon_pg_pool)
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

    debug!("Printing SQL query results");
    'outer: while let Some(row) = result_stream.next().await {
        let row = row?;
        let cert_name_or_san = row.get::<&str, _>(0);
        println!("{}", &cert_name_or_san);

        if let Some(recon_pg_pool) = &recon_pg_pool {
            let recon_db_entry_count = query_scalar!(
                r#"SELECT COUNT(*) FROM "cert-recon" WHERE "cert-name" = $1"#,
                &cert_name_or_san
            )
            .fetch_one(recon_pg_pool)
            .await?
            .map(|c| c as usize)
            .unwrap_or(0_usize);

            if recon_db_entry_count > 0 {
                info!("'{cert_name_or_san}' already exists in the recon database");
                continue 'outer;
            }

            query!(
                r#"INSERT INTO "cert-recon" VALUES (DEFAULT, $1, $2)"#,
                &domain,
                &cert_name_or_san
            )
            .execute(recon_pg_pool)
            .await?;
        }
    }

    Ok(())
}
