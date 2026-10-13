use duplex_core::{ErrorCode, Role};
use duplex_test_support::Memory;
use duplex_test_support::conformance::legacy::Caller;
use duplex_test_support::frames::{Frame, Version};

use crate::support::versions::open;

#[test]
fn should_refuse_a_client_of_an_unsupported_version() {
    let (_caller, _server, queue) = Memory::connected_pair::<Caller>();

    queue.post(Role::Provider, open(Version { major: 1, minor: 0 }));
    queue.step();

    let refusals: Vec<Frame> = queue
        .queued_slices()
        .into_iter()
        .filter(|batch| batch.from == Role::Provider as u8)
        .flat_map(|batch| batch.frames)
        .filter(|frame| matches!(frame, Frame::Refuse { .. }))
        .collect();

    assert_eq!(
        refusals,
        [Frame::Refuse {
            id: 9,
            reason: u32::from(ErrorCode::UnsupportedVersion),
            core: Version { major: 1, minor: 0 },
            protocol: Version { major: 1, minor: 1 },
            majors: vec![1],
            payload: Vec::new(),
        }]
    );
}

#[test]
fn should_connect_a_client_of_a_supported_version() {
    let (caller, server, _queue) = Memory::connected_pair::<Caller>();

    assert!(caller.connection().is_some());
    assert_eq!(server.connection(), caller.connection());
}
