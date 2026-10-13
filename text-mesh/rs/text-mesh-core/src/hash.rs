const GREATER_THAN: u32 = b'>' as u32;
const EQUALS: u32 = b'=' as u32;
const SPACE: u32 = b' ' as u32;

const UPPER_ASCII: &[u8; 128] = b"-------------------------------- !-#$%&-()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[-]^_`ABCDEFGHIJKLMNOPQRSTUVWXYZ{|}~-";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct NameHash(pub(crate) i32);

impl NameHash {
    pub(crate) const fn of(text: &str) -> i32 {
        Self::of_bytes(text.as_bytes(), false)
    }

    pub(crate) const fn of_upper(text: &str) -> i32 {
        Self::of_bytes(text.as_bytes(), true)
    }

    const fn of_bytes(bytes: &[u8], upper: bool) -> i32 {
        let mut hash = Self(0);
        let mut index = 0;

        while index < bytes.len() {
            let byte = if upper {
                bytes[index].to_ascii_uppercase()
            } else {
                bytes[index]
            };

            hash = hash.push(byte as u32);
            index += 1;
        }

        hash.0
    }

    #[allow(clippy::cast_possible_wrap)]
    pub(crate) const fn push(self, code: u32) -> Self {
        Self(
            self.0
                .wrapping_shl(3)
                .wrapping_sub(self.0)
                .wrapping_add(code as i32),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct ValueHash(pub(crate) i32);

impl ValueHash {
    pub(crate) const fn of(text: &str) -> i32 {
        let bytes = text.as_bytes();
        let mut hash = Self(0);
        let mut index = 0;

        while index < bytes.len() {
            hash = hash.push(bytes[index] as u32);
            index += 1;
        }

        hash.0
    }

    #[allow(clippy::cast_possible_wrap)]
    pub(crate) const fn push(self, code: u32) -> Self {
        Self(self.0.wrapping_shl(5).wrapping_add(self.0) ^ code as i32)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct TagHash(pub(crate) i32);

impl TagHash {
    const MAX_LENGTH: usize = 16;

    pub(crate) const fn of(text: &str) -> i32 {
        let bytes = text.as_bytes();
        let mut hash = Self(0);
        let mut index = 0;

        while index < bytes.len() {
            hash = hash.push(bytes[index] as u32);
            index += 1;
        }

        hash.0
    }

    pub(crate) fn read(characters: &[char], start: usize) -> Option<i32> {
        let mut hash = Self(0);

        for index in start..start + Self::MAX_LENGTH {
            let code = *characters.get(index)? as u32;

            if matches!(code, GREATER_THAN | EQUALS | SPACE) {
                return Some(hash.0);
            }

            hash = hash.push(code);
        }

        Some(hash.0)
    }

    #[allow(clippy::cast_possible_wrap)]
    pub(crate) const fn push(self, code: u32) -> Self {
        let upper = if code < 128 {
            UPPER_ASCII[code as usize] as u32
        } else {
            code
        };

        Self(self.0.wrapping_shl(5).wrapping_add(self.0) ^ upper as i32)
    }
}
