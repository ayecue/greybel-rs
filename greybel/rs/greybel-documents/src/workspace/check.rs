use rustc_hash::FxHashSet;

use crate::error::DocumentError;
use crate::store::{Db, SourceFile};

#[derive(Clone, Copy)]
struct Reference {
    origin: Option<SourceFile>,
    file: SourceFile,
    is_dependency: bool,
}

pub(super) struct Check<'db> {
    db: &'db dyn Db,
    read_files: FxHashSet<SourceFile>,
    parsed_files: FxHashSet<SourceFile>,
}

impl<'db> Check<'db> {
    pub(super) fn new(db: &'db dyn Db) -> Self {
        Self {
            db,
            read_files: FxHashSet::default(),
            parsed_files: FxHashSet::default(),
        }
    }

    pub(super) fn first_error(mut self, entry: SourceFile) -> Option<DocumentError> {
        let mut level = vec![Reference {
            origin: None,
            file: entry,
            is_dependency: true,
        }];

        while !level.is_empty() {
            let ready_files = match self.check_level(&level) {
                Ok(ready_files) => ready_files,
                Err(error) => return Some(error),
            };

            level = ready_files
                .into_iter()
                .flat_map(|origin| self.references(origin))
                .collect();
        }

        None
    }

    fn check_level(&mut self, level: &[Reference]) -> Result<Vec<SourceFile>, DocumentError> {
        let mut ready_files = Vec::new();
        let mut unread_files: Vec<Reference> = Vec::new();
        let needs_parse: FxHashSet<SourceFile> = level
            .iter()
            .filter(|reference| reference.is_dependency)
            .map(|reference| reference.file)
            .collect();

        for reference in level {
            if self.read_files.contains(&reference.file) {
                if reference.is_dependency && self.parse(reference.file)? {
                    ready_files.push(reference.file);
                }
            } else if !unread_files
                .iter()
                .any(|first| first.file == reference.file)
            {
                unread_files.push(*reference);
            }
        }

        for reference in unread_files {
            self.read_files.insert(reference.file);
            if reference.file.text(self.db).is_none() {
                return Err(self.missing_error(reference));
            }

            if needs_parse.contains(&reference.file) && self.parse(reference.file)? {
                ready_files.push(reference.file);
            }
        }

        Ok(ready_files)
    }

    fn parse(&mut self, file: SourceFile) -> Result<bool, DocumentError> {
        if !self.parsed_files.insert(file) {
            return Ok(false);
        }

        match file.parse(self.db).errors().first() {
            Some(error) => Err(DocumentError::Parse {
                target: file.uri(self.db).clone(),
                error: error.clone(),
            }),
            None => Ok(true),
        }
    }

    fn references(&self, origin: SourceFile) -> Vec<Reference> {
        let links = origin.links(self.db);

        origin
            .imports(self.db)
            .iter()
            .filter_map(|import| {
                Some(Reference {
                    origin: Some(origin),
                    file: *links.get(&import.target)?,
                    is_dependency: import.kind.is_dependency(),
                })
            })
            .collect()
    }

    fn missing_error(&self, reference: Reference) -> DocumentError {
        let target = reference.file.uri(self.db).clone();

        match reference.origin {
            None => DocumentError::MissingTarget { target },
            Some(origin) if reference.is_dependency => DocumentError::MissingDependency {
                target,
                origin: origin.uri(self.db).clone(),
            },
            Some(origin) => DocumentError::MissingInjection {
                target,
                origin: origin.uri(self.db).clone(),
            },
        }
    }
}
