use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

use super::{PathContext, ReferenceError};

/// Project's record of the editor version it was saved with.
const PROJECT_VERSION: &str = "ProjectSettings/ProjectVersion.txt";
const EDITOR_VERSION_KEY: &str = "m_EditorVersion:";
/// Unity Hub editor metadata in its settings folder.
const HUB_INFO: &str = "hubInfo.json";
const HUB_FOLDER: &str = "UnityHub";
const EDITOR_PATH_PREFIX: &str = " installed at ";
/// Editor's executable inside a macOS `Unity.app`.
const APP_EXECUTABLE: &str = "Contents/MacOS/Unity";
const APP_EXTENSION: &str = "app";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HubInfo {
    executable_path: PathBuf,
}

/// Unity editor of the version `project` was saved with: `UNITY` when set, else the one Unity Hub has installed.
pub(super) fn locate(project: &Path) -> Result<PathBuf, ReferenceError> {
    if let Some(editor) = std::env::var_os("UNITY") {
        return Ok(PathBuf::from(editor));
    }

    let version = editor_version(project)?;
    let hub = hub_executable()?;
    let output = Command::new(&hub)
        .args(["--", "--headless", "editors", "--installed"])
        .output()
        .at(&hub)?;
    let editors = String::from_utf8_lossy(&output.stdout);
    let install = editors
        .lines()
        .find(|line| line.split_whitespace().next() == Some(version.as_str()))
        .and_then(|line| line.split_once(EDITOR_PATH_PREFIX))
        .map(|(_, path)| PathBuf::from(path.trim()))
        .ok_or_else(|| {
            ReferenceError::MissingEditor(format!(
                "Unity Hub has no Unity {version}, which {PROJECT_VERSION} names, installed:\n{editors}"
            ))
        })?;

    if install
        .extension()
        .is_some_and(|extension| extension == APP_EXTENSION)
    {
        Ok(install.join(APP_EXECUTABLE))
    } else {
        Ok(install)
    }
}

fn editor_version(project: &Path) -> Result<String, ReferenceError> {
    let path = project.join(PROJECT_VERSION);

    fs::read_to_string(&path)
        .at(&path)?
        .lines()
        .find_map(|line| line.strip_prefix(EDITOR_VERSION_KEY))
        .map(|version| version.trim().to_owned())
        .ok_or_else(|| {
            ReferenceError::MissingEditor(format!("{} has no {EDITOR_VERSION_KEY}", path.display()))
        })
}

/// Locates the Unity Hub executable from Hub's settings-folder metadata.
fn hub_executable() -> Result<PathBuf, ReferenceError> {
    let path = hub_settings()?.join(HUB_INFO);
    let info: HubInfo = serde_json::from_str(&fs::read_to_string(&path).at(&path)?)
        .map_err(|error| ReferenceError::MissingEditor(format!("{}: {error}", path.display())))?;

    Ok(info.executable_path)
}

/// Folder of Unity Hub's settings: below the platform's folder of application settings.
fn hub_settings() -> Result<PathBuf, ReferenceError> {
    let variable = |name: &'static str| {
        std::env::var_os(name).map(PathBuf::from).ok_or_else(|| {
            ReferenceError::MissingEditor(format!(
                "{name} is not set, which Unity Hub's settings are below; set UNITY instead"
            ))
        })
    };
    let settings = if cfg!(target_os = "macos") {
        variable("HOME")?.join("Library/Application Support")
    } else if cfg!(windows) {
        variable("APPDATA")?
    } else {
        variable("XDG_CONFIG_HOME")
            .or_else(|_| variable("HOME").map(|home| home.join(".config")))?
    };

    Ok(settings.join(HUB_FOLDER))
}
