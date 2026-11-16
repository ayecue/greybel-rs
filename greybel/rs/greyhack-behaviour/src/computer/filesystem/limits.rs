//! File-system limits and the checks of names and content against them.

use crate::base::refusal::{NameKind, Refusal, RuleResult};
use crate::path;

/// Maximum length of a file or folder name.
pub const MAX_NAME: usize = 128;
/// Maximum number of nodes per file system.
pub const MAX_FILES: usize = 3125;
/// Maximum number of entries per folder.
pub const MAX_FILES_PER_FOLDER: usize = 250;
/// Symlink-resolution limit.
pub const MAX_SYMLINKS: u32 = 3;
/// Metadata bytes added to each folder and text file by `GetSizeBytes(includeMetadata)`.
pub const METADATA_SIZE: u64 = 14_000;
/// Number of bytes in one megabyte of disk space.
pub const BYTES_PER_MB: f64 = 1_000_000.0;

/// Converts bytes with the unrounded `(float)bytes * 1E-06f` expression used in disk comparisons.
#[must_use]
#[expect(
    clippy::cast_precision_loss,
    reason = "C# converts the byte count to float before multiplying it by 1E-06f"
)]
pub fn file_size_mb(bytes: u64) -> f64 {
    f64::from(bytes as f32) * f64::from(1E-6_f32)
}

/// Converts bytes as `Carpeta.GetSizeMB`, which stores the expression in its float return value.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    reason = "Carpeta.GetSizeMB returns the unrounded expression as a C# float"
)]
pub fn stored_file_size_mb(bytes: u64) -> f32 {
    file_size_mb(bytes) as f32
}

/// File-content limit in UTF-16 code units.
pub const MAX_CONTENT: usize = 160_000;
/// `import_code` size limit in UTF-16 code units.
pub const MAX_IMPORTED_CODE: usize = 4_000_000;

/// Validates the new folder name passed to `create_folder`.
pub fn check_folder_name(name: &str) -> RuleResult {
    check_name(name, NameKind::Folder)
}

/// Validates the new file name passed to `touch`.
pub fn check_file_name(name: &str) -> RuleResult {
    check_name(name, NameKind::File)
}

/// Validates the new name passed to `rename`.
pub fn check_new_name(name: &str) -> RuleResult {
    check_name(name, NameKind::New)
}

/// Validates the destination name used by `copy`, `move`, or `symlink`.
pub fn check_target_name(name: &str) -> RuleResult {
    if !path::is_valid_name(name) {
        Err(Refusal::InvalidName(NameKind::New))
    } else if name.chars().count() > MAX_NAME {
        Err(Refusal::NameTooLong)
    } else {
        Ok(())
    }
}

fn check_name(name: &str, kind: NameKind) -> RuleResult {
    if !path::is_valid_name(name) {
        Err(Refusal::InvalidName(kind))
    } else if name.chars().count() > MAX_NAME {
        Err(Refusal::LongName)
    } else {
        Ok(())
    }
}

/// Validates `set_content` input using the game's UTF-16 length limit.
pub fn check_content(content: &str) -> RuleResult {
    if content.encode_utf16().count() > MAX_CONTENT {
        Err(Refusal::ContentTooLong)
    } else {
        Ok(())
    }
}
