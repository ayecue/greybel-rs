mod capture;
mod editor;
mod error;
mod files;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub use capture::Capture;
pub(crate) use error::PathContext;
pub use error::ReferenceError;
use files::{copy_folder, run_tool};

/// TextMesh Pro release the project embeds: the last 3.0.x for Unity 2022.3, whose rendering code is the same as
/// 3.0.6's that Grey Hack ships.
const TMP_VERSION: &str = "3.0.9";
const TMP_PACKAGE: &str = "Packages/com.unity.textmeshpro";
const TMP_ESSENTIALS: &str = "Package Resources/TMP Essential Resources.unitypackage";
/// Limits `<size=…>` to 60 like Grey Hack's `TMP_Text.ValidateHtmlTag`; text-mesh-core applies the same cap.
const SIZE_CAP: u32 = 60;
/// `m_currentFontSize` assignments in the `<size>` case of `TMP_Text.ValidateHtmlTag`, with occurrence counts.
const SIZE_ASSIGNMENTS: [(&str, usize); 4] = [
    ("m_currentFontSize = m_fontSize + value;", 2),
    ("m_currentFontSize = value;", 1),
    ("m_currentFontSize = m_fontSize * value;", 1),
    ("m_currentFontSize = m_fontSize * value / 100;", 1),
];

#[derive(Debug, Clone)]
pub struct Reference {
    editor: PathBuf,
    project: PathBuf,
}

impl Reference {
    /// Project `TEXT_MESH_REFERENCE` names, with the editor `UNITY` names, else the one Unity Hub has installed of
    /// the version the project was saved with (Grey Hack's).
    ///
    /// # Errors
    /// [`ReferenceError::Unset`] outside the repository; [`ReferenceError::MissingEditor`] when no editor is found.
    pub fn locate() -> Result<Self, ReferenceError> {
        let project = PathBuf::from(
            option_env!("TEXT_MESH_REFERENCE")
                .ok_or(ReferenceError::Unset("TEXT_MESH_REFERENCE"))?,
        );

        Ok(Self {
            editor: editor::locate(&project)?,
            project,
        })
    }

    /// Bakes missing FontEngine data for fonts not covered by `export`.
    ///
    /// # Errors
    /// Returns an error if TMP embedding or Unity execution fails, or if `output` cannot be written.
    pub fn record_fonts(&self, setup: &Path, output: &Path) -> Result<(), ReferenceError> {
        if output.exists() {
            fs::remove_dir_all(output).at(output)?;
        }

        self.run(
            "FontRecorder",
            &[("-setup", setup), ("-recordOutput", output)],
        )
    }

    /// Writes `setup` to `folder` as `text-mesh.json` with `atlases/`.
    ///
    /// # Errors
    /// Returns an error if TMP embedding or Unity execution fails.
    pub fn export(&self, setup: &Path, folder: &Path) -> Result<(), ReferenceError> {
        self.run(
            "SetupExport",
            &[("-setup", setup), ("-exportOutput", folder)],
        )
    }

    /// Generates `setup` and records each `cases` line through its terminal at `width` pixels.
    ///
    /// # Errors
    /// Returns an error if TMP embedding or Unity execution fails, or if captures cannot be read.
    pub fn capture(
        &self,
        setup: &Path,
        cases: &str,
        width: u32,
    ) -> Result<Capture, ReferenceError> {
        let folder =
            std::env::temp_dir().join(format!("text-mesh-reference-{}", std::process::id()));

        if folder.exists() {
            fs::remove_dir_all(&folder).at(&folder)?;
        }

        fs::create_dir_all(&folder).at(&folder)?;

        let input = folder.join("cases.txt");

        fs::write(&input, cases).at(&input)?;

        let output = folder.join("captures");
        let width = width.to_string();

        self.run(
            "Capture",
            &[
                ("-setup", setup),
                ("-captureInput", &input),
                ("-captureOutput", &output),
                ("-captureWidth", Path::new(&width)),
            ],
        )?;

        let index_path = output.join("captures.json");
        let index = serde_json::from_str(&fs::read_to_string(&index_path).at(&index_path)?)?;

        Ok(Capture {
            folder: output,
            sources: cases
                .replace("\r\n", "\n")
                .trim_end_matches('\n')
                .split('\n')
                .map(str::to_owned)
                .collect(),
            index,
        })
    }

