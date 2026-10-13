use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_FOLDER: AtomicUsize = AtomicUsize::new(0);

/// Temporary directory containing test files, recursively deleted on drop.
pub struct TempFolder(PathBuf);

impl TempFolder {
    /// Creates a unique `name` temporary directory and writes supplied files at their relative paths.
    pub fn new(
        name: &str,
        files: impl IntoIterator<Item = (impl AsRef<Path>, impl AsRef<[u8]>)>,
    ) -> Self {
        let path = std::env::temp_dir().join(format!(
            "{name}-{}-{}",
            std::process::id(),
            NEXT_FOLDER.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&path);

        std::fs::create_dir_all(&path).expect("the temporary folder is created");
        for (file, content) in files {
            let file = path.join(file);

            if let Some(folder) = file.parent() {
                std::fs::create_dir_all(folder).expect("the file's folder is created");
            }

            std::fs::write(file, content).expect("the file is written");
        }

        Self(path)
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempFolder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
