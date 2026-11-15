use greyhack_behaviour::{NetworkKind, Rng, WorldSeed};

use super::super::{LanPlan, plan};

pub(super) fn plans() -> impl Iterator<Item = LanPlan> {
    (0..200).map(|seed| {
        plan(
            &mut Rng::new(seed),
            &WorldSeed::new(vec![
                u8::try_from(seed)
                    .expect("value fits the target integer type");
                32
            ]),
            "8.8.8.8",
            NetworkKind::Supermarket,
            "192.168.1.1",
            None,
        )
    })
}
