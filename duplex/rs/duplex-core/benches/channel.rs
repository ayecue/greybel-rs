use std::cell::RefCell;
use std::hint::black_box;
use std::rc::Rc;

use calculator::{
    Add, CalculatorRouter, Count, Divide, Log, Phase, Progress, Quotient, Step, Sum, User,
};
use criterion::{Criterion, criterion_group, criterion_main};
use duplex_core::{Bundle, SendOptions};
use duplex_test_support::conformance::{Adder, calculator, count_to, sum_terms};
use duplex_test_support::future::ready_output;
use duplex_test_support::{Memory, TaskQueue};

const MESSAGES: i64 = 1000;

fn responses(criterion: &mut Criterion) {
    let (user, calculator, queue) = Memory::connected_pair::<User>();

    queue.seal();
    Adder.attach(&calculator);
    criterion.bench_function("1000 messages and their responses", |bench| {
        bench.iter(|| {
            let pending: Vec<_> = (0..MESSAGES)
                .map(|a| user.send(&Add { a, b: 1 }).expect("the channel sends"))
                .collect();

            queue.deliver();
            for response in pending {
                black_box(ready_output(response).expect("the provider responds"));
            }
        });
    });
}

fn keyed_responses(criterion: &mut Criterion) {
    let (user, calculator, queue) = Memory::connected_pair::<User>();

    queue.seal();
    Adder.attach(&calculator);
    criterion.bench_function("1000 messages each in a bundle of its own", |bench| {
        bench.iter(|| {
            let pending: Vec<_> = (0..MESSAGES)
                .map(|a| {
                    let options = SendOptions {
                        bundle: Some(Bundle::Key(
                            u32::try_from(a).expect("the message number fits a bundle key") + 1,
                        )),
                    };

                    user.send_with(&Add { a, b: 1 }, options)
                        .expect("the channel sends")
                })
                .collect();

            queue.deliver();
            for response in pending {
                black_box(ready_output(response).expect("the provider responds"));
            }
        });
    });
}

fn notifications(criterion: &mut Criterion) {
    let (user, calculator, queue) = Memory::connected_pair::<User>();

    queue.seal();
    calculator.on::<Log>(|log, _next| {
        black_box(log);
    });

    let log = Log {
        text: "a line for the log".into(),
    };

    criterion.bench_function("1000 notifications", |bench| {
        bench.iter(|| {
            for _ in 0..MESSAGES {
                user.send(&log).expect("the channel sends");
            }

            queue.deliver();
        });
    });
}

fn nested_notifications(criterion: &mut Criterion) {
    let (user, calculator, queue) = Memory::connected_pair::<User>();

    queue.seal();
    user.on::<Progress>(|progress, _next| {
        black_box(progress);
    });

    let step = |name: &str| Step {
        name: name.to_owned(),
        result: Some(Quotient::WithRemainder {
            value: 2,
            remainder: 1,
        }),
        duration: Some(3),
    };
    let progress = Progress {
        phase: Phase::Active,
        done_share: Some(0.5),
        steps: vec![step("divide"), step("add"), step("wait")].into(),
        counts: [("lines".to_owned(), 3)].into_iter().collect(),
        note: Some(Some("almost".to_owned().into())),
        eta: Some(2.5),
    };

    criterion.bench_function("1000 nested notifications", |bench| {
        bench.iter(|| {
            for _ in 0..MESSAGES {
                calculator.send(&progress).expect("the channel sends");
            }

            queue.deliver();
        });
    });
}

fn streamed_response(criterion: &mut Criterion) {
    let (user, calculator, queue) = Memory::connected_pair::<User>();

    queue.seal();

    let tasks = TaskQueue::default();

    calculator.handle({
        let tasks = tasks.clone();

        move |count: Count, responder, _next| {
            tasks.spawn(count_to(count.to, Phase::Active, responder));
        }
    });
    criterion.bench_function("a response of 1000 items", |bench| {
        bench.iter(|| {
            let mut stream = user.send(&Count { to: 1000 }).expect("the channel sends");
            let read = Rc::new(RefCell::new(0));

            tasks.spawn({
                let read = Rc::clone(&read);

                async move {
                    while let Some(item) = stream.next().await {
                        *read.borrow_mut() += black_box(item.expect("the streamed item reads"));
                    }
                }
            });
            tasks.run(&queue);
            assert_eq!(*read.borrow(), 500_500);
        });
    });
}

fn message_stream(criterion: &mut Criterion) {
    let (user, calculator, queue) = Memory::connected_pair::<User>();

    queue.seal();

    let tasks = TaskQueue::default();

    calculator.handle({
        let tasks = tasks.clone();

        move |_: Sum, stream, _next| tasks.spawn(sum_terms(stream))
    });
    criterion.bench_function("a stream of 1000 items after a message", |bench| {
        bench.iter(|| {
            let (mut terms, sum) = user.send(&Sum {}).expect("the channel sends");

            tasks.spawn(async move {
                for term in 1..=1000 {
                    terms.send(&term).await.expect("the channel sends");
                }

                terms.finish().await.expect("the stream ends");
            });
            tasks.run(&queue);
            assert_eq!(ready_output(sum), Ok(500_500));
        });
    });
}

fn validated_responses(criterion: &mut Criterion) {
    let (user, calculator, queue) = Memory::connected_pair::<User>();

    queue.seal();
    calculator.handle(|divide: Divide, response, _next| {
        response.respond(Quotient::Exact {
            value: divide.dividend / divide.divisor,
        });
    });
    criterion.bench_function("1000 validated messages and their responses", |bench| {
        bench.iter(|| {
            let pending: Vec<_> = (0..MESSAGES)
                .map(|dividend| {
                    user.send(&Divide {
                        dividend,
                        divisor: 1,
                        precision: Some(4),
                    })
                    .expect("the channel sends")
                })
                .collect();

            queue.deliver();
            for response in pending {
                black_box(ready_output(response).expect("the provider responds"));
            }
        });
    });
}

criterion_group!(
    channel,
    responses,
    validated_responses,
    keyed_responses,
    notifications,
    nested_notifications,
    streamed_response,
    message_stream
);

criterion_main!(channel);
