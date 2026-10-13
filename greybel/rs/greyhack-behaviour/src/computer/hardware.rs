//! Hardware types, network-interface naming, and mining checks.

use crate::Rng;

/// Health below which a GPU is damaged and cannot mine (`SubWalletIntrinsics.mining`).
const GPU_DAMAGED_HEALTH: f32 = 5.0;
/// Health below which a GPU can fail the floating-point self-test (`SubWalletIntrinsics.mining`).
const GPU_UNRELIABLE_HEALTH: f32 = 40.0;

/// Why a GPU cannot start mining.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuFault {
    /// The GPU health is below 5.
    LowHealth,
    /// The GPU does not pass the self-test, which it can fail below health 40.
    SelfTest,
}

/// Checks a GPU of `health` before it mines (`SubWalletIntrinsics.mining`); draws only when the GPU can fail the
/// self-test.
pub fn check_gpu(rng: &mut Rng, health: f32) -> Option<GpuFault> {
    if health < GPU_DAMAGED_HEALTH {
        return Some(GpuFault::LowHealth);
    }

    if health < GPU_UNRELIABLE_HEALTH && (rng.next_u64() % 101) as f32 > health {
        return Some(GpuFault::SelfTest);
    }

    None
}

/// Names network interfaces in input order, using `eth0`, `eth1`, … for wired cards and `wlan0`, …
/// for Wi-Fi cards.
#[must_use]
pub fn interface_names(wifi: impl IntoIterator<Item = bool>) -> Vec<String> {
    let (mut ethernet, mut wireless) = (0, 0);

    wifi.into_iter()
        .map(|wifi| {
            let (prefix, count) = if wifi {
                ("wlan", &mut wireless)
            } else {
                ("eth", &mut ethernet)
            };

            *count += 1;

            format!("{prefix}{}", *count - 1)
        })
        .collect()
}

enum_def! {
    /// Hardware part type (`Hardware.TypeHardware`).
    pub enum HardwareKind {
        HardDisk = 0,
        Cpu = 1,
        PowerSupply = 2,
        Motherboard = 3,
        Ram = 4,
        NetCard = 5,
        Gpu = 6,
    }
}
