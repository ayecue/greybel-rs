mod connection;
mod operations;
mod request;

use connection::Connection;
pub use operations::ExecutionStep;
pub use request::GuildSetup;
use request::RequestKind;

use crate::protocol::test_support::Reply;
use crate::{TestClientError, TestClientOptions};

#[derive(Clone)]
pub struct TestClient {
    connection: Connection,
}

impl TestClient {
    pub async fn connect(options: TestClientOptions) -> Result<Self, TestClientError> {
        Ok(Self {
            connection: Connection::connect(options).await?,
        })
    }

    async fn request(&self, kind: RequestKind) -> Result<Reply, TestClientError> {
        self.connection.request(kind).await
    }

    pub fn shutdown(self) {
        drop(self);
    }
}
