use std::cell::Cell;
#[cfg(feature = "security")]
use std::cell::RefCell;

use super::TransportState;
#[cfg(feature = "security")]
use super::{Transport, WeakTransport};
#[cfg(feature = "security")]
use crate::definition::ErrorCode;
use crate::driver::{Driver, InboundBatch, OutboundBatch};
use crate::protocol::SecurityLevel;
#[cfg(feature = "security")]
use crate::security::{ReceiveOutcome, Secure, Security};

#[derive(Default)]
pub(super) struct TransportSecurity {
    #[cfg(feature = "security")]
    secure: RefCell<Option<Secure>>,
    level: Cell<SecurityLevel>,
    signs_batches: Cell<bool>,
}

#[cfg(feature = "security")]
impl<D: Driver> Transport<D> {
    /// Creates a transport with [`Transport::new`] and applies `security`.
    pub fn with_security(driver: impl FnOnce(WeakTransport<D>) -> D, security: Security) -> Self {
        let transport = Self::new(driver);

        transport.0.security.level.set(security.level());
        transport
            .0
            .security
            .signs_batches
            .set(security.signs_batches());
        match Secure::new(security) {
            Ok((secure, first)) => {
                *transport.0.security.secure.borrow_mut() = Some(secure);
                if let Some(first) = first {
                    transport.0.driver.send(OutboundBatch::from(first));
                }
            }
            Err(_) => transport.0.end(ErrorCode::Insecure),
        }

        transport
    }
}

impl<D: Driver> TransportState<D> {
    pub(crate) fn matches_security(&self, level: SecurityLevel, signed: bool) -> bool {
        self.security.level.get() >= level && (!signed || self.security.signs_batches.get())
    }

    #[cfg_attr(
        not(feature = "security"),
        expect(
            clippy::unused_self,
            reason = "without the security feature there is no security to apply"
        )
    )]
    pub(super) fn can_send(&self) -> bool {
        #[cfg(feature = "security")]
        if let Some(secure) = &*self.security.secure.borrow() {
            return secure.is_established();
        }

        true
    }

    /// Creates an outbound batch from `bytes[start..]` and seals it if transport security is active.
    #[cfg_attr(
        not(feature = "security"),
        expect(
            clippy::unnecessary_wraps,
            reason = "without the security feature there is no security to apply"
        )
    )]
    pub(super) fn outbound(&self, bytes: Vec<u8>, start: usize) -> Option<OutboundBatch> {
        #[cfg(feature = "security")]
        if let Some(secure) = &mut *self.security.secure.borrow_mut() {
            let sealed_batch = secure.seal(&bytes[start..]);

            self.batches.release(bytes);

            return sealed_batch.map(OutboundBatch::from);
        }

        Some(OutboundBatch::new(bytes, start, &self.batches))
    }

    #[cfg_attr(
        not(feature = "security"),
        expect(
            clippy::unused_self,
            clippy::unnecessary_wraps,
            reason = "without the security feature there is no security to apply"
        )
    )]
    pub(super) fn inbound(&self, batch: InboundBatch) -> Option<InboundBatch> {
        #[cfg(feature = "security")]
        {
            let payload = match &mut *self.security.secure.borrow_mut() {
                Some(secure) => secure.receive(batch.as_slice()),
                None => return Some(batch),
            };

            match payload {
                ReceiveOutcome::Batch(batch) => return Some(batch.into()),
                ReceiveOutcome::Reply(reply) => self.driver.send(OutboundBatch::from(reply)),
                ReceiveOutcome::Nothing => {}
                ReceiveOutcome::Failure => {
                    self.end(ErrorCode::Insecure);

                    return None;
                }
            }

            self.flush_all();

            None
        }

        #[cfg(not(feature = "security"))]
        Some(batch)
    }
}
