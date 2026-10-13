use std::collections::BTreeMap;
use std::io;

use greybel_documents::{AsyncResourceProvider, PathResolution, ResourceProvider, Uri};
use vscode_uri::PosixPath;

#[derive(Debug, Clone, Default)]
pub struct MemoryProvider {
    files: BTreeMap<String, String>,
    resolution: PathResolution,
}

impl MemoryProvider {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_resolution(resolution: PathResolution) -> Self {
        Self {
            files: BTreeMap::new(),
            resolution,
        }
    }

    #[must_use]
    pub fn resolution(&self) -> &PathResolution {
        &self.resolution
    }

    #[must_use]
    pub fn uri(path: &str) -> Uri {
        Uri::memory(&Self::normalize(path))
    }

    pub fn insert(&mut self, path: &str, content: impl Into<String>) -> Option<String> {
        self.files.insert(Self::normalize(path), content.into())
    }

    pub fn remove(&mut self, path: &str) -> Option<String> {
        self.files.remove(&Self::normalize(path))
    }

    #[must_use]
    pub fn contains(&self, path: &str) -> bool {
        self.files.contains_key(&Self::normalize(path))
    }

    #[must_use]
    pub fn content(&self, path: &str) -> Option<&str> {
        self.files.get(&Self::normalize(path)).map(String::as_str)
    }

    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.files.keys().map(String::as_str)
    }

    pub fn uris(&self) -> impl Iterator<Item = Uri> + '_ {
        self.paths().map(Uri::memory)
    }

    fn uri_content(&self, uri: &Uri) -> Option<&str> {
        if uri.scheme() != Uri::MEMORY || !uri.authority().is_empty() {
            return None;
        }

        self.files.get(uri.path()).map(String::as_str)
    }

    fn normalize(path: &str) -> String {
        PosixPath::resolve(&["/", path])
    }
}

impl<P: AsRef<str>, C: Into<String>> FromIterator<(P, C)> for MemoryProvider {
    fn from_iter<I: IntoIterator<Item = (P, C)>>(files: I) -> Self {
        let mut provider = Self::new();

        for (path, content) in files {
            provider.insert(path.as_ref(), content);
        }

        provider
    }
}

impl ResourceProvider for MemoryProvider {
    fn resolve(&self, target: &str) -> Uri {
        if Uri::has_scheme(target)
            && let Ok(uri) = Uri::parse(target)
        {
            return uri;
        }

        Self::uri(target)
    }

    fn relative_to(&self, source: &Uri, target: &str) -> Uri {
        self.resolution
            .relative_to(source, target, |uri| self.uri_content(uri).is_some())
    }

    fn read(&self, uri: &Uri) -> io::Result<Option<String>> {
        Ok(self.uri_content(uri).map(str::to_owned))
    }
}

impl AsyncResourceProvider for MemoryProvider {
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
