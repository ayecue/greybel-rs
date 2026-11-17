use std::time::Duration;

use crate::api::OriginPolicy;

/// Settings from environment variables.
#[derive(Debug, Clone)]
pub struct Config {
    /// `PORT`, 8080 by default.
    pub port: u16,
    /// `DATABASE_URL`; without it, workspaces are stored in memory.
    pub database_url: Option<String>,
    /// `DATABASE_MAX_CONNECTIONS`, 5 by default.
    pub max_connections: u32,
    /// `ALLOWED_ORIGINS`: `*` or a comma-separated list; `https://editor.greyscript.org` by default.
    pub allowed_origins: OriginPolicy,
    /// `SAVES_PER_MINUTE` per client IP, 30 by default.
    pub saves_per_minute: u32,
    pub save_window: Duration,
    /// `CLIENT_IP_HEADER`, `do-connecting-ip` by default (set by DigitalOcean App Platform); empty for none.
    pub client_ip_header: Option<String>,
    /// `LOG_FORMAT`: `text` (default) or `json`, one object per line.
    pub log_format: LogFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogFormat {
    #[default]
    Text,
    Json,
}

#[derive(Debug, thiserror::Error)]
#[error("{name} has an invalid value: {value}")]
pub struct ConfigError {
    name: &'static str,
    value: String,
}

impl Config {
    const DEFAULT_CLIENT_IP_HEADER: &str = "do-connecting-ip";
    const DEFAULT_ORIGIN: &str = "https://editor.greyscript.org";

    pub fn from_env() -> Result<Self, ConfigError> {
        Self::read(|name| std::env::var(name).ok())
    }

    pub fn read(get: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let client_ip_header =
            get("CLIENT_IP_HEADER").unwrap_or_else(|| Self::DEFAULT_CLIENT_IP_HEADER.to_owned());

        Ok(Self {
            port: Self::parse_number(&get, "PORT", 8080)?,
            database_url: get("DATABASE_URL").filter(|url| !url.is_empty()),
            max_connections: Self::parse_number(&get, "DATABASE_MAX_CONNECTIONS", 5)?,
            allowed_origins: OriginPolicy::parse(
                &get("ALLOWED_ORIGINS").unwrap_or_else(|| Self::DEFAULT_ORIGIN.to_owned()),
            ),
            saves_per_minute: Self::parse_number(&get, "SAVES_PER_MINUTE", 30)?,
            save_window: Duration::from_secs(60),
            client_ip_header: (!client_ip_header.is_empty()).then_some(client_ip_header),
            log_format: match get("LOG_FORMAT").as_deref().map(str::trim) {
                None | Some("" | "text") => LogFormat::Text,
                Some("json") => LogFormat::Json,
                Some(value) => {
                    return Err(ConfigError {
                        name: "LOG_FORMAT",
                        value: value.to_owned(),
                    });
                }
            },
        })
    }

    fn parse_number<N: std::str::FromStr>(
        get: &impl Fn(&str) -> Option<String>,
        name: &'static str,
        default: N,
    ) -> Result<N, ConfigError> {
        match get(name) {
            Some(value) => value
                .trim()
                .parse()
                .map_err(|_| ConfigError { name, value }),
            None => Ok(default),
        }
    }
}
