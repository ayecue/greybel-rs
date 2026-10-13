//! Validates protocol versions and their recorded fingerprints.
//! Set `RELEASE_VERSIONS=1` to record unreleased versions.

use std::path::Path;

use duplex_protocol::VersionHistory;

use crate::support::folders::{DUPLEX_DEFINITIONS, ELSEWHERE, folders};

#[test]
fn should_load_every_folder_of_versions() {
    let mut found = Vec::new();

    folders(Path::new(env!("PROTOCOLS")), &mut found);
    for folder in DUPLEX_DEFINITIONS {
        folders(&Path::new(env!("DUPLEX")).join(folder), &mut found);
    }

    found.extend(ELSEWHERE.map(|folder| Path::new(env!("DUPLEX_TS")).join(folder)));
    found.sort();

    let failures: Vec<String> = found
        .iter()
        .filter_map(|folder| {
            if std::env::var_os("RELEASE_VERSIONS").is_some()
                && let Err(error) = VersionHistory::release(folder)
            {
                return Some(error.to_string());
            }

            VersionHistory::load(folder)
                .err()
                .map(|error| error.to_string())
        })
        .collect();

    assert!(
        found.len() > ELSEWHERE.len(),
        "no folders under protocols/ or duplex/"
    );
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}
