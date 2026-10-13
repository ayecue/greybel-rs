use hyper::header::{HeaderMap, HeaderValue};

use super::TransportId;

/// Server-assigned transport ID header, sent by clients on every request after opening.
pub const TRANSPORT_HEADER: &str = "duplex-transport";
/// Header for the latest client-received batch sequence.
pub const RECEIVED_HEADER: &str = "duplex-received";

/// Client request transport ID parse result.
#[derive(Debug, Clone, Copy)]
pub(crate) enum TransportClaim {
    /// Missing transport ID; the initial POST opens a transport.
    Nothing,
    /// Valid transport ID.
    Transport(TransportId),
    /// Malformed transport ID.
    Invalid,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ClientRequest {
    pub(crate) transport: TransportClaim,
    /// Latest client-reported batch sequence, or 0 without the header.
    pub(crate) last_batch: u64,
    /// Whether a valid received-sequence header permits discarding acknowledged batches.
    pub(crate) acknowledges: bool,
}

impl ClientRequest {
    pub(crate) fn of(headers: &HeaderMap) -> Self {
        let transport = match headers.get(TRANSPORT_HEADER) {
            None => TransportClaim::Nothing,
            Some(id) => TransportId::parse(id.as_bytes())
                .map_or(TransportClaim::Invalid, TransportClaim::Transport),
        };
        let last_batch = headers.get(RECEIVED_HEADER).and_then(Self::number);

        Self {
            transport,
            last_batch: last_batch.unwrap_or(0),
            acknowledges: last_batch.is_some(),
        }
    }

    fn number(value: &HeaderValue) -> Option<u64> {
        value.to_str().ok()?.trim().parse().ok()
    }
}
