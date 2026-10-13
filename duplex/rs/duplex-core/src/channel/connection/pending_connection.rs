use crate::definition::ErrorCode;
use crate::protocol::Protocol;

pub(crate) type OpenOutcome<Pr> =
    Result<<Pr as Protocol>::Ack, Result<<Pr as Protocol>::Refusal, ErrorCode>>;

type Settle<Pr> = Box<dyn FnOnce(OpenOutcome<Pr>)>;

/// Client open request awaiting provider acceptance or denial.
pub struct PendingConnection<Pr: Protocol> {
    settle: Option<Settle<Pr>>,
}

impl<Pr: Protocol> PendingConnection<Pr> {
    pub(crate) fn new(settle: impl FnOnce(OpenOutcome<Pr>) + 'static) -> Self {
        Self {
            settle: Some(Box::new(settle)),
        }
    }

    /// Accepts the client and sends the protocol `Ack` payload.
    pub fn accept(mut self, ack: Pr::Ack) {
        self.settle(Ok(ack));
    }

    /// Denies the client and sends the protocol `Refusal` payload.
    pub fn deny(mut self, refusal: Pr::Refusal) {
        self.settle(Err(Ok(refusal)));
    }

    fn settle(&mut self, outcome: OpenOutcome<Pr>) {
        if let Some(settle) = self.settle.take() {
            settle(outcome);
        }
    }
}

impl<Pr: Protocol> Drop for PendingConnection<Pr> {
    fn drop(&mut self) {
        self.settle(Err(Err(ErrorCode::DroppedMessage)));
    }
}
