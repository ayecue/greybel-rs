use std::sync::Arc;

use rustc_hash::FxHashMap;
use vscode_uri::Uri;

use crate::options::DocumentOptions;

#[salsa::input(debug)]
pub struct Project {
    #[returns(ref)]
    pub options: DocumentOptions,
}

#[salsa::input(debug)]
pub struct SourceFile {
    pub project: Project,
    #[returns(ref)]
    pub uri: Uri,
    #[returns(copy)]
    pub version: u32,
    #[returns(ref)]
    pub text: Option<Arc<str>>,
    #[returns(ref)]
    pub links: FxHashMap<String, SourceFile>,
}
