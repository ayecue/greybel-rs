use greyhack_behaviour::collect_acks;

use crate::Crypto;

/// Capture progress displayed by `aireplay` (`Terminal.CollectData`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AckCount {
    /// Packet count shown on the terminal's last line.
    pub packets: i32,
    /// ACKs received.
    pub acks: i32,
}

/// Methods of [`World`] for intrusion capture.
impl Crypto {
    pub fn collect_acks(&self, power: i32, max_acks: i32) -> AckCount {
        let (packets, acks) = collect_acks(&mut self.world.world_mut().rng, power, max_acks);

        AckCount { packets, acks }
    }
}
