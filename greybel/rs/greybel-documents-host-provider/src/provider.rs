use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::io;

use duplex_core::{Channel, Driver, Error};
use greybel_documents::{AsyncResourceProvider, PathResolution, Uri};

use crate::protocol::{FindFiles, ReadFile, Server};

pub struct HostProvider<D: Driver> {
    channel: Channel<Server, D>,
    resolution: PathResolution,
    client_uris: RefCell<HashSet<Uri>>,
    prefetch_cache: RefCell<HashMap<Uri, Option<String>>>,
}

impl<D: Driver> HostProvider<D> {
    #[must_use]
    pub fn new(channel: Channel<Server, D>, resolution: PathResolution) -> Self {
        Self {
            channel,
            resolution,
            client_uris: RefCell::default(),
            prefetch_cache: RefCell::default(),
        }
    }

    #[must_use]
    pub fn with_folders(mut self, folders: impl IntoIterator<Item = Uri>) -> Self {
        self.resolution = self.resolution.with_folders(folders);

        self
    }

    #[must_use]
    pub fn channel(&self) -> &Channel<Server, D> {
        &self.channel
    }

    #[must_use]
    pub fn resolution(&self) -> &PathResolution {
        &self.resolution
    }

    pub async fn index(&self, include: &str, exclude: Option<&str>) -> io::Result<usize> {
        let request = FindFiles {
            include: include.into(),
            exclude: exclude.map(Into::into),
        };
        let uris = self
            .channel
            .send(&request)
            .map_err(Self::failure)?
            .await
            .map_err(Self::failure)?;
        let client_uris: HashSet<Uri> =
            uris.iter().filter_map(|uri| Uri::parse(uri).ok()).collect();
        let count = client_uris.len();

        *self.client_uris.borrow_mut() = client_uris;

        Ok(count)
    }

    pub fn record_creation(&self, uri: &Uri) {
        self.client_uris.borrow_mut().insert(uri.clone());
        self.prefetch_cache.borrow_mut().remove(uri);
    }

    pub fn invalidate_content(&self, uri: &Uri) {
        self.prefetch_cache.borrow_mut().remove(uri);
    }

    pub fn record_deletion(&self, uri: &Uri) {
        self.client_uris.borrow_mut().remove(uri);
        self.prefetch_cache.borrow_mut().remove(uri);
    }

    async fn exists(&self, uri: Uri) -> io::Result<bool> {
        if self.client_uris.borrow().contains(&uri) {
            return Ok(true);
        }

        if let Some(content) = self.prefetch_cache.borrow().get(&uri) {
            return Ok(content.is_some());
        }

        let content = self.read_file(&uri).await?;
        let found = content.is_some();

        self.prefetch_cache.borrow_mut().insert(uri, content);

        Ok(found)
    }

    async fn read_file(&self, uri: &Uri) -> io::Result<Option<String>> {
        let request = ReadFile {
            uri: uri.to_string().into(),
        };

        self.channel
            .send(&request)
            .map_err(Self::failure)?
            .await
            .map_err(Self::failure)
    }

    fn failure(error: Error) -> io::Error {
        io::Error::other(error)
    }
}

impl<D: Driver> AsyncResourceProvider for HostProvider<D> {
    async fn resolve(&self, target: &str) -> io::Result<Uri> {
        Uri::from_path_or_uri(target)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))
    }

    async fn relative_to(&self, source: &Uri, target: &str) -> io::Result<Uri> {
        self.resolution
            .relative_to_async(source, target, |uri| self.exists(uri))
            .await
    }

    async fn read(&self, uri: &Uri) -> io::Result<Option<String>> {
        let cached_content = self.prefetch_cache.borrow_mut().remove(uri);

        match cached_content {
            Some(content) => Ok(content),
            None => self.read_file(uri).await,
        }
    }
}
