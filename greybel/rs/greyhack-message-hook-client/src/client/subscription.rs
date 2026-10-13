use std::pin::Pin;
use std::sync::Weak;
use std::task::{Context, Poll};
use std::time::Duration;

use futures_core::Stream;
use tokio::sync::mpsc::UnboundedReceiver;

use super::ClientError;
use super::hub::Hub;
use crate::protocol::GreybelNotification;

pub type Filter = Box<dyn Fn(&GreybelNotification) -> bool + Send + Sync>;

pub struct Subscription {
    rx: UnboundedReceiver<GreybelNotification>,
    hub: Weak<Hub>,
    id: u64,
}

impl std::fmt::Debug for Subscription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Subscription")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

impl Subscription {
    pub(super) fn new(rx: UnboundedReceiver<GreybelNotification>, hub: Weak<Hub>, id: u64) -> Self {
        Self { rx, hub, id }
    }

    pub async fn next(&mut self) -> Option<GreybelNotification> {
        self.rx.recv().await
    }

    pub async fn next_message(
        &mut self,
        timeout: Option<Duration>,
    ) -> Result<GreybelNotification, ClientError> {
        let next = async { self.rx.recv().await.ok_or(ClientError::Disconnect) };

        match timeout {
            Some(limit) => tokio::time::timeout(limit, next)
                .await
                .map_err(|_| ClientError::Timeout(limit))?,
            None => next.await,
        }
    }
}

impl Stream for Subscription {
    type Item = GreybelNotification;

    fn poll_next(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<GreybelNotification>> {
        self.get_mut().rx.poll_recv(context)
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        if let Some(hub) = self.hub.upgrade() {
            hub.unsubscribe(self.id);
        }
    }
}
