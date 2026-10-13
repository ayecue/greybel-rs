use std::future::Future;

use vscode_uri::{PosixPath, Uri, UriChange};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathResolution {
    extensions: Vec<String>,
    folders: Vec<Uri>,
}

impl Default for PathResolution {
    fn default() -> Self {
        Self::new(["src"])
    }
}

impl PathResolution {
    #[must_use]
    pub fn new<E: Into<String>>(extensions: impl IntoIterator<Item = E>) -> Self {
        Self {
            extensions: extensions.into_iter().map(Into::into).collect(),
            folders: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_folders(mut self, folders: impl IntoIterator<Item = Uri>) -> Self {
        self.folders = folders.into_iter().collect();

        self
    }

    #[must_use]
    pub fn with_base_folder(self, root: Option<Uri>, entry: &Uri) -> Self {
        self.with_folders(root.or_else(|| entry.dirname().ok()))
    }

    #[must_use]
    pub fn extensions(&self) -> &[String] {
        &self.extensions
    }

    #[must_use]
    pub fn folders(&self) -> &[Uri] {
        &self.folders
    }

    #[must_use]
    pub fn folder_of(&self, uri: &Uri) -> Option<&Uri> {
        self.folders
            .iter()
            .filter(|folder| Self::contains(folder, uri))
            .max_by_key(|folder| folder.path().len())
    }

    #[must_use]
    pub fn contains(folder: &Uri, uri: &Uri) -> bool {
        Self::path_in(folder, uri).is_some()
    }

    #[must_use]
    pub fn path_in<'u>(folder: &Uri, uri: &'u Uri) -> Option<&'u str> {
        if folder.scheme() != uri.scheme() || folder.authority() != uri.authority() {
            return None;
        }

        match uri
            .path()
            .strip_prefix(folder.path().trim_end_matches('/'))?
        {
            "" => Some(""),
            remainder => remainder.strip_prefix('/'),
        }
    }

    #[must_use]
    pub fn join(&self, source: &Uri, target: &str) -> Uri {
        let base = if PosixPath::is_absolute(target) {
            self.folder_of(source)
                .cloned()
                .unwrap_or_else(|| Self::with_path(source, "/".to_owned()))
        } else {
            source.dirname().unwrap_or_else(|_| source.clone())
        };

        base.join_path(&[target])
            .unwrap_or_else(|_| Self::with_path(&base, PosixPath::join(&[base.path(), target])))
    }

    #[must_use]
    pub fn relative_to(&self, source: &Uri, target: &str, exists: impl Fn(&Uri) -> bool) -> Uri {
        self.resolve(&self.join(source, target), exists)
    }

    pub async fn relative_to_async<F, E>(
        &self,
        source: &Uri,
        target: &str,
        exists: impl Fn(Uri) -> F,
    ) -> Result<Uri, E>
    where
        F: Future<Output = Result<bool, E>>,
    {
        self.resolve_async(&self.join(source, target), exists).await
    }

    #[must_use]
    pub fn resolve(&self, uri: &Uri, exists: impl Fn(&Uri) -> bool) -> Uri {
        if exists(uri) || self.has_extension(uri) && !self.candidates(uri).any(|c| exists(&c)) {
            return uri.clone();
        }

        self.candidates(uri)
            .find(|candidate| exists(candidate))
            .unwrap_or_else(|| self.fallback(uri))
    }

    pub async fn resolve_async<F, E>(&self, uri: &Uri, exists: impl Fn(Uri) -> F) -> Result<Uri, E>
    where
        F: Future<Output = Result<bool, E>>,
    {
        if exists(uri.clone()).await? {
            return Ok(uri.clone());
        }

        for candidate in self.candidates(uri) {
            if exists(candidate.clone()).await? {
                return Ok(candidate);
            }
        }

        Ok(if self.has_extension(uri) {
            uri.clone()
        } else {
            self.fallback(uri)
        })
    }

    fn candidates<'a>(&'a self, uri: &'a Uri) -> impl Iterator<Item = Uri> + 'a {
        self.extensions
            .iter()
            .map(move |extension| Self::with_path(uri, format!("{}.{extension}", uri.path())))
    }

    fn fallback(&self, uri: &Uri) -> Uri {
        self.candidates(uri).next().unwrap_or_else(|| uri.clone())
    }

    fn has_extension(&self, uri: &Uri) -> bool {
        self.extensions.iter().any(|extension| {
            uri.path()
                .strip_suffix(extension.as_str())
                .is_some_and(|stem| stem.ends_with('.'))
        })
    }

    fn with_path(uri: &Uri, path: String) -> Uri {
        uri.with(UriChange {
            path: Some(path),
            ..UriChange::default()
        })
        .unwrap_or_else(|_| uri.clone())
    }
}
