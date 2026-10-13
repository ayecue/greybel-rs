use std::cell::RefCell;
use std::rc::Rc;

use duplex_core::{Channel, ChannelOptions, ErrorCode};
use duplex_driver_stdio::{PortDriver, StreamDriver};
use duplex_test_support::conformance::calculator::CalculatorRouter;
use duplex_test_support::conformance::login::ServiceRouter;
use duplex_test_support::conformance::{Adder, Directory, calculator, credentials, login, user};
use duplex_test_support::local::run_local;

use crate::support::stream::{SETTLE, STREAM_BYTES, pair, transports};

#[test]
fn should_reply_to_a_request_over_the_stream() {
    run_local(async {
        let (user, _calculator) = pair();
        let sum = user
            .send(&user::Add { a: 2, b: 3 })
            .expect("the request is sent")
            .await;

        assert_eq!(sum, Ok(5));
    });
}

#[test]
fn should_carry_batches_larger_than_the_stream_buffer() {
    run_local(async {
        let (user, _calculator) = pair();
        let sums: Vec<_> = (0..200)
            .map(|a| {
                user.send(&user::Add { a, b: a })
                    .expect("the request is sent")
            })
            .collect();
        let mut replies = Vec::new();

        for sum in sums {
            replies.push(sum.await);
        }

        assert_eq!(replies, (0..200).map(|a| Ok(a * 2)).collect::<Vec<_>>());
    });
}

#[test]
fn should_close_when_the_stream_ends() {
    run_local(async {
        let (stream, other_end) = tokio::io::duplex(STREAM_BYTES);
        let (reader, writer) = tokio::io::split(stream);
        let transport = StreamDriver::transport(reader, writer);
        let user =
            Channel::<user::User, _>::new(&transport, ChannelOptions::without_health_checks());

        drop(other_end);
        tokio::time::sleep(SETTLE).await;
        assert_eq!(user.close_reason(), Some(ErrorCode::ConnectionLoss));
    });
}

#[test]
fn should_carry_the_channels_of_two_protocols_over_one_stream() {
    run_local(async {
        let (near, far) = transports();
        let user = Channel::<user::User, _>::new(&near, ChannelOptions::without_health_checks());
        let member = Channel::<login::Member, _>::open(
            &near,
            ChannelOptions::without_health_checks(),
            &credentials(),
        )
        .expect("the login channel writes its opening credentials");
        let calculator = Channel::<calculator::Calculator, _>::new(
            &far,
            ChannelOptions::without_health_checks(),
        );

        Adder.attach(&calculator);

        let service =
            Channel::<login::Service, _>::new(&far, ChannelOptions::without_health_checks());

        Directory.attach(&service);

        let (sum, name) = tokio::join!(
            user.send(&user::Add { a: 2, b: 3 })
                .expect("the request is sent"),
            member.send(&login::Whoami {}).expect("the request is sent")
        );

        assert_eq!(sum, Ok(5));
        assert_eq!(name, Ok("ada".to_owned()));
        assert_eq!(
            match member.state() {
                duplex_core::ConnectionState::Active(active) => {
                    active.ack().map(|ack| ack.greeting.clone())
                }
                _ => None,
            },
            Some("hello, ada".to_owned())
        );
    });
}

#[test]
fn should_serve_from_a_channel_spawned_onto_another_thread() {
    run_local(async {
        let (near, far) = transports();
        let user = Channel::<user::User, _>::new(&near, ChannelOptions::without_health_checks());
        let mut spawned = None;
        let relay = far.spawn::<calculator::Calculator>(|port| {
            spawned = Some(std::thread::spawn(move || {
                run_local(async {
                    let transport = PortDriver::transport(port);
                    let calculator = Channel::<calculator::Calculator, _>::new(
                        &transport,
                        ChannelOptions::without_health_checks(),
                    );

                    Adder.attach(&calculator);

                    calculator.close_event().await
                })
            }));
        });
        let relaying = tokio::task::spawn_local(relay);
        let sum = user
            .send(&user::Add { a: 2, b: 3 })
            .expect("the request is sent")
            .await;

        drop(far);

        let closed = tokio::task::spawn_blocking(move || {
            spawned
                .expect("the provider thread was spawned")
                .join()
                .expect("the provider thread does not panic")
        })
        .await
        .expect("the provider thread join task completes");

        relaying
            .await
            .expect("the relay task finishes without panicking");
        assert_eq!(sum, Ok(5));
        assert_eq!(closed, ErrorCode::ConnectionLoss);
    });
}

