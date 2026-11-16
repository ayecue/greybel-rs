use std::io::{Read as _, Write as _};

use duplex_core::Bytes;
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Encodes and decodes game fields named `zip*` that carry text or JSON in protocol `bytes`.
pub trait Payload: Sized {
    /// Compresses `data` and prefixes the game's raw/gzip mode byte.
    fn compress(data: &[u8]) -> Self;

    #[must_use]
    fn from_text(text: &str) -> Self {
        Self::compress(text.as_bytes())
    }

    #[must_use]
    fn from_json<T: Serialize + ?Sized>(value: &T) -> Self {
        Self::compress(serde_json::to_string(value).unwrap_or_default().as_bytes())
    }

    /// Decompresses the raw or gzip data following a mode byte.
    ///
    /// # Errors
    ///
    /// Fails for malformed gzip data or decompression errors.
    fn decompress(&self) -> std::io::Result<Vec<u8>>;

    /// Decodes dearchive text as lossy UTF-8; empty and invalid payloads return an empty string.
    #[must_use]
    fn text(&self) -> String {
        self.decompress()
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .unwrap_or_default()
    }

    /// Deserializes dearchive JSON into `T`; invalid payloads and JSON return `None`.
    #[must_use]
    fn json<T: DeserializeOwned>(&self) -> Option<T> {
        serde_json::from_str(&self.text()).ok()
    }
}

impl Payload for Bytes {
    fn compress(data: &[u8]) -> Self {
        if data.len() < 24 {
            return Self(raw(data));
        }

        let mut archive = Vec::with_capacity(data.len() + 1);

        archive.push(1);
        {
            let mut encoder = GzEncoder::new(&mut archive, Compression::default());

            encoder.write_all(data).expect("a Vec writes");
            encoder.finish().expect("a Vec writes");
        }

        if base64_len(archive.len()) >= base64_len(data.len() + 1) {
            Self(raw(data))
        } else {
            Self(archive)
        }
    }

    fn decompress(&self) -> std::io::Result<Vec<u8>> {
        decompress(&self.0)
    }
}

fn raw(data: &[u8]) -> Vec<u8> {
    let mut raw = Vec::with_capacity(data.len() + 1);

    raw.push(0);
    raw.extend_from_slice(data);

    raw
}

fn base64_len(bytes: usize) -> usize {
    bytes.div_ceil(3) * 4
}

pub(crate) fn decompress(bytes: &[u8]) -> std::io::Result<Vec<u8>> {
    let Some((mode, payload)) = bytes.split_first() else {
        return Ok(Vec::new());
    };

    if *mode == 0 {
        return Ok(payload.to_vec());
    }

    let mut data = Vec::new();

    GzDecoder::new(payload).read_to_end(&mut data)?;

    Ok(data)
}
