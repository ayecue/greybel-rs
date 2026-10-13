//! Path helpers (`Util.Path`), including root-relative paths without a leading slash.

use std::borrow::Cow;

pub const SEPARATOR: char = '/';
pub const ROOT: &str = "/";
/// Maximum path length accepted by `get_abs_path`, in string positions.
pub const MAX_PATH: usize = 1024;
/// Maximum explicit base-path length accepted by `get_abs_path`, in string positions.
pub const MAX_BASE_PATH: usize = 64;
const MAX_FOLDERS: usize = 16;

/// Normalizes `path` to one leading slash with no trailing or duplicate separators. Returns a borrow
/// when already normalized; empty and all-separator paths become `/`.
#[must_use]
pub fn normalize(path: &str) -> Cow<'_, str> {
    if path == ROOT || (path.starts_with('/') && !path.ends_with('/') && !path[1..].contains("//"))
    {
        return Cow::Borrowed(path);
    }

    let segments: Vec<&str> = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();

    if segments.is_empty() {
        Cow::Borrowed(ROOT)
    } else {
        Cow::Owned(format!("/{}", segments.join("/")))
    }
}

/// Splits path segments like `SplitPath`; `/` returns `["/"]`, and empty input returns `None`.
#[must_use]
pub fn split(path: &str) -> Option<Vec<&str>> {
    if path.is_empty() {
        return None;
    }

    if !path.contains(SEPARATOR) {
        return Some(vec![path]);
    }

    if path == ROOT {
        return Some(vec![ROOT]);
    }

    let path = path.strip_prefix(SEPARATOR).unwrap_or(path);
    let path = path.strip_suffix(SEPARATOR).unwrap_or(path);

    Some(path.split(SEPARATOR).collect())
}

/// Returns each segment of `path` with the path from the root through it, so `/a/b` gives `a` with
/// `/a` and `b` with `/a/b`. Gives nothing for an empty path.
#[must_use]
pub fn steps(path: &str) -> Vec<(&str, String)> {
    let mut full = String::new();

    split(path)
        .into_iter()
        .flatten()
        .map(|segment| {
            full.push(SEPARATOR);
            full.push_str(segment);

            (segment, full.clone())
        })
        .collect()
}

/// Extracts the final segment with `GetFileName` semantics.
#[must_use]
pub fn file_name(path: &str) -> &str {
    split(path)
        .and_then(|segments| segments.last().copied())
        .unwrap_or("")
}

/// Removes the final segment with `GetPathNoFile` semantics. Root files return `/`; a single segment
/// remains unchanged.
#[must_use]
pub fn parent(path: &str) -> &str {
    let name = file_name(path);

    if name == path {
        return path;
    }

    let end = path.len().saturating_sub(name.len() + 1);

    match &path[..end] {
        "" => ROOT,
        parent => parent,
    }
}

/// Checks whether a path exceeds the 16-separator `ReachedMaxFolders` limit.
#[must_use]
pub fn has_too_many_folders(path: &str) -> bool {
    path.matches(SEPARATOR).count() > MAX_FOLDERS
}

/// Joins `parent` and `name` with `/` and collapses doubled separators like Grey Hack.
#[must_use]
pub fn join(parent: &str, name: &str) -> String {
    format!("{parent}/{name}").replace("//", "/")
}

/// Returns absolute `path` unchanged; otherwise joins `path` to `current`.
#[must_use]
pub fn resolve(path: &str, current: &str) -> String {
    if path.starts_with(SEPARATOR) {
        path.to_owned()
    } else {
        join(current, path)
    }
}

/// Checks for a nonempty filename other than `.` using only `[a-zA-Z0-9_.-]`.
#[must_use]
pub fn is_valid_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

/// Checks for a nonempty path without `//` using only `[a-zA-Z0-9-_/.]`.
#[must_use]
pub fn is_valid(path: &str) -> bool {
    !path.is_empty()
        && !path.contains("//")
        && path
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-' | '/'))
}

/// Checks for nonempty text containing only `[a-zA-Z0-9]`.
#[must_use]
pub fn is_alphanumeric(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| c.is_ascii_alphanumeric())
}

/// Removes the final source extension to name the binary built from `name`; `foo` and `.src` have
/// none (`ShellIntrinsics.build`).
#[must_use]
pub fn binary_name(name: &str) -> Option<&str> {
    match name.rfind('.') {
        Some(dot) if dot > 0 => Some(&name[..dot]),
        _ => None,
    }
}

/// Returns `user`'s home directory.
#[must_use]
pub fn home(user: &str) -> String {
    if user == crate::ROOT_USER {
        "/root".to_owned()
    } else {
        format!("/home/{user}")
    }
}

/// Resolves `~`, `.`, and `..` against `current` with `PlayerUtils.GetAbsolutePath` semantics. Only the
/// root path retains a trailing separator.
#[must_use]
pub fn absolute(path: &str, current: &str, user: &str) -> String {
    let home = home(user);
    let mut text = if path == "~" {
        home
    } else if let Some(remainder) = path.strip_prefix("~/") {
        format!("{home}/{remainder}")
    } else {
        path.to_owned()
    };

    if text == "." || text == "./" {
        return current.to_owned();
    }

    if let Some(remainder) = text.strip_prefix("./") {
        text = format!("{current}/{remainder}");
    }

    if !text.starts_with(SEPARATOR) {
        text = format!("{current}/{text}");
    }

    text = format!("{text}/").replace("//", "/");
    if text.contains("../") {
        text = resolve_parents(&text);
    }

    if text == ROOT {
        text
    } else {
        text.trim_end_matches(SEPARATOR).to_owned()
    }
}

/// Applies `GetConstructedPath` segment rules: `..` pops the previous segment and `.` is ignored.
fn resolve_parents(path: &str) -> String {
    let mut segments: Vec<&str> = Vec::new();

    for segment in path.split(SEPARATOR).filter(|segment| !segment.is_empty()) {
        match segment {
            ".." => {
                segments.pop();
            }
            "." => {}
            segment => segments.push(segment),
        }
    }

    format!("/{}", segments.join("/"))
}
