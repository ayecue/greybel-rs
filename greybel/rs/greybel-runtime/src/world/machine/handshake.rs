use std::cell::RefCell;
use std::rc::Rc;

use duplex_core::{Channel, Driver, ErrorCode, Peer, Protocol};
use tokio::sync::oneshot;

type Ack<P> = <<P as Peer>::Protocol as Protocol>::Ack;

/// Waits for `channel` to connect; returns the provider acknowledgement or the connection failure/close code.
pub(super) async fn ack<P: Peer, D: Driver>(
    channel: &Channel<P, D>,
) -> Result<Option<Ack<P>>, ErrorCode> {
    let (sender, receiver) = oneshot::channel();
    let sender = Rc::new(RefCell::new(Some(sender)));
    let settle = move |result: Result<Option<Ack<P>>, ErrorCode>| {
        if let Some(sender) = sender.borrow_mut().take() {
            let _ = sender.send(result);
        }
    };
    let hooks = [
        channel.on_ack({
            let settle = settle.clone();

            move |active| settle(Ok(active.ack().cloned()))
        }),
        channel.on_failure({
            let settle = settle.clone();

            move |failure| settle(Err(failure.reason()))
        }),
        channel.on_close(move |end| settle(Err(end.reason()))),
    ];
    let result = receiver.await.unwrap_or(Err(ErrorCode::ClosedChannel));

    for hook in hooks {
        channel.remove_hook(hook);
    }

    result
}
