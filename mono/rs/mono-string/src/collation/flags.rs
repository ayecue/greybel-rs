fn flag(unit: i32) -> Option<(usize, u8)> {
    let unit = usize::try_from(unit).ok().filter(|&unit| unit < 128)?;

    Some((unit / 8, 1 << (unit % 8)))
}

pub(super) fn has_flag(flags: Option<&[u8; 16]>, unit: i32) -> bool {
    flags.is_some_and(|flags| flag(unit).is_some_and(|(at, bit)| flags[at] & bit != 0))
}

pub(super) fn set_flag(flags: Option<&mut [u8; 16]>, unit: i32) {
    if let (Some(flags), Some((at, bit))) = (flags, flag(unit)) {
        flags[at] |= bit;
    }
}

pub(super) fn compare_flag_pair(first: bool, second: bool) -> i32 {
    match (first, second) {
        (true, false) => 1,
        (false, true) => -1,
        _ => 0,
    }
}
