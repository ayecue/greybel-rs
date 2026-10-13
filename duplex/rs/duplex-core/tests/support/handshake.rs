use duplex_core::{Channel, ChannelOptions};
use duplex_test_support::conformance::credentials;
use duplex_test_support::conformance::login::{Member, Service};
use duplex_test_support::{Memory, MemoryQueue};

/// Creates a member/login-service channel pair using `ada`'s credentials without delivering the handshake.
pub fn signing_in() -> (
    Channel<Member, Memory>,
    Channel<Service, Memory>,
    MemoryQueue,
) {
    Memory::open_pair::<Member>(ChannelOptions::default(), &credentials())
}
