use duplex_core::Protocol;
use duplex_test_support::conformance::legacy::Legacy;
use duplex_test_support::frames::{self, Frame, Slice, Version};

pub fn open(protocol: Version) -> Vec<u8> {
    Slice {
        core: frames::CORE,
        channel: <Legacy as Protocol>::CHANNEL_ID,
        from: 0,
        connection: 0,
        frames: vec![Frame::Open {
            id: 9,
            core: Version { major: 1, minor: 0 },
            protocol,
            fingerprint: <Legacy as Protocol>::VERSIONS[usize::from(protocol.minor)].fingerprint,
            payload: Vec::new(),
        }],
    }
    .encode()
}
