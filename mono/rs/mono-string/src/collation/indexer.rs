pub(super) struct Indexer {
    ranges: &'static [(i32, i32)],
    default: i32,
}

impl Indexer {
    pub(super) fn index(&self, cp: i32) -> i32 {
        let mut start_index = 0;

        for &(start, end) in self.ranges {
            if cp < start {
                return self.default;
            }

            if cp < end {
                return cp - start + start_index;
            }

            start_index += end - start;
        }

        self.default
    }
}

pub(super) const IGNORABLE: Indexer = Indexer {
    ranges: &[(0, 13312), (40960, 42240), (63744, 65536)],
    default: -1,
};
pub(super) const CATEGORY: Indexer = Indexer {
    ranges: &[
        (0, 4608),
        (7680, 10240),
        (12288, 13312),
        (19968, 40960),
        (44032, 55216),
        (63744, 65536),
    ],
    default: 0,
};
pub(super) const LEVEL1: Indexer = CATEGORY;
pub(super) const LEVEL2: Indexer = Indexer {
    ranges: &[(0, 3840), (7680, 10240), (12288, 13312), (64256, 65536)],
    default: 0,
};
pub(super) const LEVEL3: Indexer = Indexer {
    ranges: &[(0, 4608), (7680, 10240), (12288, 13312), (64256, 65536)],
    default: 0,
};
