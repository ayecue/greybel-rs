mod build;
mod connection;
mod context;
mod error;
mod hub;
mod id;
mod options;
mod subscription;

use std::sync::Arc;
use std::time::Duration;

pub use build::{Build, BuildOutcome};
use connection::Connection;
pub use context::{Context, ContextOptions, Evaluation, RuntimeState};
pub use error::ClientError;
use hub::Hub;
use id::IdGenerator;
pub use options::ClientOptions;
pub use subscription::{Filter, Subscription};
use tokio::sync::Mutex;

use crate::protocol::GreybelNotification;
use crate::protocol::greyhack::{Greybel, HealthCheckRpc, StatusRpc};

struct Inner {
    hub: Arc<Hub>,
    connection: Mutex<Option<Connection>>,
    request_timeout: Duration,
    ids: IdGenerator,
}

impl Drop for Inner {
    fn drop(&mut self) {
        self.hub.close();
    }
}

#[derive(Clone)]
pub struct Client {
    inner: Arc<Inner>,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("connected", &self.is_connected())
            .finish_non_exhaustive()
    }
}

impl Client {
    pub async fn connect(options: ClientOptions) -> Result<Self, ClientError> {
        let hub = Arc::new(Hub::default());
        let connection = Connection::connect(&options, Arc::clone(&hub)).await?;

        Ok(Self {
            inner: Arc::new(Inner {
                hub,
                connection: Mutex::new(Some(connection)),
                request_timeout: options.request_timeout,
                ids: IdGenerator::default(),
            }),
        })
    }

    #[must_use]
    pub fn is_connected(&self) -> bool {
        !self.inner.hub.is_closed()
    }

    #[must_use]
    pub fn request_timeout(&self) -> Duration {
        self.inner.request_timeout
    }

    pub fn subscribe(
        &self,
        filter: impl Fn(&GreybelNotification) -> bool + Send + Sync + 'static,
    ) -> Subscription {
        self.inner.hub.subscribe(Box::new(filter))
    }

    /// Sends a message for plugin handling through the client connection.
    pub async fn send<M>(&self, message: &M) -> Result<(), ClientError>
    where
        M: duplex_core::Message<Sender = Greybel> + Clone + Send + 'static,
    {
        if !self.is_connected() {
            return Err(ClientError::Disconnect);
        }

        let connection = self.inner.connection.lock().await;
        let connection = connection.as_ref().ok_or(ClientError::Disconnect)?;
        let message = message.clone();

        connection
            .send(Box::new(move |channel| channel.send(&message).map(drop)))
            .await
    }

    /// Subscribes before sending `message`, then awaits the first matching notification until `timeout` or the
    /// client's default request timeout.
    pub async fn request<M>(
        &self,
        message: &M,
        matches: impl Fn(&GreybelNotification) -> bool + Send + Sync + 'static,
        timeout: Option<Duration>,
    ) -> Result<GreybelNotification, ClientError>
    where
        M: duplex_core::Message<Sender = Greybel> + Clone + Send + 'static,
    {
        let mut subscription = self.subscribe(matches);

        self.send(message).await?;

        subscription
            .next_message(Some(timeout.unwrap_or(self.inner.request_timeout)))
            .await
    }

    pub async fn health_check(&self) -> Result<StatusRpc<'static>, ClientError> {
        let notification = self
            .request(
                &HealthCheckRpc {},
                |n| matches!(n, GreybelNotification::StatusRpc(_)),
                None,
            )
            .await?;

        match notification {
            GreybelNotification::StatusRpc(status) => Ok(status),
            other => Err(ClientError::UnexpectedResponse(other.name())),
        }
    }

    pub async fn create_build(
        &self,
        destination: &str,
        resource_destination: &str,
        auto_compile: bool,
    ) -> Result<Build, ClientError> {
        Build::create(
            self.clone(),
            destination,
            resource_destination,
            auto_compile,
        )
        .await
    }

    pub async fn create_context(&self, options: ContextOptions) -> Result<Context, ClientError> {
        Context::create(self.clone(), options).await
    }

    pub async fn shutdown(&self) {
        self.inner.connection.lock().await.take();
        self.inner.hub.close();
    }

    fn new_id(&self) -> String {
        self.inner.ids.generate()
    }
}
