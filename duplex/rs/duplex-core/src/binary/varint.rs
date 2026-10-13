pub(crate) const CONTINUATION: u8 = 0x80;
pub(crate) const PAYLOAD: u8 = 0x7f;
pub(super) const SHIFT: u32 = 7;

/// Varuint encoding with seven value bits per byte, least significant group first, and the high bit clear only on the last byte.
#[derive(Debug, Clone, Copy)]
pub struct Varuint {
    bytes: [u8; Self::MAX_BYTES],
    size: u8,
}

impl Varuint {
    /// Maximum encoded byte length of a `u64`.
    pub const MAX_BYTES: usize = 10;

    /// Encodes `value` as varuint.
    #[must_use]
    pub fn new(mut value: u64) -> Self {
        let mut bytes = [0; Self::MAX_BYTES];
        let mut size = 0;

        while value >= u64::from(CONTINUATION) {
            bytes[usize::from(size)] = (Self::low_byte(value) & PAYLOAD) | CONTINUATION;
            value >>= SHIFT;
            size += 1;
        }

        bytes[usize::from(size)] = Self::low_byte(value);

        Self {
            bytes,
            size: size + 1,
        }
    }

    /// Returns the encoded byte length of `value`.
    #[must_use]
    pub fn size(value: u64) -> usize {
        (u64::BITS - value.max(1).leading_zeros()).div_ceil(SHIFT) as usize
    }

    /// Returns the encoded bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..usize::from(self.size)]
    }

    fn low_byte(value: u64) -> u8 {
        value.to_le_bytes()[0]
    }
}

pub(super) trait ZigZag {
    fn zigzag(self) -> u64;
}

impl ZigZag for i64 {
    fn zigzag(self) -> u64 {
        ((self << 1) ^ (self >> 63)).cast_unsigned()
    }
}

pub(super) trait UnZigZag {
    fn unzigzag(self) -> i64;
}

impl UnZigZag for u64 {
    fn unzigzag(self) -> i64 {
        (self >> 1).cast_signed() ^ -((self & 1).cast_signed())
    }
}
