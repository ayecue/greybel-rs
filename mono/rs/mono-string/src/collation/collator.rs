use super::context::{Comparison, Context, Escape};
use super::contraction::Contraction;
use super::extender::{Extender, dash_type, extender_type};
use super::flags::{compare_flag_pair, has_flag, set_flag};
use super::kana::{has_special_weight, is_half_width_kana, is_hiragana, is_japanese_small_letter};
use super::table::CollationData;

impl CollationData {
    pub(super) fn level2_of(&self, cp: i32, extender: Extender) -> u8 {
        match extender {
            Extender::Buggy => 5,
            Extender::Conditional => 0,
            _ => self.level2(cp),
        }
    }

    pub(super) fn filter_extender(&self, cp: i32, extender: Extender) -> i32 {
        if extender != Extender::Conditional || !has_special_weight(cp) {
            return cp;
        }

        let half = is_half_width_kana(cp);
        let katakana = !is_hiragana(cp);
        let pick = |hiragana: i32, katakana_full: i32, half_width: i32| {
            if half {
                half_width
            } else if katakana {
                katakana_full
            } else {
                hiragana
            }
        };

        match self.level1(cp) & 7 {
            2 => pick(12354, 12450, 65393),
            3 => pick(12356, 12452, 65394),
            4 => pick(12358, 12454, 65395),
            5 => pick(12360, 12456, 65396),
            6 => pick(12362, 12458, 65397),
            _ => cp,
        }
    }

    pub(super) fn contraction(&self, s: &[u16], start: isize, end: isize) -> Option<&Contraction> {
        let first = at(s, start);

        for contraction in &self.contractions {
            let difference = i32::from(contraction.source[0]) - first;

            if difference > 0 {
                return None;
            }

            if difference < 0 || end - start < len(&contraction.source) {
                continue;
            }

            let from = usize::try_from(start).expect("a position");

            if s[from..from + contraction.source.len()] == contraction.source[..] {
                return Some(contraction);
            }
        }

        None
    }

    pub(super) fn tail_contraction(
        &self,
        s: &[u16],
        start: isize,
        end: isize,
    ) -> Option<&Contraction> {
        debug_assert!(start != end && end >= -1 && start < len(s) && len(s) > end + 1);

        let last = s[usize::try_from(start).expect("a position")];

        self.contractions.iter().find(|contraction| {
            let source = &contraction.source;

            if len(source) > start - end || source[source.len() - 1] != last {
                return false;
            }

            let from = usize::try_from(start - len(source) + 1).expect("a position");

            s[from..from + source.len()] == source[..]
        })
    }

