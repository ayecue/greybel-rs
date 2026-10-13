pub(super) struct Context {
    pub(super) buffer1: [u8; 4],
    pub(super) buffer2: [u8; 4],
    pub(super) prev_code: i32,
    pub(super) prev_sort_key: bool,
    pub(super) always_match: Option<[u8; 16]>,
    pub(super) never_match: Option<[u8; 16]>,
}

impl Context {
    pub(super) fn new(match_flags: bool) -> Self {
        Self {
            buffer1: [0; 4],
            buffer2: [0; 4],
            prev_code: -1,
            prev_sort_key: false,
            always_match: match_flags.then_some([0; 16]),
            never_match: match_flags.then_some([0; 16]),
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct Escape<'s> {
    pub(super) source: Option<&'s [u16]>,
    pub(super) index: isize,
    pub(super) start: isize,
    pub(super) end: isize,
    pub(super) optional: isize,
}

impl Escape<'_> {
    pub(super) const NONE: Self = Self {
        source: None,
        index: 0,
        start: 0,
        end: 0,
        optional: 0,
    };
}

pub(super) struct Comparison {
    pub(super) difference: i32,
    pub(super) target_consumed: bool,
}

impl Comparison {
    pub(super) fn early(difference: i32) -> Self {
        Self {
            difference,
            target_consumed: false,
        }
    }
}
