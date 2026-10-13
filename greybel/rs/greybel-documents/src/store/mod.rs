mod files;
mod inputs;
mod parse_result;
mod queries;

pub(crate) use files::SourceFileTable;
pub use inputs::{Project, SourceFile};
pub use parse_result::ParseResult;

#[salsa::db]
pub trait Db: salsa::Database {}

#[salsa::db]
#[derive(Default, Clone)]
pub struct DocumentDatabase {
    storage: salsa::Storage<Self>,
}

#[salsa::db]
impl salsa::Database for DocumentDatabase {}

#[salsa::db]
impl Db for DocumentDatabase {}
