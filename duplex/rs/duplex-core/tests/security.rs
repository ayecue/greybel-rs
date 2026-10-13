use duplex_core::{
    Channel, ChannelOptions, EntropyUnavailable, ErrorCode, Role, Security, SecurityRole,
};
use duplex_test_support::Memory;
use duplex_test_support::binary::hex;
use duplex_test_support::conformance::calculator::{Calculator, User};
use duplex_test_support::fixtures::Fixtures;

use crate::support::security::{
    CLIENT_IDENTITY, PROVIDER_IDENTITY, SECURE_FIXTURES, authenticated, calculator, identified,
    zeros,
};
use crate::support::{log, logs};

#[test]
fn should_send_nothing_a_channel_writes_in_the_clear() {
    let (user, calculator, queue) = calculator(authenticated());
    let logged = logs(&calculator);

    queue.deliver();
    user.send(&log("a secret"))
        .expect("the secret log notification is sent");

    let posted = queue.take();

    assert!(
        posted
            .iter()
            .all(|batch| !batch.windows(8).any(|window| window == b"a secret"))
    );
    for batch in posted {
        queue.post(Role::Provider, batch);
    }

    queue.deliver();
    assert_eq!(*logged.borrow(), ["a secret"]);
}

#[test]
fn should_secure_a_transport_as_the_shared_fixtures_say() {
    let (near, far, queue) = Memory::transports_with_security(
        identified(SecurityRole::Initiator, CLIENT_IDENTITY, PROVIDER_IDENTITY)
            .with_batch_signatures()
            .with_entropy(zeros),
        identified(SecurityRole::Responder, PROVIDER_IDENTITY, CLIENT_IDENTITY)
            .with_batch_signatures()
            .with_entropy(zeros),
    );
    let _user = Channel::<User, _>::new(&near, ChannelOptions::without_health_checks());
    let _calculator = Channel::<Calculator, _>::new(&far, ChannelOptions::without_health_checks());
    let delivered = queue.deliver_bytes();

    assert!(delivered.len() >= SECURE_FIXTURES.len());
    for (name, batch) in SECURE_FIXTURES.iter().zip(&delivered) {
        Fixtures::check(name, &hex(batch));
    }
}

#[test]
fn should_close_what_attaches_to_a_transport_it_has_no_secret_randomness_for() {
    let (near, _far, queue) = Memory::transports_with_security(
        Security::new(SecurityRole::Initiator).with_entropy(|_| Err(EntropyUnavailable)),
        Security::new(SecurityRole::Responder),
    );
    let user = Channel::<User, _>::new(&near, ChannelOptions::default());

    queue.run_tasks();
    assert_eq!(user.close_reason(), Some(ErrorCode::Insecure));
    assert!(queue.take().is_empty(), "sends nothing");
}
