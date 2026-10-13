use greyhack_behaviour::FileKind;
use mono_random::Random;

use super::super::{ContentFile, RandomContent};

pub(super) const SEEDS: i32 = 3_000;

pub(super) fn users() -> Vec<String> {
    ["root", "anna", "ben"].map(str::to_owned).to_vec()
}

pub(super) fn draw(seed: i32, neurobox: bool) -> RandomContent {
    RandomContent::draw(
        &mut Random::new(seed),
        &users(),
        neurobox,
        &|_, _| false,
        &|_| true,
    )
}

pub(super) fn all(neurobox: bool) -> Vec<RandomContent> {
    (0..SEEDS).map(|seed| draw(seed, neurobox)).collect()
}

pub(super) fn is_chat(file: &ContentFile) -> bool {
    file.data.kind == FileKind::ChatLog
}
