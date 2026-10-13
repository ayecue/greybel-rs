//! Stores deflate-compressed files for embedding and decompresses each file on demand.
//!
//! The little-endian format starts with `BNDL` magic and version 1, followed by a file-name and size table and
//! deflate streams. The `fingerprint` feature verifies decompressed files against SHA-256 hashes.

use std::io::{Read, Write};
use std::ops::Range;
use std::sync::OnceLock;

const MAX_PREALLOCATED_ENTRIES: usize = 4096;

use flate2::Compression;
use flate2::read::DeflateDecoder;
use flate2::write::DeflateEncoder;

#[cfg(feature = "fingerprint")]
mod fingerprint;

#[cfg(feature = "fingerprint")]
pub use fingerprint::Fingerprint;

const MAGIC: &[u8; 4] = b"BNDL";
const VERSION: u8 = 1;

/// Error for an invalid version-1 bundle.
#[derive(Debug, thiserror::Error)]
pub enum BundleError {
    #[error("not a bundle: no BNDL magic")]
    Magic,
    #[error("bundle version {0}, only 1 is read")]
    Version(u8),
    #[error("the bundle ends inside its {0}")]
    Incomplete(&'static str),
    #[error("a file name is not UTF-8")]
    Name,
}

/// Parsed bundle that caches each file after its first decompression.
#[derive(Debug)]
pub struct Bundle {
    bytes: &'static [u8],
    entries: Vec<Entry>,
}

#[derive(Debug)]
struct Entry {
    name: &'static str,
    /// Size of the decompressed file in bytes.
    size: usize,
    /// Byte range containing the file's deflate stream.
    stream: Range<usize>,
    contents: OnceLock<Box<[u8]>>,
}

struct Cursor<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, count: usize, what: &'static str) -> Result<&'a [u8], BundleError> {
        let end = self
            .position
            .checked_add(count)
            .ok_or(BundleError::Incomplete(what))?;
        let slice = self
            .bytes
            .get(self.position..end)
            .ok_or(BundleError::Incomplete(what))?;

        self.position = end;

        Ok(slice)
    }

    fn u16(&mut self, what: &'static str) -> Result<usize, BundleError> {
        let bytes = self.take(2, what)?;

        Ok(usize::from(u16::from_le_bytes([bytes[0], bytes[1]])))
    }

    fn u32(&mut self, what: &'static str) -> Result<usize, BundleError> {
        let bytes = self.take(4, what)?;
        let value = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);

        usize::try_from(value).map_err(|_| BundleError::Incomplete(what))
    }
}

impl Bundle {
    /// Parses the header and file table without decompressing any files.
    ///
    /// # Errors
    ///
    /// Returns an error for unsupported magic or version, non-UTF-8 file names, or tables and streams extending
    /// beyond `bytes`.
    pub fn read(bytes: &'static [u8]) -> Result<Self, BundleError> {
        let mut cursor = Cursor { bytes, position: 0 };

        if cursor.take(4, "magic")? != MAGIC {
            return Err(BundleError::Magic);
        }

        let version = cursor.take(1, "version")?[0];

        if version != VERSION {
            return Err(BundleError::Version(version));
        }

        let count = cursor.u32("file count")?;
        let mut table = Vec::with_capacity(count.min(MAX_PREALLOCATED_ENTRIES));

        for _ in 0..count {
            let length = cursor.u16("table")?;
            let name = std::str::from_utf8(cursor.take(length, "table")?)
                .map_err(|_| BundleError::Name)?;
            let size = cursor.u32("table")?;
            let stream_size = cursor.u32("table")?;

            table.push((name, size, stream_size));
        }

        let mut entries = Vec::with_capacity(table.len());

        for (name, size, stream_size) in table {
            let start = cursor.position;

            cursor.take(stream_size, "contents")?;
            entries.push(Entry {
                name,
                size,
                stream: start..cursor.position,
                contents: OnceLock::new(),
            });
        }

        Ok(Self { bytes, entries })
    }

