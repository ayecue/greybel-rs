//! Temporary folders, fake games, zip archives and a download server.

pub mod download;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use greyhack_setup::game::GameLayout;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

/// A new folder in the system's temporary folder, removed when dropped.
pub struct TemporaryFolder {
    path: PathBuf,
}

impl TemporaryFolder {
    pub fn new() -> Self {
        static COUNT: AtomicUsize = AtomicUsize::new(0);

        let path = std::env::temp_dir().join(format!(
            "greyhack-setup-{}-{}",
            std::process::id(),
            COUNT.fetch_add(1, Ordering::Relaxed)
        ));

        std::fs::create_dir_all(&path).expect("a temporary folder");

        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn join(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }

    /// A macOS game in this folder: `Grey Hack.app` with its `Managed` folder.
    pub fn game(&self) -> GameLayout {
        std::fs::create_dir_all(self.join("Grey Hack.app/Contents/Resources/Data/Managed"))
            .expect("the game's folders");

        GameLayout::from_folder(&self.path).expect("the game")
    }

    /// Writes `text` to `path` in this folder.
    pub fn write(&self, path: &str, text: &str) -> PathBuf {
        let full = self.join(path);

        std::fs::create_dir_all(full.parent().expect("a folder")).expect("the folder");
        std::fs::write(&full, text).expect("the file");

        full
    }

    /// A zip file `name` in this folder with the given files and contents.
    pub fn archive(&self, name: &str, files: &[(&str, &str)]) -> PathBuf {
        let path = self.join(name);
        let mut zip = ZipWriter::new(std::fs::File::create(&path).expect("the archive"));

        for (file, text) in files {
            zip.start_file(*file, SimpleFileOptions::default())
                .expect("an entry");
            zip.write_all(text.as_bytes()).expect("its content");
        }

        zip.finish().expect("the archive");

        path
    }
}

impl Drop for TemporaryFolder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