#[test]
fn should_close_a_spawned_channel_for_both_peers_with_the_transports_reason() {
    run_local(async {
        let (near, far) = transports();
        let user = Channel::<user::User, _>::new(&near, ChannelOptions::without_health_checks());
        let mut spawned = None;
        let relay = far.spawn::<calculator::Calculator>(|port| {
            spawned = Some(std::thread::spawn(move || {
                run_local(async {
                    let transport = PortDriver::transport(port);
                    let calculator = Channel::<calculator::Calculator, _>::new(
                        &transport,
                        ChannelOptions::without_health_checks(),
                    );
                    let ends = Rc::new(RefCell::new(Vec::new()));
                    let recorded = Rc::clone(&ends);

                    calculator.on_close(move |end| recorded.borrow_mut().push(end.reason()));
                    Adder.attach(&calculator);
                    calculator.close_event().await;

                    ends.take()
                })
            }));
        });
        let relaying = tokio::task::spawn_local(relay);
        let sum = user
            .send(&user::Add { a: 2, b: 3 })
            .expect("the request is sent")
            .await;

        far.close_all(ErrorCode::Denial);

        let user_closed = user.close_event().await;
        let spawned_ends = tokio::task::spawn_blocking(move || {
            spawned
                .expect("the provider thread was spawned")
                .join()
                .expect("the provider thread does not panic")
        })
        .await
        .expect("the provider thread join task completes");

        relaying
            .await
            .expect("the relay task finishes without panicking");
        assert_eq!(sum, Ok(5));
        assert_eq!(
            (user_closed, spawned_ends.first().copied()),
            (ErrorCode::Denial, Some(ErrorCode::Denial))
        );
    });
}

#[test]
fn should_serve_connections_on_the_threads_of_a_hub_manager() {
    run_local(async {
        let manager =
            std::sync::Arc::new(duplex_tokio::HubManager::start(2).expect("the threads start"));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("it listens");
        let address = listener.local_addr().expect("an address");
        let (threads, mut served) = tokio::sync::mpsc::unbounded_channel();
        let serving = {
            let manager = std::sync::Arc::clone(&manager);

            tokio::spawn(async move {
                manager
                    .serve_tcp(listener, move |stream| {
                        let threads = threads.clone();

                        async move {
                            let _ = threads.send(std::thread::current().id());
                            let transport =
                                StreamDriver::tcp(stream).expect("the connection is set up");
                            let calculator = Channel::<calculator::Calculator, _>::new(
                                &transport,
                                ChannelOptions::without_health_checks(),
                            );

                            Adder.attach(&calculator);
                            calculator.close_event().await;
                        }
                    })
                    .await;
            })
        };
        let mut users = Vec::new();

        for _ in 0..4 {
            let stream = tokio::net::TcpStream::connect(address)
                .await
                .expect("it connects");
            let transport = StreamDriver::tcp(stream).expect("the connection is set up");

            users.push(Channel::<user::User, _>::new(
                &transport,
                ChannelOptions::without_health_checks(),
            ));
        }

        for (a, user) in (0i64..).zip(&users) {
            let sum = user
                .send(&user::Add { a, b: 1 })
                .expect("the request is sent")
                .await;

            assert_eq!(sum, Ok(a + 1));
        }

        let mut seen = std::collections::HashSet::new();

        for _ in 0..4 {
            seen.insert(served.recv().await.expect("each connection is served"));
        }

        assert_eq!(seen.len(), 2);
        assert!(!seen.contains(&std::thread::current().id()));
        serving.abort();
    });
}
