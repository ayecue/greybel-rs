//! Asynchronous client for Grey Hack's message-hook plugin.
//!
//! [`protocol`] contains game messages and plugin duplex bindings. [`Client`] sends messages, matches responses,
//! and manages notification subscriptions.
//!
//! ```no_run
//! # async fn run() -> Result<(), greyhack_message_hook_client::ClientError> {
//! use greyhack_message_hook_client::{Client, ClientOptions};
//!
//! let client = Client::connect(ClientOptions::default()).await?;
//! let status = client.health_check().await?;
//! println!("plugin {}", status.plugin_version);
//! client.shutdown().await;
//! # Ok(())
//! # }
//! ```

pub mod protocol;

mod client;

pub use client::{
    Build, BuildOutcome, Client, ClientError, ClientOptions, Context, ContextOptions, Evaluation,
    Filter, RuntimeState, Subscription,
};
pub use protocol::{
    BuildState, ContextBreakpoint, DebugEntry, DebugFrame, DebugValue, GreybelNotification,
    Payload, ScriptType, StackItem, StepMode,
};
