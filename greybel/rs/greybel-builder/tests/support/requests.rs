//! Shared request builders and settings for builder tests.

use std::collections::BTreeMap;

use super::editor::{Build, TranspileMode, TranspileSettings};

pub fn settings(mode: TranspileMode) -> TranspileSettings {
    TranspileSettings {
        mode,
        obfuscation: false,
        strict_mode: false,
        variables: BTreeMap::new(),
        excluded_names: Vec::new(),
    }
}

/// Default source extensions that imports may omit: `src`, `gs`, and `ms`.
pub const EXTENSIONS: [&str; 3] = ["src", "gs", "ms"];

pub fn build(
    entries: &[&str],
    installer_length: Option<u32>,
    deploy_port: Option<u16>,
) -> Build<'static> {
    Build {
        root: None,
        entries: entries
            .iter()
            .map(|entry| (*entry).to_owned())
            .collect::<Vec<_>>()
            .into(),
        extensions: EXTENSIONS
            .iter()
            .map(|extension| (*extension).to_owned())
            .collect::<Vec<_>>()
            .into(),
        transpile: settings(TranspileMode::Default),
        ingame_directory: "/root/project".into(),
        output_name: None,
        auto_compile: None,
        installer_length,
        deploy_port,
    }
}
