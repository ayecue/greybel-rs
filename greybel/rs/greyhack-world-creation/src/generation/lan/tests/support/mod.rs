use greyhack_behaviour::{NetworkKind, WorldSeed};

use super::super::{LanPlan, plan};
use crate::Rng;

pub(super) fn plans() -> impl Iterator<Item = LanPlan> {
    (0..200).map(|seed| {
        plan(
            &mut Rng::new(seed),
            &WorldSeed::new(vec![seed as u8; 32]),
            "8.8.8.8",
            NetworkKind::Supermarket,
            "192.168.1.1",
            None,
        )
    })
}
