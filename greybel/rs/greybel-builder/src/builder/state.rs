use duplex_core::{Channel, ChannelOptions, Driver, Transport};
use greybel_documents::PathResolution;
use greybel_documents_host_provider::HostProvider;
use greybel_documents_host_provider::protocol::Server;

use crate::protocol::Builder;

/// Builder and documents channels shared by request handlers and the build loop.
pub(crate) struct BuilderState<D: Driver> {
    pub builder: Channel<Builder, D>,
    pub server: Channel<Server, D>,
}

impl<D: Driver> BuilderState<D> {
    pub(super) fn new(transport: &Transport<D>) -> Self {
        Self {
            builder: Channel::new(transport, ChannelOptions::default()),
            server: Channel::new(transport, ChannelOptions::default()),
        }
    }

    pub(crate) fn provider(&self, resolution: PathResolution) -> HostProvider<D> {
        HostProvider::new(self.server.clone(), resolution)
    }
}