    fn run(&self, command: &str, arguments: &[(&str, &Path)]) -> Result<(), ReferenceError> {
        self.embed_text_mesh_pro()?;

        let logs = self.project.join("Logs");

        fs::create_dir_all(&logs).at(&logs)?;

        let log = logs.join(format!("{command}.log"));
        let mut process = Command::new(&self.editor);

        process
            .args(["-batchmode", "-quit", "-projectPath"])
            .arg(&self.project)
            .args([
                "-executeMethod",
                &format!("TextMeshReference.Editor.{command}.Run"),
            ])
            .arg("-logFile")
            .arg(&log);
        for (name, value) in arguments {
            process.arg(name).arg(value);
        }

        let status = process.status().at(&self.editor)?;

        if status.success() {
            return Ok(());
        }

        let tail = fs::read_to_string(&log).unwrap_or_default();
        let tail: Vec<&str> = tail.lines().rev().take(40).collect();

        Err(ReferenceError::Command {
            command: format!("Unity {command}"),
            detail: format!(
                "{status}, the end of {}:\n{}",
                log.display(),
                tail.into_iter().rev().collect::<Vec<_>>().join("\n")
            ),
        })
    }

    /// Installs TextMesh Pro in the reference project and refreshes its Essential Resources.
    fn embed_text_mesh_pro(&self) -> Result<(), ReferenceError> {
        let work = std::env::temp_dir().join(format!("text-mesh-pro-{}", std::process::id()));

        fs::create_dir_all(&work).at(&work)?;

        let package = self.project.join(TMP_PACKAGE);

        if !package.exists() {
            let archive = work.join("package.tgz");

            run_tool(
                Command::new("curl")
                    .args(["-sSfL", "-o"])
                    .arg(&archive)
                    .arg(format!(
                        "https://download.packages.unity.com/com.unity.textmeshpro/-/com.unity.textmeshpro-{TMP_VERSION}.tgz"
                    )),
            )?;
            run_tool(
                Command::new("tar")
                    .arg("-xzf")
                    .arg(&archive)
                    .arg("-C")
                    .arg(&work),
            )?;
            copy_folder(&work.join("package"), &package, &|name| name != "Tests")?;
            Self::cap_sizes(&package.join("Scripts/Runtime/TMP_Text.cs"))?;
        }

        let essentials = work.join("essentials");

        fs::create_dir_all(&essentials).at(&essentials)?;
        run_tool(
            Command::new("tar")
                .arg("-xzf")
                .arg(package.join(TMP_ESSENTIALS))
                .arg("-C")
                .arg(&essentials),
        )?;
        self.unpack_assets(&essentials)?;

        fs::remove_dir_all(&work).at(&work)
    }

    fn cap_sizes(path: &Path) -> Result<(), ReferenceError> {
        let bytes = fs::read(path).at(path)?;
        let mut source =
            String::from_utf8_lossy(bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes))
                .into_owned();

        for (assignment, count) in SIZE_ASSIGNMENTS {
            let actual = source.matches(assignment).count();

            if actual != count {
                return Err(ReferenceError::Command {
                    command: "capping <size>".to_owned(),
                    detail: format!(
                        "expected {count} of {assignment:?} in TMP_Text.cs, not {actual}"
                    ),
                });
            }

            source = source.replace(
                assignment,
                &format!("{assignment} if (m_currentFontSize > {SIZE_CAP}) m_currentFontSize = {SIZE_CAP};"),
            );
        }

        fs::write(path, source).at(path)
    }

    /// Imports an unpacked `.unitypackage` into the project, writing each asset folder's `pathname`, `asset`, and `asset.meta`.
    fn unpack_assets(&self, package: &Path) -> Result<(), ReferenceError> {
        for entry in fs::read_dir(package).at(package)? {
            let folder = entry.at(package)?.path();
            let Ok(pathname) = fs::read_to_string(folder.join("pathname")) else {
                continue;
            };
            let relative = pathname.lines().next().unwrap_or_default();
            let target = self.project.join(relative);
            let asset = folder.join("asset");

            if asset.exists() {
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent).at(parent)?;
                }

                fs::copy(&asset, &target).at(&asset)?;
            } else {
                fs::create_dir_all(&target).at(&target)?;
            }

            let meta = folder.join("asset.meta");

            if meta.exists() {
                let target_meta = PathBuf::from(format!("{}.meta", target.display()));

                fs::copy(&meta, &target_meta).at(&meta)?;
            }
        }

        Ok(())
    }
}
