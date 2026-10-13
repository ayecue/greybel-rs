//! MCP stdio server for GreyScript and MiniScript API references, language-server diagnostics, builds and
//! transformations, and program execution in simulated worlds or Grey Hack.

mod server;

use rmcp::ServiceExt as _;
pub use server::{Server, ServerOptions};

#[derive(Debug, thiserror::Error)]
pub enum ServeError {
    #[error("{0}")]
    Start(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("{0}")]
    Wait(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// Serves tools configured by `options` on stdin/stdout until client disconnect. Requires a Tokio `LocalSet`.
pub async fn serve(options: ServerOptions) -> Result<(), ServeError> {
    let service = Server::new(options)
        .serve(rmcp::transport::stdio())
        .await
        .map_err(|error| ServeError::Start(Box::new(error)))?;

    service
        .waiting()
        .await
        .map(|_| ())
        .map_err(|error| ServeError::Wait(Box::new(error)))
}
