use std::{
    fmt::Display,
    net::{AddrParseError, IpAddr},
    str::FromStr,
    sync::OnceLock,
};

use regex::Regex;
use sqlx::{
    migrate::Migrator,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use thiserror::Error;
use tracing::{debug, error, trace};

const FQDN_RE_SRC: &str = r"^(?P<fqdn>(?:[a-zA-Z0-9-]{1,63}\.){1,}(?:[a-zA-Z0-9-]{1,63}))$";
static FQDN_RE: OnceLock<Regex> = OnceLock::new();
static MIGRATOR: Migrator = sqlx::migrate!("../../migrations");

#[tracing::instrument]
pub async fn create_recon_db_pool(
    host: &str,
    username: &str,
    password: Option<&str>,
    database: &str,
) -> Result<sqlx::postgres::PgPool, sqlx::migrate::MigrateError> {
    let recon_pg_connect_ops = if let Some(recon_db_password) = password {
        PgConnectOptions::new().password(recon_db_password)
    } else {
        PgConnectOptions::new()
    }
    .host(host)
    .username(username)
    .database(database);

    let recon_pg_pool = PgPoolOptions::new().connect_lazy_with(recon_pg_connect_ops);

    MIGRATOR.run(&recon_pg_pool).await?;

    Ok(recon_pg_pool)
}

#[derive(Debug, Clone)]
pub struct Fqdn(pub Vec<String>);

impl Fqdn {
    pub fn domain(&self) -> String {
        self.0[self.0.len() - 2..].join(".")
    }
}

impl<'a> From<&'a hickory_resolver::Name> for Fqdn {
    fn from(name: &'a hickory_resolver::Name) -> Self {
        let components = name
            .iter()
            .map(|lbl| String::from_utf8_lossy(lbl).to_string())
            .collect();

        Fqdn(components)
    }
}

impl FromStr for Fqdn {
    type Err = ParseFqdnError;

    #[tracing::instrument]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let fqdn_re = FQDN_RE.get_or_init(|| {
            debug!("Compiling the FQDN regular expression");
            Regex::new(FQDN_RE_SRC).expect("compiling the FQDN_RE_SRC regular expression")
        });

        trace!("Validating string length");
        if s.is_empty() || s.len() > 253 {
            error!("String is empty or longer than 253 characters: '{s}'");
            return Err(ParseFqdnError);
        }

        trace!("Validating string for FQDN format");
        let fqdn: Vec<String> = fqdn_re
            .captures(s)
            .and_then(|cap| cap.name("fqdn"))
            .map(|mat| mat.as_str().split('.'))
            .map(|splt| splt.map(|elmt| elmt.to_string()).collect())
            .ok_or(ParseFqdnError)?;

        #[cfg(feature = "strict-fqdn-validation")]
        {
            trace!("Validating string against illegal characters");
            if fqdn.iter().any(|label| {
                label.ends_with('-')
                    || label.contains("--")
                    || label.starts_with(['-', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9'])
            }) {
                error!("String contains illegal characters: '{}'", fqdn.join("."));
                return Err(ParseFqdnError);
            }
        }

        Ok(Fqdn(fqdn))
    }
}

impl Display for Fqdn {
    #[tracing::instrument(skip_all)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.join("."))
    }
}

#[derive(Debug, Error)]
#[error("expected a fully qualified domain name")]
pub struct ParseFqdnError;

#[derive(Debug, Clone)]
pub enum IpAddrOrFqdn {
    IpAddr(IpAddr),
    Fqdn(Fqdn),
}

impl FromStr for IpAddrOrFqdn {
    type Err = ParseIpAddrOrFqdnError;

    #[tracing::instrument]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match IpAddr::from_str(s) {
            Ok(ip_addr) => Ok(IpAddrOrFqdn::IpAddr(ip_addr)),
            Err(e1) => match Fqdn::from_str(s) {
                Ok(fqdn) => Ok(IpAddrOrFqdn::Fqdn(fqdn)),
                Err(e2) => Err(ParseIpAddrOrFqdnError(e1, e2)),
            },
        }
    }
}

impl Display for IpAddrOrFqdn {
    #[tracing::instrument(skip_all)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IpAddrOrFqdn::IpAddr(ip_addr) => write!(f, "{}", ip_addr)?,
            IpAddrOrFqdn::Fqdn(fqdn) => write!(f, "{}", fqdn)?,
        }

        Ok(())
    }
}

#[derive(Debug, Error)]
#[error("expected either an IP address or a fully qualified domain name: {}; {}", .0, .1)]
pub struct ParseIpAddrOrFqdnError(pub AddrParseError, pub ParseFqdnError);
