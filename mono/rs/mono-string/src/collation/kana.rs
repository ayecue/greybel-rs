pub(super) fn has_special_weight(c: i32) -> bool {
    let c = c as u16;

    if c < 0x3041 {
        return false;
    }

    if (0xFF66..0xFF9E).contains(&c) {
        return true;
    }

    if c >= 0x3300 {
        return false;
    }

    if c < 0x309D {
        return c < 0x3099;
    }

    if c < 0x3100 {
        return c != 0x30FB;
    }

    if c < 0x32D0 {
        return false;
    }

    c < 0x32FF
}

pub(super) fn is_half_width_kana(c: i32) -> bool {
    (0xFF66..=0xFF9D).contains(&(c as u16))
}

pub(super) fn is_hiragana(c: i32) -> bool {
    (0x3041..=0x3094).contains(&(c as u16))
}

pub(super) fn is_japanese_small_letter(c: i32) -> bool {
    let c = c as u16;

    if (0xFF67..=0xFF6F).contains(&c) {
        return true;
    }

    c > 0x3040
        && matches!(
            c,
            0x3041
                | 0x3043
                | 0x3045
                | 0x3047
                | 0x3049
                | 0x3063
                | 0x3083
                | 0x3085
                | 0x3087
                | 0x308E
                | 0x30A1
                | 0x30A3
                | 0x30A5
                | 0x30A7
                | 0x30A9
                | 0x30C3
                | 0x30E3
                | 0x30E5
                | 0x30E7
                | 0x30EE
                | 0x30F5
                | 0x30F6
        )
}
