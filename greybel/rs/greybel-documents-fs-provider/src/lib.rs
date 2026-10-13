use std::collections::BTreeMap;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use globset::{GlobBuilder, GlobMatcher};
use greybel_documents::{AsyncResourceProvider, PathResolution, ResourceProvider, Uri};

#[derive(Debug, Clone, Default)]
pub struct FsProvider {
    resolution: PathResolution,
    memory: Arc<BTreeMap<Uri, String>>,
}

impl FsProvider {
    #[must_use]
    pub fn new(resolution: PathResolution) -> Self {
        Self {
            resolution,
            memory: Arc::default(),
        }
    }

    #[must_use]
    pub fn with_memory(mut self, files: Arc<BTreeMap<Uri, String>>) -> Self {
        self.memory = files;

        self
    }

    #[must_use]
    pub fn resolution(&self) -> &PathResolution {
        &self.resolution
    }

    #[must_use]
    pub fn uri(path: &Path) -> Uri {
        Uri::file(&Self::absolute(path).to_string_lossy())
    }

    #[must_use]
    pub fn path(uri: &Uri) -> Option<PathBuf> {
        (uri.scheme() == Uri::FILE).then(|| PathBuf::from(uri.fs_path()))
    }

    #[must_use]
    pub fn exists(uri: &Uri) -> bool {
        Self::path(uri).is_some_and(|path| path.exists())
    }

    /// Returns the text at `uri`, or `None` if no file exists there.
    #[must_use]
    pub fn text(uri: &Uri) -> Option<String> {
        Self::path(uri).and_then(|path| std::fs::read_to_string(path).ok())
    }

    /// Returns a display path for `uri`: relative to the current folder when possible, absolute for other files,
    /// or the original URI for non-file documents.
    #[must_use]
    pub fn display_path(uri: &Uri) -> String {
        let Some(path) = Self::path(uri) else {
            return uri.to_string();
        };
        let relative = std::env::current_dir()
            .ok()
            .and_then(|folder| path.strip_prefix(folder).ok().map(Path::to_path_buf));

        relative.unwrap_or(path).display().to_string()
    }

    /// Returns `path` for a file, all contained files for a folder, or no paths otherwise.
    #[must_use]
    pub fn files(path: &Path) -> Vec<Uri> {
        if path.is_dir() {
            Self::find_in(path, "**/*")
        } else if path.is_file() {
            vec![Self::uri(path)]
        } else {
            Vec::new()
        }
    }

    /// Returns sorted files in `folder` that match the `include` glob.
    #[must_use]
    pub fn find_in(folder: &Path, include: &str) -> Vec<Uri> {
        let resolution =
            PathResolution::new(Vec::<String>::new()).with_folders([Self::uri(folder)]);

        Self::new(resolution).find(include, None)
    }

    #[must_use]
    pub fn find(&self, include: &str, exclude: Option<&str>) -> Vec<Uri> {
        let Some(include) = Self::matcher(include) else {
            return Vec::new();
        };
        let exclude = exclude.and_then(Self::matcher);
        let mut files = Vec::new();

        for folder in self.resolution.folders().iter().filter_map(Self::path) {
            Self::walk(&folder, &folder, &include, exclude.as_ref(), &mut files);
        }

        files.sort();
        files.dedup();

        files
    }

    fn matcher(pattern: &str) -> Option<GlobMatcher> {
        GlobBuilder::new(pattern)
            .literal_separator(true)
            .build()
            .ok()
            .map(|glob| glob.compile_matcher())
    }

    fn walk(
        folder: &Path,
        directory: &Path,
        include: &GlobMatcher,
        exclude: Option<&GlobMatcher>,
        files: &mut Vec<Uri>,
    ) {
        let Ok(entries) = std::fs::read_dir(directory) else {
            return;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(relative) = path.strip_prefix(folder) else {
                continue;
            };
            let relative = relative.to_string_lossy().replace('\\', "/");
            let is_excluded = |path: &str| exclude.is_some_and(|exclude| exclude.is_match(path));

            if path.is_dir() {
                let link = entry.file_type().is_ok_and(|kind| kind.is_symlink());

                if !link && !is_excluded(&relative) && !is_excluded(&format!("{relative}/")) {
                    Self::walk(folder, &path, include, exclude, files);
                }
            } else if include.is_match(&relative) && !is_excluded(&relative) {
                files.push(Self::uri(&path));
            }
        }
    }

    fn absolute(path: &Path) -> PathBuf {
        let full_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir().unwrap_or_default().join(path)
        };
        let mut clean_path = PathBuf::new();

        for component in full_path.components() {
            match component {
                Component::CurDir => {}
                Component::ParentDir => {
                    clean_path.pop();
                }
                other => clean_path.push(other),
            }
        }

        clean_path
    }
}

impl ResourceProvider for FsProvider {
    fn resolve(&self, target: &str) -> Uri {
        if Uri::has_scheme(target)
            && let Ok(uri) = Uri::parse(target)
        {
            return uri;
        }

        Self::uri(Path::new(target))
    }

    fn relative_to(&self, source: &Uri, target: &str) -> Uri {
        self.resolution.relative_to(source, target, |uri| {
            self.memory.contains_key(uri) || Self::exists(uri)
        })
    }

    fn read(&self, uri: &Uri) -> io::Result<Option<String>> {
        if let Some(content) = self.memory.get(uri) {
            return Ok(Some(content.clone()));
        }

        let Some(path) = Self::path(uri) else {
            return Ok(None);
        };

        match std::fs::read_to_string(path) {
            Ok(content) => Ok(Some(content)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }
}

impl AsyncResourceProvider for FsProvider {
    async fn resolve(&self, target: &str) -> io::Result<Uri> {
        Ok(ResourceProvider::resolve(self, target))
    }

    async fn relative_to(&self, source: &Uri, target: &str) -> io::Result<Uri> {
        Ok(ResourceProvider::relative_to(self, source, target))
    }

    async fn read(&self, uri: &Uri) -> io::Result<Option<String>> {
        ResourceProvider::read(self, uri)
    }
}
