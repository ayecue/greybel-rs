use super::{Uri, UriChange};
use crate::error::Result;
use crate::posix::PosixPath;

impl Uri {
    pub fn join_path(&self, paths: &[&str]) -> Result<Self> {
        let mut path_parts = vec![self.components.path.as_str()];

        path_parts.extend_from_slice(paths);

        self.with_path(PosixPath::join(&path_parts))
    }

    pub fn resolve_path(&self, paths: &[&str]) -> Result<Self> {
        let slash_added = !self.components.path.starts_with('/');
        let base = if slash_added {
            format!("/{}", self.components.path)
        } else {
            self.components.path.clone()
        };
        let mut segments = vec![base.as_str()];

        segments.extend_from_slice(paths);

        let mut resolved_path = PosixPath::resolve(&segments);

        if slash_added && resolved_path.starts_with('/') && self.components.authority.is_empty() {
            resolved_path.remove(0);
        }

        self.with_path(resolved_path)
    }

    pub fn dirname(&self) -> Result<Self> {
        if self.components.path.is_empty() || self.components.path == "/" {
            return Ok(self.clone());
        }

        let path = match PosixPath::dirname(&self.components.path) {
            "." => "",
            path => path,
        };

        self.with_path(path.to_owned())
    }

    #[must_use]
    pub fn basename(&self) -> &str {
        PosixPath::basename(&self.components.path)
    }

    #[must_use]
    pub fn extname(&self) -> &str {
        PosixPath::extname(&self.components.path)
    }

    fn with_path(&self, path: String) -> Result<Self> {
        self.with(UriChange {
            path: Some(path),
            ..UriChange::default()
        })
    }
}
