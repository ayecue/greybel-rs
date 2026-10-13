use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use duplex_core::{ErrorCode, HealthCheck, HealthChecks, PendingConnection, TransportFlavour};
use duplex_test_support::conformance::Directory;
use duplex_test_support::conformance::login::{Login, ServiceRouter, Session, Whoami};
use duplex_test_support::future::ready_output;

use crate::support::handshake::signing_in;

#[test]
fn should_refuse_a_member_whose_open_is_dropped() {
    let (member, service, queue) = signing_in();

    service.on_connect(|_, pending| drop(pending));
    queue.deliver();
    assert_eq!(member.close_reason(), Some(ErrorCode::DroppedMessage));
}

#[test]
fn should_wait_for_an_open_settled_later() {
    let (member, service, queue) = signing_in();
    let waiting: Rc<RefCell<Option<PendingConnection<Login>>>> = Rc::default();
    let kept = Rc::clone(&waiting);

    service.on_connect(move |_, pending| {
        kept.replace(Some(pending));
    });
    queue.deliver();

    let before = member.connection();

    if let Some(pending) = waiting.take() {
        pending.accept(Session {
            greeting: "later".to_owned(),
        });
    }

    queue.deliver();
    assert_eq!(before, None);
    assert!(member.connection().is_some());
    assert_eq!(service.connection(), member.connection());
}

#[test]
fn should_accept_by_the_router_of_the_service() {
    let (member, service, queue) = signing_in();

    Directory.attach(&service);
    queue.deliver();

    let whoami = member.send(&Whoami {}).expect("the whoami request is sent");

    queue.deliver();
    assert_eq!(ready_output(whoami), Ok("ada".to_owned()));
}

#[test]
fn should_generate_the_flavours_of_a_protocol() {
    let flavours = <Login as duplex_core::Protocol>::FLAVOURS;

    assert_eq!(
        flavours,
        [
            TransportFlavour {
                transport_name: "stdio",
                health_check: Some(HealthChecks::On(HealthCheck {
                    interval: Duration::from_secs(1),
                    close_threshold: 7,
                    ..HealthCheck::DEFAULT
                })),
                stream_window: None,
                timeout: None,
                max_message: None,
            },
            TransportFlavour {
                transport_name: "port",
                health_check: Some(HealthChecks::Off),
                stream_window: None,
                timeout: None,
                max_message: None,
            },
        ]
    );
}
