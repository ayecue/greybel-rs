use std::sync::LazyLock;

use super::contraction::{Contraction, contractions};
use super::indexer::{CATEGORY, IGNORABLE, LEVEL1, LEVEL2, LEVEL3};

const CORE: &[u8] = include_bytes!("core.bin");
const TAILORING: &[u8] = include_bytes!("tailoring.bin");
const UNASSIGNED_BITMAP: &[u8; 0x10000 / 8] = include_bytes!("unassigned.bin");

const RESOURCE_VERSION: u8 = 3;
const INVARIANT: u32 = 127;
const UNSAFE_FLAG_LENGTH: usize = 96;

pub(super) struct CollationData {
    ignorable_flags: &'static [u8],
    categories: &'static [u8],
    level1: &'static [u8],
    level2: &'static [u8],
    level3: &'static [u8],
    pub(super) contractions: Vec<Contraction>,
    pub(super) french_sort: bool,
    unsafe_flags: [u8; UNSAFE_FLAG_LENGTH],
}

pub(super) static TABLES: LazyLock<CollationData> = LazyLock::new(CollationData::load);

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().expect("four bytes"))
}

fn usize_at(bytes: &[u8], at: usize) -> usize {
    usize::try_from(u32_at(bytes, at)).expect("a table offset")
}

impl CollationData {
    fn load() -> Self {
        assert_eq!(CORE[0], RESOURCE_VERSION, "collation.core.bin version");
        assert_eq!(
            TAILORING[0], RESOURCE_VERSION,
            "collation.tailoring.bin version"
        );

        let mut at = 1;
        let mut section = || {
            let len = usize_at(CORE, at);

            at += 4;

            let bytes = &CORE[at..at + len];

            at += len;

            bytes
        };
        let ignorable_flags = section();
        let categories = section();
        let level1 = section();
        let level2 = section();
        let level3 = section();

        let mut at = 1;
        let count = usize_at(TAILORING, at);

        at += 4;

        let mut invariant = None;

        for _ in 0..count {
            let lcid = u32_at(TAILORING, at);
            let index = usize_at(TAILORING, at + 4);
            let len = usize_at(TAILORING, at + 8);
            let french_sort = TAILORING[at + 12] != 0;

            at += 13;
            if lcid == INVARIANT {
                invariant = Some((index, len, french_sort));
            }
        }

        at += 2;

        let units = usize_at(TAILORING, at);

        at += 4;

        let tailoring: Vec<u16> = TAILORING[at..at + units * 2]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u16::from_le_bytes(*pair))
            .collect();
        let (index, len, french_sort) = invariant.expect("the invariant culture's tailoring");
        let contractions = contractions(&tailoring[index..], len);

        let mut unsafe_flags = [0; UNSAFE_FLAG_LENGTH];

        for contraction in contractions.iter().filter(|c| c.source.len() > 1) {
            for &unit in &contraction.source {
                unsafe_flags[usize::from(unit / 8)] |= 1 << (unit & 7);
            }
        }

        Self {
            ignorable_flags,
            categories,
            level1,
            level2,
            level3,
            contractions,
            french_sort,
            unsafe_flags,
        }
    }

    pub(super) fn category(&self, cp: i32) -> u8 {
        self.categories[table_index(CATEGORY.index(cp))]
    }

    pub(super) fn level1(&self, cp: i32) -> u8 {
        self.level1[table_index(LEVEL1.index(cp))]
    }

    pub(super) fn level2(&self, cp: i32) -> u8 {
        self.level2[table_index(LEVEL2.index(cp))]
    }

    pub(super) fn level3(&self, cp: i32) -> u8 {
        self.level3[table_index(LEVEL3.index(cp))]
    }

    fn is_ignorable_with(&self, cp: i32, flag: u8) -> bool {
        if cp == 0 {
            return true;
        }

        if flag & 1 != 0 {
            if is_unassigned(cp) {
                return true;
            }

            if (55424..56192).contains(&cp) {
                return true;
            }
        }

        let index = IGNORABLE.index(cp);

        index >= 0 && self.ignorable_flags[table_index(index)] & flag != 0
    }

    pub(super) fn is_ignorable(&self, cp: i32) -> bool {
        self.is_ignorable_with(cp, 1)
    }

    pub(super) fn is_ignorable_non_spacing(&self, cp: i32) -> bool {
        self.is_ignorable_with(cp, 4)
    }

    pub(super) fn is_safe(&self, cp: i32) -> bool {
        let Ok(cp) = usize::try_from(cp) else {
            return true;
        };

        cp / 8 >= UNSAFE_FLAG_LENGTH || self.unsafe_flags[cp / 8] & (1 << (cp % 8)) == 0
    }
}

fn table_index(index: i32) -> usize {
    usize::try_from(index).expect("an index the table covers")
}

fn is_unassigned(cp: i32) -> bool {
    let unit = usize::from(cp as u16);

    UNASSIGNED_BITMAP[unit / 8] & (1 << (unit % 8)) != 0
}
