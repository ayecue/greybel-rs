use std::rc::Rc;

use super::state::ChannelState;
use crate::connection_id::ConnectionId;
use crate::definition::{ErrorCode, FrameKind};
use crate::driver::Driver;
use crate::frame::{CORE_MAJOR, Control, Frame, FrameHead, FrameReader};
use crate::protocol::Peer;

#[derive(Default)]
pub(super) struct Reassembly {
    bytes: Vec<u8>,
    dropping: bool,
}

impl<P: Peer, D: Driver> ChannelState<P, D> {
    pub(super) fn on_fragment(
        self: &Rc<Self>,
        connection: Option<ConnectionId>,
        core: u8,
        bytes: &[u8],
        ends: bool,
    ) {
        let batch_bytes = {
            let mut fragments = self.fragments.borrow_mut();
            let reassembly = fragments.get_or_insert_with(Reassembly::default);

            if reassembly.bytes.is_empty() && !reassembly.dropping {
                let head = FrameHead::read(bytes);
                let is_oversized = |head: FrameHead| {
                    head.length > self.settings.max_message && !Control::is_control(head.kind)
                };

                if head.is_none_or(is_oversized) {
                    reassembly.dropping = true;
                    drop(fragments);
                    if let Some(head) = head {
                        self.refuse_over_limit(connection, Some(head.kind), head.id);
                    }

                    return self.discard_reassembly(ends);
                }
            }

            if reassembly.dropping {
                drop(fragments);

                return self.discard_reassembly(ends);
            }

            reassembly.bytes.extend_from_slice(bytes);
            if !ends {
                return;
            }

            fragments.take().map(|reassembly| reassembly.bytes)
        };
        let Some(batch_bytes) = batch_bytes else {
            return;
        };
        let batch: Rc<[u8]> = Rc::from(batch_bytes);
        let mut active = false;

        for frame in FrameReader::new(&batch, 0..batch.len()) {
            let Ok(frame) = frame else {
                return;
            };

            if core != CORE_MAJOR
                && !matches!(&frame, Frame::Control(control) if control.is_frozen())
            {
                continue;
            }

            if !matches!(frame, Frame::Fragment { .. }) {
                self.route(connection, frame, &batch, &mut active);
            }
        }
    }

    fn discard_reassembly(&self, ends: bool) {
        if ends {
            self.fragments.replace(None);
        }
    }

    pub(super) fn refuse_over_limit(
        self: &Rc<Self>,
        connection: Option<ConnectionId>,
        kind: Option<FrameKind>,
        id: Option<u32>,
    ) {
        let (Some(connection), Some(kind), Some(id)) = (connection, kind, id) else {
            return;
        };

        match kind {
            FrameKind::Message => self.write_rejection(connection, id, ErrorCode::Overload),
            FrameKind::MessageItem | FrameKind::MessageEnd => {
                let exchange_id = self.messages.borrow().inbound_id(id);

                if let Some(id) = exchange_id {
                    self.reject_inbound(id, ErrorCode::Overload);
                }
            }
            FrameKind::ResponseStart
            | FrameKind::ResponseItem
            | FrameKind::ResponseEnd
            | FrameKind::Failure => {
                let exchange_id = self.messages.borrow().outbound_id(id);

                if let Some(id) = exchange_id {
                    self.cancel_outbound(id, ErrorCode::Overload);
                }
            }
            _ => {}
        }
    }
}
