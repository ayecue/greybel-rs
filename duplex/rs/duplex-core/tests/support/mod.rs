#![allow(dead_code)]

pub mod batch_stream;
pub mod binary;
pub mod cs;
pub mod dispatch;
pub mod handshake;
pub mod hub;
pub mod scenarios;
pub mod security;
pub mod streams;
pub mod tags;
pub mod ts;
pub mod validation;
pub mod versions;

use std::cell::RefCell;
use std::rc::Rc;

use duplex_core::{Channel, Health, Message as _, NextNotification, Protocol, Responder, Role};
use duplex_test_support::binary::bytes;
use duplex_test_support::conformance::calculator::{
    Add, Calculation, Calculator, CalculatorRouter, Log, LogRef, Progress,
};
use duplex_test_support::frames::{self, Frame, Slice};
use duplex_test_support::{Memory, MemoryQueue, recorder};

pub const CONNECTION: u32 = 1;
pub const CHANNEL: u32 = <Calculation as Protocol>::CHANNEL_ID;

pub fn batch(from: Role, connection: u32, frames: Vec<Frame>) -> Slice {
    Slice {
        core: frames::CORE,
        channel: CHANNEL,
        from: from as u8,
        connection,
        frames,
    }
}

pub fn log(text: &str) -> Log<'static> {
    Log {
        text: text.to_owned().into(),
    }
}

pub fn add_request(id: u32, a: i64, b: i64) -> Frame {
    Frame::Message {
        type_id: 1,
        id: Some(id),
        body: bytes(&Add { a, b }),
    }
}

pub fn healths<P: duplex_core::Peer>(channel: &Channel<P, Memory>) -> Rc<RefCell<Vec<Health>>> {
    let (healths, record) = recorder();
    let _ = channel.on_health(record);

    healths
}

pub type KeptAdd = Rc<RefCell<Option<(Add, Responder<i64>)>>>;

pub fn keep_add_requests(calculator: &Channel<Calculator, Memory>) -> KeptAdd {
    let pending = Rc::new(RefCell::new(None));

    calculator.handle({
        let pending = Rc::clone(&pending);

        move |add: Add, response, _next| *pending.borrow_mut() = Some((add, response))
    });

    pending
}

pub fn logs(calculator: &Channel<Calculator, Memory>) -> Rc<RefCell<Vec<String>>> {
    let (logged, record) = recorder();

    calculator.on::<Log>(move |log, _next| record(log.text.to_owned()));

    logged
}

pub struct Logger(pub Rc<RefCell<Vec<String>>>);

impl CalculatorRouter for Logger {
    fn on_log(&self, message: LogRef<'_>, next: NextNotification<Log<'static>>) {
        self.0.borrow_mut().push(message.text.to_string());
        next.call(message);
    }
}

pub fn notifications_then(count: usize, last: Frame) -> Vec<Frame> {
    let progress = validation::progress(Vec::new());
    let mut frames: Vec<_> = (0..count)
        .map(|_| Frame::Message {
            type_id: Progress::ID,
            id: None,
            body: bytes(&progress),
        })
        .collect();

    frames.push(last);

    frames
}

pub fn post_frames(queue: &MemoryQueue, frames: Vec<Frame>) {
    queue.post(
        Role::Client,
        batch(Role::Provider, CONNECTION, frames).encode(),
    );
}
