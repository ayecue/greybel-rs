use greyhack_behaviour::collect_acks;

use crate::World;

/// Capture progress displayed by `aireplay` (`Terminal.CollectData`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AckCount {
    /// Packet count shown on the terminal's last line.
    pub packets: i32,
    /// ACKs received.
    pub acks: i32,
}

impl World {
    /// Reads packets from signal `power` until ACK count exceeds `max_acks` (`Terminal.CollectData`).
    pub fn collect_acks(&mut self, power: i32, max_acks: i32) -> AckCount {
        let (packets, acks) = collect_acks(&mut self.rng, power, max_acks);

        AckCount { packets, acks }
    }
}