    /// Returns and caches the decompressed contents of `name`, or `None` if absent.
    ///
    /// # Panics
    ///
    /// Panics if deflate data is corrupt or its decompressed size differs from the file table.
    #[must_use]
    pub fn file(&self, name: &str) -> Option<&[u8]> {
        let entry = self.entry(name)?;

        Some(
            entry
                .contents
                .get_or_init(|| self.inflate(entry).into_boxed_slice()),
        )
    }

    /// Returns an owned decompressed copy of `name`, or `None` if absent. Reads cached contents when available
    /// without adding new entries to the cache.
    ///
    /// # Panics
    ///
    /// Panics for the same reasons as [`Bundle::file`].
    #[must_use]
    pub fn unpack(&self, name: &str) -> Option<Vec<u8>> {
        let entry = self.entry(name)?;

        Some(match entry.contents.get() {
            Some(contents) => contents.to_vec(),
            None => self.inflate(entry),
        })
    }

    /// Returns owned decompressed contents of `name` as UTF-8 text, or `None` if absent.
    ///
    /// # Panics
    ///
    /// Panics for the same reasons as [`Bundle::file`] or when contents are not UTF-8.
    #[must_use]
    pub fn unpack_text(&self, name: &str) -> Option<String> {
        self.unpack(name).map(|contents| {
            String::from_utf8(contents).unwrap_or_else(|error| panic!("{name}: not UTF-8: {error}"))
        })
    }

    fn entry(&self, name: &str) -> Option<&Entry> {
        self.entries.iter().find(|entry| entry.name == name)
    }

    fn inflate(&self, entry: &Entry) -> Vec<u8> {
        let mut contents = Vec::with_capacity(entry.size);

        DeflateDecoder::new(&self.bytes[entry.stream.clone()])
            .read_to_end(&mut contents)
            .unwrap_or_else(|error| panic!("{}: broken deflate stream: {error}", entry.name));
        assert_eq!(
            contents.len(),
            entry.size,
            "{}: unpacked to another size",
            entry.name
        );

        contents
    }

    /// Returns cached decompressed contents of `name` as UTF-8 text, or `None` if absent.
    ///
    /// # Panics
    ///
    /// Panics for the same reasons as [`Bundle::file`] or when contents are not UTF-8.
    #[must_use]
    pub fn text(&self, name: &str) -> Option<&str> {
        self.file(name).map(|contents| {
            std::str::from_utf8(contents)
                .unwrap_or_else(|error| panic!("{name}: not UTF-8: {error}"))
        })
    }

    /// Returns file names in their bundle order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|entry| entry.name)
    }
}

/// Builds a bundle while preserving file insertion order.
#[derive(Debug, Default)]
pub struct BundleWriter {
    files: Vec<(String, Vec<u8>)>,
}

impl BundleWriter {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_file(mut self, name: impl Into<String>, contents: impl Into<Vec<u8>>) -> Self {
        self.files.push((name.into(), contents.into()));

        self
    }

    /// Encodes the bundle using maximum deflate compression for every file.
    ///
    /// # Panics
    ///
    /// Panics if a file name exceeds 65,535 bytes or a file's original or compressed size exceeds `u32::MAX`.
    #[must_use]
    pub fn finish(self) -> Vec<u8> {
        let streams: Vec<Vec<u8>> = self
            .files
            .iter()
            .map(|(_, contents)| {
                let mut encoder = DeflateEncoder::new(Vec::new(), Compression::best());

                encoder
                    .write_all(contents)
                    .expect("writing to memory succeeds");

                encoder.finish().expect("writing to memory succeeds")
            })
            .collect();
        let size = |length: usize| {
            u32::try_from(length)
                .expect("a file under 4 GiB")
                .to_le_bytes()
        };
        let mut bytes = MAGIC.to_vec();

        bytes.push(VERSION);
        bytes.extend(size(self.files.len()));
        for ((name, contents), stream) in self.files.iter().zip(&streams) {
            bytes.extend(
                u16::try_from(name.len())
                    .expect("a name under 64 KiB")
                    .to_le_bytes(),
            );
            bytes.extend(name.as_bytes());
            bytes.extend(size(contents.len()));
            bytes.extend(size(stream.len()));
        }

        for stream in streams {
            bytes.extend(stream);
        }

        bytes
    }
}
