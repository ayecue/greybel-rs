use hyper::header::HeaderValue;

const BYTES: usize = 16;
const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

/// Random server-assigned transport ID encoded as 32 hexadecimal header digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct TransportId([u8; BYTES]);

impl TransportId {
    pub(crate) fn random() -> Option<Self> {
        let mut bytes = [0; BYTES];

        getrandom::fill(&mut bytes).ok()?;

        Some(Self(bytes))
    }

    pub(crate) fn parse(text: &[u8]) -> Option<Self> {
        let (pairs, []) = text.as_chunks::<2>() else {
            return None;
        };

        if pairs.len() != BYTES {
            return None;
        }

        let mut bytes = [0; BYTES];

        for (byte, &[high, low]) in bytes.iter_mut().zip(pairs) {
            *byte = (Self::digit(high)? << 4) | Self::digit(low)?;
        }

        Some(Self(bytes))
    }

    pub(crate) fn to_header(self) -> HeaderValue {
        let mut text = [0; BYTES * 2];

        for (pair, byte) in text.as_chunks_mut::<2>().0.iter_mut().zip(self.0) {
            *pair = [
                HEX_DIGITS[usize::from(byte >> 4)],
                HEX_DIGITS[usize::from(byte & 0xf)],
            ];
        }

        HeaderValue::from_bytes(&text).expect("hexadecimal digits make a header value")
    }

    fn digit(character: u8) -> Option<u8> {
        char::from(character)
            .to_digit(16)
            .and_then(|digit| u8::try_from(digit).ok())
    }
}

#[cfg(test)]
#[path = "tests/transport_id.rs"]
mod tests;
