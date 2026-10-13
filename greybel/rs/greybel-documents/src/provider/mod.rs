mod join;
mod request;
mod resolution;

use std::future::Future;
use std::io;

pub use request::{ResourceRequest, ResourceResponse};
pub use resolution::PathResolution;
use vscode_uri::Uri;

pub trait ResourceProvider {
    fn resolve(&self, target: &str) -> Uri;
    fn relative_to(&self, source: &Uri, target: &str) -> Uri;
    fn read(&self, uri: &Uri) -> io::Result<Option<String>>;
}

pub trait AsyncResourceProvider {
    fn resolve(&self, target: &str) -> impl Future<Output = io::Result<Uri>>;
    fn relative_to(&self, source: &Uri, target: &str) -> impl Future<Output = io::Result<Uri>>;
    fn read(&self, uri: &Uri) -> impl Future<Output = io::Result<Option<String>>>;
}
