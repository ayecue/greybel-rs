//! The data of a file: its kind, content, size, and command.

use greybel_source_map::SourceMap;

use super::kinds::{Command, FileKind};

#[derive(Debug, Clone, PartialEq)]
pub struct FileData {
    pub kind: FileKind,
    pub content: String,
    /// Application launched by the file when its raw command names a known GUI program.
    pub command: Option<Command>,
    /// Unmodified game command string (`Fichero.comando`), also used to identify apt caches and record viewers.
    pub command_name: String,
    /// Binary size in bytes, or text length in UTF-16 code units as recorded by `Archivo.SetContenido`.
    pub size: u64,
    pub allow_import: bool,
    pub encrypt_password: String,
    /// Source locations for built-program lines, used by runtime errors.
    pub source_map: SourceMap,
    /// Marks one of the game's built-in compiled scripts (`Archivo.isDefaultContent`).
    pub default_content: bool,
}

impl Default for FileData {
    fn default() -> Self {
        Self {
            kind: FileKind::Text,
            content: String::new(),
            command: None,
            command_name: String::new(),
            size: 0,
            allow_import: false,
            encrypt_password: String::new(),
            source_map: SourceMap::default(),
            default_content: false,
        }
    }
}

impl FileData {
    #[must_use]
    pub fn text(content: &str) -> Self {
        let mut data = Self::default();

        data.set_content(content);

        data
    }

    /// Creates binary data of `size` bytes.
    #[must_use]
    pub fn binary(size: u64) -> Self {
        Self {
            kind: FileKind::Binary,
            size,
            ..Self::default()
        }
    }

    /// Creates an application binary of `size` bytes that launches `command`.
    #[must_use]
    pub fn application(command: Command, size: u64) -> Self {
        Self {
            command: Some(command),
            command_name: command.name().to_owned(),
            ..Self::binary(size)
        }
    }

    /// Creates a library file of `size` bytes.
    #[must_use]
    pub fn library(size: u64) -> Self {
        Self {
            kind: FileKind::Library,
            ..Self::binary(size)
        }
    }

    /// Derives size from the filename like `CacheBinarySize.GetSize`: 110,000–7,100,009 bytes,
    /// with a maximum of 650,009 bytes for libraries.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "CacheBinarySize.GetSize truncates weighted doubles like C# (int)"
    )]
    pub fn binary_size(name: &str, library: bool) -> u64 {
        const MIN: f64 = 110_000.0;
        let span = if library { 650_000.0 } else { 7_100_000.0 } - MIN;
        let weight: f64 = name
            .encode_utf16()
            .zip(1..)
            .map(|(unit, position)| f64::from(unit) * f64::from(position))
            .sum();
        let scaled_size = (MIN + span * ((weight % 1000.0) / 1000.0)) as u64;

        scaled_size + (weight % 10.0) as u64
    }

    /// Replaces content and updates its size like `Archivo.SetContenido`.
    pub fn set_content(&mut self, content: &str) {
        content.clone_into(&mut self.content);

        let length = content.encode_utf16().count();

        if !self.is_binary() {
            self.size = length as u64;
        } else if self.kind == FileKind::Program && !self.default_content {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                clippy::cast_precision_loss,
                reason = "Archivo.SetContenido uses C# float arithmetic and integral byte sizes"
            )]
            {
                self.size = (length as f32 * 0.65).max(1.0) as u64;
            }
        }
    }

    /// Creates program data from GreyScript source.
    #[must_use]
    pub fn script(source: &str) -> Self {
        Self {
            kind: FileKind::Program,
            content: source.to_owned(),
            ..Self::default()
        }
    }

    /// Creates an `aireplay` capture containing JSON consumed by `aircrack`.
    #[must_use]
    pub fn capture(content: &str) -> Self {
        Self {
            content: content.to_owned(),
            ..Self::binary(content.len() as u64)
        }
    }

    /// Creates a program that retains source for `launch` and derives its size from the source.
    #[must_use]
    pub fn binary_from_source(source: &str) -> Self {
        let mut data = Self::script(source);

        data.set_content(source);

        data
    }

    #[must_use]
    pub fn is_encrypted(&self) -> bool {
        !self.encrypt_password.is_empty()
    }

    #[must_use]
    pub fn is_executable(&self) -> bool {
        !self.is_encrypted() && (!self.command_name.is_empty() || self.kind == FileKind::Program)
    }

    #[must_use]
    pub fn is_binary(&self) -> bool {
        self.kind.is_binary() || self.is_executable() || self.is_encrypted()
    }
}
