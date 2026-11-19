use std::path::{Path, PathBuf};

use super::MessageHook;
use crate::bepinex::BepInExMajor;
use crate::error::{SetupError, SetupResult};
use crate::game::GameLayout;

/// Installs and removes the message hook's assembly in `BepInEx/plugins`, together with its `.pdb`. Only one of the
/// two BepInEx builds is installed at a time.
pub struct HookInstaller<'a> {
    game: &'a GameLayout,
}

impl<'a> HookInstaller<'a> {
    const EXTENSIONS: [&'static str; 2] = ["dll", "pdb"];

    #[must_use]
    pub fn new(game: &'a GameLayout) -> Self {
        Self { game }
    }

    /// Installs `assembly`, which must be the build for `major`, in place of any installed build.
    pub fn install(&self, major: BepInExMajor, assembly: &Path) -> SetupResult<PathBuf> {
        let name = MessageHook::assembly_name(major);

        if assembly.file_stem().is_none_or(|stem| stem != name) {
            return Err(SetupError::HookFileWrong {
                path: assembly.to_path_buf(),
                major: major.number(),
                assembly_name: name,
            });
        }

        self.remove()?;

        let plugins = self.game.plugins_folder();
        let target = plugins.join(format!("{name}.dll"));

        std::fs::create_dir_all(&plugins).map_err(SetupError::io(&plugins))?;
        std::fs::copy(assembly, &target).map_err(SetupError::io(assembly))?;

        let symbols = assembly.with_extension("pdb");

        if symbols.is_file() {
            std::fs::copy(&symbols, plugins.join(format!("{name}.pdb")))
                .map_err(SetupError::io(&symbols))?;
        }

        Ok(target)
    }

    /// Removes both builds of the hook.
    pub fn remove(&self) -> SetupResult<()> {
        let plugins = self.game.plugins_folder();

        for major in BepInExMajor::ALL {
            for extension in Self::EXTENSIONS {
                let path =
                    plugins.join(format!("{}.{extension}", MessageHook::assembly_name(major)));

                match std::fs::remove_file(&path) {
                    Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                        return Err(SetupError::io(&path)(error));
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    /// The names of the assemblies in `BepInEx/plugins` and its folders, sorted.
    #[must_use]
    pub fn assemblies(&self) -> Vec<String> {
        let mut names = Vec::new();
        let mut folders = vec![self.game.plugins_folder()];

        while let Some(folder) = folders.pop() {
            let Ok(entries) = std::fs::read_dir(&folder) else {
                continue;
            };

            for path in entries.flatten().map(|entry| entry.path()) {
                if path.is_dir() {
                    folders.push(path);
                } else if path.extension().is_some_and(|extension| extension == "dll")
                    && let Some(stem) = path.file_stem()
                {
                    names.push(stem.to_string_lossy().into_owned());
                }
            }
        }

        names.sort();

        names
    }

    /// The name of the hook build in `BepInEx/plugins`, if any.
    #[must_use]
    pub fn current_build(&self) -> Option<String> {
        let builds = BepInExMajor::ALL.map(MessageHook::assembly_name);

        self.assemblies()
            .into_iter()
            .find(|name| builds.contains(&name.as_str()))
    }
}
