use std::collections::{BTreeMap, BTreeSet};

use ttf_parser::GlyphId;
use ttf_parser::kern::Format;

use super::FontFile;

impl FontFile<'_> {
    /// Sums `kern` adjustments for glyph pairs in font units using TMP Font Asset Creator behavior; ignores GPOS.
    #[must_use]
    pub fn kerning(&self, glyphs: &BTreeSet<u16>) -> BTreeMap<(u16, u16), i64> {
        let mut pairs = BTreeMap::new();
        let Some(kern) = self.face.tables().kern else {
            return pairs;
        };

        for subtable in kern.subtables {
            if !subtable.horizontal || subtable.variable || subtable.has_cross_stream {
                continue;
            }

            let mut add = |first: u16, second: u16, value: i16| {
                if value != 0 {
                    *pairs.entry((first, second)).or_insert(0) += i64::from(value);
                }
            };

            match &subtable.format {
                Format::Format0(table) => {
                    for pair in table.pairs {
                        let (first, second) = (pair.left().0, pair.right().0);

                        if glyphs.contains(&first) && glyphs.contains(&second) {
                            add(first, second, pair.value);
                        }
                    }
                }
                _ => {
                    for &first in glyphs {
                        for &second in glyphs {
                            if let Some(value) =
                                subtable.glyphs_kerning(GlyphId(first), GlyphId(second))
                            {
                                add(first, second, value);
                            }
                        }
                    }
                }
            }
        }

        pairs.retain(|_, value| *value != 0);

        pairs
    }
}
