use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

use super::error::{PathContext, ReferenceError};

/// Serializes `value` as fixture JSON with one-space indentation and a trailing newline.
pub(super) fn write_json(value: &Value, path: &Path) -> Result<(), ReferenceError> {
    let mut text = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b" ");
    let mut serializer = serde_json::Serializer::with_formatter(&mut text, formatter);

    serde::Serialize::serialize(value, &mut serializer)?;
    text.push(b'\n');

    fs::write(path, text).at(path)
}

pub(super) fn run_tool(command: &mut Command) -> Result<(), ReferenceError> {
    let output = command.output().map_err(|source| ReferenceError::Io {
        path: PathBuf::from(command.get_program()),
        source,
    })?;

    if output.status.success() {
        return Ok(());
    }

    Err(ReferenceError::Command {
        command: format!("{command:?}"),
        detail: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

pub(super) fn replace_folder(folder: &Path) -> Result<(), ReferenceError> {
    if folder.exists() {
        fs::remove_dir_all(folder).at(folder)?;
    }

    fs::create_dir_all(folder).at(folder)
}

/// Copies entries from `from` to `to` when their names satisfy `accepts`; accepted directories are copied in full.
pub(super) fn copy_folder(
    from: &Path,
    to: &Path,
    accepts: &dyn Fn(&str) -> bool,
) -> Result<(), ReferenceError> {
    fs::create_dir_all(to).at(to)?;
    for entry in fs::read_dir(from).at(from)? {
        let entry = entry.at(from)?;
        let name = entry.file_name();

        if !accepts(&name.to_string_lossy()) {
            continue;
        }

        let source = entry.path();
        let target = to.join(&name);

        if entry.file_type().at(&source)?.is_dir() {
            copy_folder(&source, &target, &|_| true)?;
        } else {
            fs::copy(&source, &target).at(&source)?;
        }
    }

    Ok(())
}
