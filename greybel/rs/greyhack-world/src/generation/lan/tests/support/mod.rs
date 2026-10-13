use greyhack_behaviour::NetworkKind;

use super::super::{LanPlan, plan};
use crate::Rng;

pub(super) fn plans() -> impl Iterator<Item = LanPlan> {
    (0..200).map(|seed| {
        plan(
            &mut Rng::new(seed),
            NetworkKind::Supermarket,
            "192.168.1.1",
            None,
        )
    })
}
