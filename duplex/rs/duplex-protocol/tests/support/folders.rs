use std::path::{Path, PathBuf};

pub const EXTENSION: &str = "protocol";

/// Definition folders under `duplex/`: the core definition and conformance protocols.
pub const DUPLEX_DEFINITIONS: [&str; 2] = ["core", "conformance"];

pub const ELSEWHERE: [&str; 2] = [
    "duplex-driver-worker/test/support/echo",
    "duplex-driver-worker/test/support/offered-echo",
];

pub fn folders(folder: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    let mut has_definitions = false;

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() {
            folders(&path, found);
        } else if path
            .extension()
            .is_some_and(|extension| extension == EXTENSION)
        {
            has_definitions = true;
        }
    }

    if has_definitions {
        found.push(folder.to_owned());
    }
}
