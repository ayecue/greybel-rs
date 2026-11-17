use std::io::IsTerminal;
use std::net::SocketAddr;
use std::process::ExitCode;

use greybel_editor_service::api::{Api, RateLimiter};
use greybel_editor_service::config::{Config, LogFormat};
use greybel_editor_service::store::{MemoryStore, PostgresStore};
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> ExitCode {
    let config = match Config::from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("greybel-editor-service: {error}");

            return ExitCode::FAILURE;
        }
    };

    Logger::init(config.log_format);

    match Service::run(config).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!(%error, "the service stopped");

            ExitCode::FAILURE
        }
    }
}

/// Logs to stdout; `RUST_LOG` overrides the level.
struct Logger;

impl Logger {
    /// sqlx reports every start's `CREATE TABLE IF NOT EXISTS` of its migrations table as a notice.
    const DEFAULT_FILTER: &str = "info,sqlx::postgres::notice=warn";

    fn init(format: LogFormat) {
        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(Self::DEFAULT_FILTER));
        let subscriber = tracing_subscriber::fmt().with_env_filter(filter);

        match format {
            LogFormat::Text => subscriber.with_ansi(std::io::stdout().is_terminal()).init(),
            LogFormat::Json => subscriber
                .json()
                .flatten_event(true)
                .with_current_span(true)
                .with_span_list(false)
                .init(),
        }
    }
}

struct Service;

impl Service {
    async fn run(config: Config) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!(
            version = env!("CARGO_PKG_VERSION"),
            port = config.port,
            store = if config.database_url.is_some() { "postgres" } else { "memory" },
            origins = %config.allowed_origins,
            saves_per_minute = config.saves_per_minute,
            client_ip_header = config.client_ip_header.as_deref().unwrap_or(""),
            "starting"
        );

        let limiter = RateLimiter::new(
            config.saves_per_minute,
            config.save_window,
            config.client_ip_header.clone(),
        );
        let router = match &config.database_url {
            Some(url) => {
                let store = PostgresStore::connect(url, config.max_connections).await?;

                Api::new(store, limiter).router(&config.allowed_origins)
            }
            None => {
                tracing::warn!("DATABASE_URL is not set; workspaces are stored in memory");

                Api::new(MemoryStore::default(), limiter).router(&config.allowed_origins)
            }
        };
        let listener = TcpListener::bind(("0.0.0.0", config.port)).await?;

        tracing::info!(port = config.port, "listening");
        axum::serve(
            listener,
            router.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(Self::wait_for_shutdown())
        .await?;
        tracing::info!("stopped");

        Ok(())
    }

    async fn wait_for_shutdown() {
        let interrupt = tokio::signal::ctrl_c();

        #[cfg(unix)]
        {
            let Ok(mut terminate) =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            else {
                let _ = interrupt.await;

                return;
            };

            tokio::select! {
                _ = interrupt => tracing::info!(signal = "SIGINT", "shutting down, finishing open requests"),
                _ = terminate.recv() => tracing::info!(signal = "SIGTERM", "shutting down, finishing open requests"),
            }
        }

        #[cfg(not(unix))]
        {
            let _ = interrupt.await;

            tracing::info!("shutting down, finishing open requests");
        }
    }
}
