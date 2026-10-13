use std::sync::Arc;

use rustc_hash::FxHashMap;
use salsa::Setter;
use vscode_uri::Uri;

use super::{Db, Project, SourceFile};

#[derive(Debug, Default)]
pub(crate) struct SourceFileTable {
    files: FxHashMap<Uri, SourceFile>,
}

impl SourceFileTable {
    pub(crate) fn get(&self, uri: &Uri) -> Option<SourceFile> {
        self.files.get(uri).copied()
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = SourceFile> + '_ {
        self.files.values().copied()
    }

    pub(crate) fn store<DB: Db>(
        &mut self,
        db: &mut DB,
        project: Project,
        uri: &Uri,
        text: Option<Arc<str>>,
    ) -> SourceFile {
        match self.files.get(uri) {
            Some(&file) => {
                if *file.text(db) != text {
                    let version = file.version(db).wrapping_add(1);

                    file.set_text(db).to(text);
                    file.set_version(db).to(version);
                }

                file
            }
            None => {
                let file = SourceFile::new(db, project, uri.clone(), 0, text, FxHashMap::default());

                self.files.insert(uri.clone(), file);

                file
            }
        }
    }
}

impl SourceFile {
    pub(crate) fn link<DB: Db>(self, db: &mut DB, target: &str, file: SourceFile) {
        if self.links(db).get(target) == Some(&file) {
            return;
        }

        let mut links = self.links(db).clone();

        links.insert(target.to_owned(), file);
        self.set_links(db).to(links);
    }
}
