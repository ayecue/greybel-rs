pub const SPACE: u8 = b' ';
pub const TAB: u8 = b'\t';
pub const LINE_FEED: u8 = b'\n';
pub const CARRIAGE_RETURN: u8 = b'\r';
pub const UNDERSCORE: u8 = b'_';
pub const FIRST_NON_ASCII: u8 = 0x80;

#[must_use]
pub fn is_whitespace(c: u8) -> bool {
    c == SPACE || c == TAB
}

#[must_use]
pub fn is_end_of_line(c: u8) -> bool {
    c == LINE_FEED || c == CARRIAGE_RETURN
}

#[must_use]
pub fn is_windows_newline(c: u8, next: u8) -> bool {
    matches!(
        (c, next),
        (CARRIAGE_RETURN, LINE_FEED) | (LINE_FEED, CARRIAGE_RETURN)
    )
}

#[must_use]
pub fn is_identifier_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == UNDERSCORE || c >= FIRST_NON_ASCII
}

#[must_use]
pub fn is_identifier_part(c: u8) -> bool {
    IDENTIFIER_PARTS[usize::from(c)]
}

const IDENTIFIER_PARTS: [bool; 256] = {
    let mut parts = [false; 256];
    let mut c = 0;

    while c < 256 {
        let byte = c as u8;

        parts[c] = byte.is_ascii_alphanumeric() || byte == UNDERSCORE || byte >= FIRST_NON_ASCII;
        c += 1;
    }

    parts
};

#[must_use]
pub(crate) fn identifier_end(bytes: &[u8], from: usize) -> usize {
    bytes
        .get(from..)
        .and_then(|remainder| remainder.iter().position(|&byte| !is_identifier_part(byte)))
        .map_or(bytes.len(), |length| from + length)
}