    pub(super) fn compare(
        &self,
        s1: &[u16],
        idx1: isize,
        len1: isize,
        s2: &[u16],
        idx2: isize,
        len2: isize,
    ) -> i32 {
        let mut ctx = Context::new(false);

        self.compare_internal(s1, idx1, len1, s2, idx2, len2, true, false, &mut ctx)
            .difference
            .signum()
    }

    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    pub(super) fn compare_internal<'s>(
        &'s self,
        mut s1: &'s [u16],
        mut idx1: isize,
        len1: isize,
        mut s2: &'s [u16],
        mut idx2: isize,
        len2: isize,
        skip_heading_extenders: bool,
        immediate_breakup: bool,
        ctx: &mut Context,
    ) -> Comparison {
        let mut start1 = idx1;
        let mut start2 = idx2;
        let mut end1 = idx1 + len1;
        let mut end2 = idx2 + len2;
        let mut previous_code = -1;
        let mut previous_sort_key = false;
        let mut difference = 0_i32;
        let mut level = 5;
        let mut special1: isize = -1;
        let mut special2: isize = -1;
        let mut special_weight1 = 0_i32;
        let mut special_weight2 = 0_i32;

        if skip_heading_extenders {
            while idx1 < end1 && extender_type(at(s1, idx1)) != Extender::None {
                idx1 += 1;
            }

            while idx2 < end2 && extender_type(at(s2, idx2)) != Extender::None {
                idx2 += 1;
            }
        }

        let mut extender1 = Extender::None;
        let mut extender2 = Extender::None;
        let mut checked1 = idx1;
        let mut checked2 = idx2;
        let mut escape1 = Escape::NONE;
        let mut escape2 = Escape::NONE;

        loop {
            if idx1 < end1 && self.is_ignorable(at(s1, idx1)) {
                idx1 += 1;
                continue;
            }

            while idx2 < end2 && self.is_ignorable(at(s2, idx2)) {
                idx2 += 1;
            }

            if idx1 >= end1 {
                let Some(source) = escape1.source.take() else {
                    break;
                };

                s1 = source;
                start1 = escape1.start;
                idx1 = escape1.index;
                end1 = escape1.end;
                checked1 = escape1.optional;
                continue;
            }

            if idx2 >= end2 {
                let Some(source) = escape2.source.take() else {
                    break;
                };

                s2 = source;
                start2 = escape2.start;
                idx2 = escape2.index;
                end2 = escape2.end;
                checked2 = escape2.optional;
                continue;
            }

            if checked1 < idx1 && checked2 < idx2 {
                while idx1 < end1 && idx2 < end2 && at(s1, idx1) == at(s2, idx2) {
                    idx1 += 1;
                    idx2 += 1;
                }

                if idx1 == end1 || idx2 == end2 {
                    continue;
                }

                let back1 = checked1;
                let back2 = checked2;

                checked1 = idx1;
                checked2 = idx2;
                idx1 -= 1;
                idx2 -= 1;
                while idx1 > back1 && self.category(at(s1, idx1)) == 1 {
                    idx1 -= 1;
                }

                while idx2 > back2 && self.category(at(s2, idx2)) == 1 {
                    idx2 -= 1;
                }

                while idx1 > back1 && !self.is_safe(at(s1, idx1)) {
                    idx1 -= 1;
                }

                while idx2 > back2 && !self.is_safe(at(s2, idx2)) {
                    idx2 -= 1;
                }
            }

            let here1 = idx1;
            let here2 = idx2;
            let mut keyed1 = false;
            let mut keyed2 = false;
            let mut cp1 = at(s1, idx1);
            let mut cp2 = at(s2, idx2);
            let mut special_kana1 = false;
            let mut special_kana2 = false;

            extender1 = extender_type(cp1);
            if extender1 != Extender::None {
                if ctx.prev_code < 0 {
                    if !ctx.prev_sort_key {
                        idx1 += 1;
                        continue;
                    }

                    keyed1 = true;
                } else {
                    cp1 = self.filter_extender(ctx.prev_code, extender1);
                }
            }

            extender2 = extender_type(cp2);
            if extender2 != Extender::None {
                if previous_code < 0 {
                    if !previous_sort_key {
                        idx2 += 1;
                        continue;
                    }

                    keyed2 = true;
                } else {
                    cp2 = self.filter_extender(previous_code, extender2);
                }
            }

            let category1 = self.category(cp1);
            let category2 = self.category(cp2);
            let special_weight =
                |cp| i32::from(self.level1(cp)).wrapping_shl(8 + u32::from(self.level3(cp)));

            if category1 == 6 {
                if level == 5 {
                    special1 = match escape1.source {
                        Some(_) => escape1.index - escape1.start,
                        None => here1 - start1,
                    };
                    special_weight1 = special_weight(cp1);
                }

                ctx.prev_code = cp1;
                idx1 += 1;
            }

            if category2 == 6 {
                if level == 5 {
                    special2 = match escape2.source {
                        Some(_) => escape2.index - escape2.start,
                        None => here2 - start2,
                    };
                    special_weight2 = special_weight(cp2);
                }

                previous_code = cp2;
                idx2 += 1;
            }

            if category1 == 6 || category2 == 6 {
                if level == 5 {
                    if special_weight1 == special_weight2 {
                        special1 = -1;
                        special2 = -1;
                        special_weight1 = 0;
                        special_weight2 = 0;
                    } else {
                        level = 4;
                    }
                }

                continue;
            }

            let contraction1 = if extender1 == Extender::None {
                self.contraction(s1, idx1, end1)
            } else {
                None
            };
            let mut advance1 = 1;

            if keyed1 {
            } else if let Some(contraction) = contraction1 {
                advance1 = len(&contraction.source);

                if let Some(sort_key) = contraction.sort_key {
                    ctx.buffer1 = sort_key;
                    ctx.prev_code = -1;
                    ctx.prev_sort_key = true;
                } else if escape1.source.is_none() {
                    escape1 = Escape {
                        source: Some(s1),
                        start: start1,
                        index: here1 + len(&contraction.source),
                        end: end1,
                        optional: checked1,
                    };
                    s1 = contraction.replacement.as_deref().expect("a replacement");
                    idx1 = 0;
                    start1 = 0;
                    end1 = len(s1);
                    checked1 = 0;
                    continue;
                }
            } else {
                ctx.buffer1[0] = category1;
                ctx.buffer1[1] = self.level1(cp1);
                if level > 1 {
                    ctx.buffer1[2] = self.level2_of(cp1, extender1);
                }

                if level > 2 {
                    ctx.buffer1[3] = self.level3(cp1);
                }

                if level > 3 {
                    special_kana1 = has_special_weight(cp1);
                }

                if category1 > 1 {
                    ctx.prev_code = cp1;
                }
            }

            let contraction2 = if extender2 == Extender::None {
                self.contraction(s2, idx2, end2)
            } else {
                None
            };

            if keyed2 {
                idx2 += 1;
            } else if let Some(contraction) = contraction2 {
                idx2 += len(&contraction.source);

                if let Some(sort_key) = contraction.sort_key {
                    ctx.buffer2 = sort_key;
                    previous_code = -1;
                    previous_sort_key = true;
                } else if escape2.source.is_none() {
                    escape2 = Escape {
                        source: Some(s2),
                        start: start2,
                        index: here2 + len(&contraction.source),
                        end: end2,
                        optional: checked2,
                    };
                    s2 = contraction.replacement.as_deref().expect("a replacement");
                    idx2 = 0;
                    start2 = 0;
                    end2 = len(s2);
                    checked2 = 0;
                    continue;
                }
            } else {
                ctx.buffer2[0] = category2;
                ctx.buffer2[1] = self.level1(cp2);
                if level > 1 {
                    ctx.buffer2[2] = self.level2_of(cp2, extender2);
                }

                if level > 2 {
                    ctx.buffer2[3] = self.level3(cp2);
                }

                if level > 3 {
                    special_kana2 = has_special_weight(cp2);
                }

                if category2 > 1 {
                    previous_code = cp2;
                }

                idx2 += 1;
            }

            idx1 += advance1;
            while idx1 < end1 && self.category(at(s1, idx1)) == 1 {
                if ctx.buffer1[2] == 0 {
                    ctx.buffer1[2] = 2;
                }

                ctx.buffer1[2] =
                    ctx.buffer1[2].wrapping_add(self.level2_of(at(s1, idx1), Extender::None));
                idx1 += 1;
            }

            while idx2 < end2 && self.category(at(s2, idx2)) == 1 {
                if ctx.buffer2[2] == 0 {
                    ctx.buffer2[2] = 2;
                }

                ctx.buffer2[2] =
                    ctx.buffer2[2].wrapping_add(self.level2_of(at(s2, idx2), Extender::None));
                idx2 += 1;
            }

            let (key1, key2) = (ctx.buffer1, ctx.buffer2);
            let mut step = i32::from(key1[0]) - i32::from(key2[0]);

            if step == 0 {
                step = i32::from(key1[1]) - i32::from(key2[1]);
            }

            if step != 0 {
                return Comparison::early(step);
            }

            if level == 1 {
                continue;
            }

            step = i32::from(key1[2]) - i32::from(key2[2]);
            if step != 0 {
                difference = step;
                if immediate_breakup {
                    return Comparison::early(-1);
                }

                level = if self.french_sort { 2 } else { 1 };
                continue;
            }

            if level == 2 {
                continue;
            }

            step = i32::from(key1[3]) - i32::from(key2[3]);
            if step != 0 {
                difference = step;
                if immediate_breakup {
                    return Comparison::early(-1);
                }

                level = 2;
                continue;
            }

            if level == 3 {
                continue;
            }

            if special_kana1 != special_kana2 {
                if immediate_breakup {
                    return Comparison::early(-1);
                }

                difference = if special_kana1 { 1 } else { -1 };
                level = 3;
                continue;
            }

            if !special_kana1 {
                continue;
            }

            step = compare_flag_pair(
                !is_japanese_small_letter(cp1),
                !is_japanese_small_letter(cp2),
            );
            if step == 0 {
                step = dash_type(extender1) - dash_type(extender2);
            }

            if step == 0 {
                step = compare_flag_pair(is_hiragana(cp1), is_hiragana(cp2));
            }

            if step == 0 {
                step = compare_flag_pair(!is_half_width_kana(cp1), !is_half_width_kana(cp2));
            }

            if step != 0 {
                if immediate_breakup {
                    return Comparison::early(-1);
                }

                difference = step;
                level = 3;
            }
        }

        if difference != 0 && level > 2 {
            while idx1 < end1
                && idx2 < end2
                && self.is_ignorable_non_spacing(at(s1, idx1))
                && self.is_ignorable_non_spacing(at(s2, idx2))
            {
                difference = i32::from(self.level2_of(at(s1, idx1), extender1))
                    - i32::from(self.level2_of(at(s2, idx2), extender2));
                if difference != 0 {
                    break;
                }

                idx1 += 1;
                idx2 += 1;
                extender1 = Extender::None;
                extender2 = Extender::None;
            }
        }

        if level == 1 && difference != 0 {
            while idx1 < end1 && self.is_ignorable_non_spacing(at(s1, idx1)) {
                idx1 += 1;
            }

            while idx2 < end2 && self.is_ignorable_non_spacing(at(s2, idx2)) {
                idx2 += 1;
            }
        }

        if difference == 0 {
            difference = if special1 < 0 && special2 >= 0 {
                -1
            } else if special2 < 0 && special1 >= 0 {
                1
            } else {
                match i32::try_from(special1 - special2).expect("a text position") {
                    0 => special_weight1.wrapping_sub(special_weight2),
                    positions => positions,
                }
            };
        }

        let target_consumed = difference == 0 && idx2 == end2;
        let difference = if idx1 != end1 {
            1
        } else if idx2 != end2 {
            -1
        } else {
            difference
        };

        Comparison {
            difference,
            target_consumed,
        }
    }

    pub(super) fn is_prefix(
        &self,
        s: &[u16],
        target: &[u16],
        start: isize,
        length: isize,
        skip_heading_extenders: bool,
        ctx: &mut Context,
    ) -> bool {
        self.compare_internal(
            s,
            start,
            length,
            target,
            0,
            len(target),
            skip_heading_extenders,
            true,
            ctx,
        )
        .target_consumed
    }

    pub(super) fn is_suffix(&self, s: &[u16], target: &[u16]) -> bool {
        let mut ctx = Context::new(true);
        let mut target_key = [0; 4];
        let index = self.last_index_of(s, target, len(s) - 1, len(s), &mut target_key, &mut ctx);

        index >= 0 && self.compare(s, index, len(s) - index, target, 0, len(target)) == 0
    }

    pub(super) fn last_index_of_ordinal(
        s: &[u16],
        target: &[u16],
        start: isize,
        length: isize,
    ) -> isize {
        if target.is_empty() {
            return start;
        }

        if len(s) < len(target) || len(target) > length {
            return -1;
        }

        let stop = start - length + len(target) - 1;
        let last = at(target, len(target) - 1);
        let mut index = start;

        while index > stop {
            if at(s, index) != last {
                index -= 1;
                continue;
            }

            let from = index - len(target) + 1;

            index -= 1;
            if (0..len(target) - 1)
                .rev()
                .all(|offset| at(s, from + offset) == at(target, offset))
            {
                return from;
            }
        }

        -1
    }

    pub(super) fn last_index_of(
        &self,
        s: &[u16],
        target: &[u16],
        mut start: isize,
        mut length: isize,
        target_key: &mut [u8; 4],
        ctx: &mut Context,
    ) -> isize {
        let origin = start;
        let mut i = 0;

        while i < len(target) && self.is_ignorable(at(target, i)) {
            i += 1;
        }

        if i == len(target) {
            if !target.contains(&0) {
                return start;
            }

            return Self::last_index_of_ordinal(s, target, start, length);
        }

        let contraction = self.contraction(target, i, len(target) - i);
        let replacement = contraction.and_then(|contraction| contraction.replacement.as_deref());
        let mut no_level4 = true;
        let mut first = -1;

        if replacement.is_none() {
            if let Some(contraction) = contraction {
                *target_key = contraction.sort_key.expect("a sort key");
            } else {
                first = at(target, i);
                target_key[0] = self.category(first);
                target_key[1] = self.level1(first);
                target_key[2] = self.level2_of(first, Extender::None);
                target_key[3] = self.level3(first);
                no_level4 = !has_special_weight(first);
            }

            i += 1;
            while i < len(target) && self.category(at(target, i)) == 1 {
                if target_key[2] == 0 {
                    target_key[2] = 2;
                }

                target_key[2] =
                    target_key[2].wrapping_add(self.level2_of(at(target, i), Extender::None));
                i += 1;
            }
        }

        loop {
            let index = match replacement {
                None => self.last_index_of_sort_key(
                    s,
                    start,
                    origin,
                    length,
                    *target_key,
                    first,
                    no_level4,
                    ctx,
                ),
                Some(replacement) => {
                    self.last_index_of(s, replacement, start, length, target_key, ctx)
                }
            };

            if index < 0 {
                return -1;
            }

            length -= start - index;
            start = index;
            if self.is_prefix(s, target, index, origin - index + 1, false, ctx) {
                let mut hit = index;

                while hit < origin && self.is_ignorable(at(s, hit)) {
                    hit += 1;
                }

                return hit;
            }

            let step = self
                .contraction(s, index, origin - index + 1)
                .map_or(1, |contraction| len(&contraction.source));

            start -= step;
            length -= step;
            if length <= 0 {
                return -1;
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn last_index_of_sort_key(
        &self,
        s: &[u16],
        start: isize,
        origin: isize,
        length: isize,
        sort_key: [u8; 4],
        target: i32,
        no_level4: bool,
        ctx: &mut Context,
    ) -> isize {
        let end = start - length;
        let mut index = start;

        while index > end {
            let match_start = index;

            if self.matches_backward(s, &mut index, end, origin, target, sort_key, no_level4, ctx) {
                return match_start;
            }
        }

        -1
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn matches_backward(
        &self,
        s: &[u16],
        index: &mut isize,
        end: isize,
        origin: isize,
        target: i32,
        sort_key: [u8; 4],
        no_level4: bool,
        ctx: &mut Context,
    ) -> bool {
        let unit = at(s, *index);

        if has_flag(ctx.always_match.as_ref(), unit) {
            return true;
        }

        if has_flag(ctx.never_match.as_ref(), unit) {
            *index -= 1;

            return false;
        }

        let extender = extender_type(unit);
        let mut contraction = None;
        let matched = self.matches_backward_core(
            s,
            index,
            end,
            origin,
            target,
            sort_key,
            no_level4,
            extender,
            &mut contraction,
            ctx,
        );

        if contraction.is_none() && extender == Extender::None {
            let flags = if matched {
                ctx.always_match.as_mut()
            } else {
                ctx.never_match.as_mut()
            };

            set_flag(flags, unit);
        }

        matched
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn matches_backward_core<'t>(
        &'t self,
        s: &[u16],
        index: &mut isize,
        end: isize,
        origin: isize,
        target: i32,
        sort_key: [u8; 4],
        no_level4: bool,
        extender: Extender,
        contraction: &mut Option<&'t Contraction>,
        ctx: &mut Context,
    ) -> bool {
        let here = *index;
        let mut cp = -1;

        if extender != Extender::None {
            let mut diacritic = 0;
            let mut back = *index;
            let (base, category) = loop {
                if back < 0 {
                    return false;
                }

                if !self.is_ignorable(at(s, back)) {
                    let base = at(s, back);
                    let category = self.category(base);

                    if category != 1 {
                        break (base, category);
                    }

                    diacritic = self.level2_of(base, Extender::None);
                }

                back -= 1;
            };

            cp = self.filter_extender(base, extender);
            ctx.buffer1[0] = category;
            ctx.buffer1[1] = self.level1(cp);
            ctx.buffer1[2] = self.level2_of(cp, extender);
            ctx.buffer1[3] = self.level3(cp);
            if extender != Extender::Conditional && diacritic != 0 {
                ctx.buffer1[2] = if ctx.buffer1[2] == 0 {
                    diacritic.wrapping_add(2)
                } else {
                    diacritic
                };
            }

            *index -= 1;
        }

        if extender == Extender::None {
            *contraction = self.tail_contraction(s, *index, end);
        }

        if let Some(hit) = *contraction {
            *index -= len(&hit.source);
            if !no_level4 {
                return false;
            }

            let Some(_) = hit.sort_key else {
                let replacement = hit.replacement.as_deref().expect("a replacement");
                let last = len(replacement) - 1;

                return self.last_index_of_sort_key(
                    replacement,
                    last,
                    last,
                    len(replacement),
                    sort_key,
                    target,
                    no_level4,
                    ctx,
                ) >= 0;
            };

            ctx.buffer1 = sort_key;
            ctx.prev_code = -1;
            ctx.prev_sort_key = true;
        } else if extender == Extender::None {
            cp = at(s, *index);
            *index -= 1;
            ctx.buffer1[0] = self.category(cp);

            let mut differs = false;

            if ctx.buffer1[0] == sort_key[0] {
                ctx.buffer1[1] = self.level1(cp);
            } else {
                differs = true;
            }

            if ctx.buffer1[1] == sort_key[1] {
                ctx.buffer1[2] = self.level2_of(cp, extender);
            } else {
                differs = true;
            }

            if differs {
                return false;
            }

            ctx.buffer1[3] = self.level3(cp);
            if ctx.buffer1[0] != 1 {
                ctx.prev_code = cp;
            }
        }

        if extender == Extender::None {
            let mut next = here + 1;

            while next < origin && self.category(at(s, next)) == 1 {
                if ctx.buffer1[2] == 0 {
                    ctx.buffer1[2] = 2;
                }

                ctx.buffer1[2] =
                    ctx.buffer1[2].wrapping_add(self.level2_of(at(s, next), Extender::None));
                next += 1;
            }
        }

        Self::matches_primitive(ctx.buffer1, cp, extender, sort_key, target, no_level4)
    }

    pub(super) fn matches_primitive(
        source: [u8; 4],
        cp: i32,
        extender: Extender,
        target: [u8; 4],
        target_cp: i32,
        no_level4: bool,
    ) -> bool {
        if source != target {
            return false;
        }

        if no_level4 && (cp < 0 || !has_special_weight(cp)) {
            return true;
        }

        if no_level4 || extender == Extender::Conditional {
            return false;
        }

        is_japanese_small_letter(cp) == is_japanese_small_letter(target_cp)
            && dash_type(extender) == dash_type(Extender::None)
            && is_hiragana(cp) == is_hiragana(target_cp)
            && is_half_width_kana(cp) == is_half_width_kana(target_cp)
    }
}

pub(super) fn len(units: &[u16]) -> isize {
    isize::try_from(units.len()).expect("a .NET string length")
}

pub(super) fn at(units: &[u16], index: isize) -> i32 {
    i32::from(units[usize::try_from(index).expect("a position in the text")])
}
