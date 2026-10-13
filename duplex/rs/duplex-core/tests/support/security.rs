use duplex_core::{Channel, ChannelOptions, EntropyUnavailable, Security, SecurityRole, Transport};
use duplex_test_support::conformance::Adder;
use duplex_test_support::conformance::calculator::{Calculator, CalculatorRouter, User};
use duplex_test_support::{Memory, MemoryQueue};

pub const CLIENT_IDENTITY: [u8; 32] = [1; 32];

pub const PROVIDER_IDENTITY: [u8; 32] = [2; 32];

pub const SECURE_FIXTURES: [&str; 5] = [
    "secureFirst",
    "secureSecond",
    "secureThird",
    "secureClientBatch",
    "secureProviderBatch",
];

pub fn identified(role: SecurityRole, identity: [u8; 32], trusted: [u8; 32]) -> Security {
    Security::new(role)
        .with_identity(identity)
        .with_pinned_keys(vec![Security::public_key(trusted)])
}

#[allow(clippy::unnecessary_wraps)]
pub fn zeros(bytes: &mut [u8]) -> Result<(), EntropyUnavailable> {
    bytes.fill(0);

    Ok(())
}

pub fn authenticated() -> (Transport<Memory>, Transport<Memory>, MemoryQueue) {
    Memory::transports_with_security(
        identified(SecurityRole::Initiator, CLIENT_IDENTITY, PROVIDER_IDENTITY),
        identified(SecurityRole::Responder, PROVIDER_IDENTITY, CLIENT_IDENTITY),
    )
}

pub fn calculator(
    (near, far, queue): (Transport<Memory>, Transport<Memory>, MemoryQueue),
) -> (
    Channel<User, Memory>,
    Channel<Calculator, Memory>,
    MemoryQueue,
) {
    let user = Channel::<User, _>::new(&near, ChannelOptions::default());
    let calculator = Channel::<Calculator, _>::new(&far, ChannelOptions::default());

    Adder.attach(&calculator);

    (user, calculator, queue)
}
