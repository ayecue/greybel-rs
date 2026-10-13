use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::future::{Future, poll_fn};
use std::rc::Rc;
use std::task::{Poll, Waker};

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::MessageChannel;

/// Completion flag and waker for a task awaiting an event-loop turn.
struct QueuedTask {
    ran: Rc<Cell<bool>>,
    waker: Rc<RefCell<Waker>>,
}

/// Yields worker tasks to the event loop through `MessageChannel`, before timer callbacks.
pub struct EventLoop {
    channel: MessageChannel,
    turns: RefCell<VecDeque<QueuedTask>>,
    _on_message: Closure<dyn FnMut()>,
}

thread_local! {
    static EVENT_LOOP: Option<EventLoop> = EventLoop::new();
}

impl EventLoop {
    /// Returns a future that yields to the event loop and resolves after its queued `MessageChannel` message is
    /// processed.
    pub fn yield_now() -> impl Future<Output = ()> {
        let ran = Rc::new(Cell::new(false));
        let mut waker: Option<Rc<RefCell<Waker>>> = None;

        poll_fn(move |context| {
            if ran.get() {
                return Poll::Ready(());
            }

            if let Some(waker) = &waker {
                waker.borrow_mut().clone_from(context.waker());

                return Poll::Pending;
            }

            let turn = QueuedTask {
                ran: Rc::clone(&ran),
                waker: Rc::new(RefCell::new(context.waker().clone())),
            };

            waker = Some(Rc::clone(&turn.waker));

            let queued = EVENT_LOOP.with(|event_loop| {
                event_loop
                    .as_ref()
                    .is_some_and(|event_loop| event_loop.queue(turn))
            });

            if queued {
                Poll::Pending
            } else {
                Poll::Ready(())
            }
        })
    }

    fn new() -> Option<Self> {
        let channel = MessageChannel::new().ok()?;
        let on_message = Closure::<dyn FnMut()>::new(|| {
            let next = EVENT_LOOP.with(|event_loop| {
                event_loop
                    .as_ref()
                    .and_then(|event_loop| event_loop.turns.borrow_mut().pop_front())
            });

            if let Some(turn) = next {
                turn.ran.set(true);
                turn.waker.borrow().wake_by_ref();
            }
        });

        channel
            .port1()
            .set_onmessage(Some(on_message.as_ref().unchecked_ref()));

        Some(Self {
            channel,
            turns: RefCell::new(VecDeque::new()),
            _on_message: on_message,
        })
    }

    fn queue(&self, turn: QueuedTask) -> bool {
        if self.channel.port2().post_message(&JsValue::NULL).is_err() {
            return false;
        }

        self.turns.borrow_mut().push_back(turn);

        true
    }
}
