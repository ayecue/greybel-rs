use std::cell::RefCell;
use std::rc::Rc;

use duplex_core::{Channel, ChannelOptions, Error, IncomingStream, Responder};
use duplex_test_support::conformance::calculator::{Calculator, Count, Phase, Sum, User};
use duplex_test_support::conformance::sum_terms;
use duplex_test_support::{Memory, MemoryQueue, TaskQueue, recorder};

pub const WINDOW: u32 = 32;

/// Records the result of sending each counted item.
pub type Sent = Rc<RefCell<Vec<Result<u32, Error>>>>;

/// Installs spawned `Count` and `Sum` handlers on `calculator` and returns their task queue and send results.
pub fn counting(calculator: &Channel<Calculator, Memory>) -> (TaskQueue, Sent) {
    let tasks = TaskQueue::default();
    let sent = Rc::new(RefCell::new(Vec::new()));

    calculator.handle({
        let (tasks, sent) = (tasks.clone(), Rc::clone(&sent));

        move |count: Count, responder, _next| {
            let sent = Rc::clone(&sent);

            tasks.spawn(async move {
                let mut items = responder.start(Phase::Active);

                for number in 1..=count.to {
                    let result = items.send(number).await;
                    let failed = result.is_err();

                    sent.borrow_mut().push(result.map(|()| number));
                    if failed {
                        return;
                    }
                }

                items.end((1..=u64::from(count.to)).sum());
            });
        }
    });
    calculator.handle({
        let tasks = tasks.clone();

        move |_: Sum, stream, _next| tasks.spawn(sum_terms(stream))
    });

    (tasks, sent)
}

pub fn pair() -> (
    Channel<User, Memory>,
    Channel<Calculator, Memory>,
    MemoryQueue,
) {
    let options = ChannelOptions {
        stream_window: Some(WINDOW),
        ..ChannelOptions::default()
    };
    let (user, calculator, queue) = Memory::connected_pair_with_options::<User>(options);

    (user, calculator, queue)
}

pub type KeptSums = Rc<RefCell<Vec<(IncomingStream<(), i64>, Responder<i64>)>>>;

pub fn summing_later(calculator: &Channel<Calculator, Memory>) -> KeptSums {
    let (kept, record) = recorder();

    calculator.handle(move |_: Sum, (terms, responder), _next| record((terms, responder)));

    kept
}
