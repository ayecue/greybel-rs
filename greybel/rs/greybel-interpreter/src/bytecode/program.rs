use std::fmt;
use std::rc::Rc;

use greybel_core::{ColumnSettings, Dialect, LineIndex};

use super::{Code, CodeId, Op, Source};

#[derive(Debug, Clone)]
pub struct File {
    pub path: Rc<str>,
    pub uri: Rc<str>,
    pub lines: LineIndex,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Position in a program source file. Diagnostics use `path`; `uri` identifies the document.
pub struct Location {
    pub path: Rc<str>,
    pub uri: Rc<str>,
    pub line: u32,
    pub character: u32,
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.path, self.line, self.character)
    }
}

#[derive(Debug, Clone)]
pub struct Program {
    pub(crate) codes: Vec<Code>,
    pub(crate) entry: CodeId,
    pub(crate) files: Vec<File>,
    pub(crate) import_count: u32,
    pub(crate) warnings: Vec<String>,
    pub(crate) dialect: Dialect,
}

impl Program {
    #[must_use]
    pub fn entry(&self) -> CodeId {
        self.entry
    }

    #[must_use]
    pub fn code_count(&self) -> usize {
        self.codes.len()
    }

    #[must_use]
    pub fn code(&self, id: CodeId) -> &Code {
        &self.codes[id.0 as usize]
    }

    #[must_use]
    pub fn location(&self, source: Source) -> Location {
        let file = &self.files[source.file.0 as usize];
        let position = file
            .lines
            .position(source.span.start, ColumnSettings::default());

        Location {
            path: Rc::clone(&file.path),
            uri: Rc::clone(&file.uri),
            line: position.line,
            character: position.character,
        }
    }

    #[must_use]
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    pub(crate) fn append(&mut self, other: Program) -> CodeId {
        let code_offset = u32::try_from(self.codes.len()).expect("fewer than 2^32 code blocks");
        let file_offset = u32::try_from(self.files.len()).expect("fewer than 2^32 files");
        let slot_offset = self.import_count;
        let entry = CodeId(other.entry.0 + code_offset);

        for mut code in other.codes {
            for op in &mut code.ops {
                match op {
                    Op::Function { code, .. } => code.0 += code_offset,
                    Op::Import { slot, code } => {
                        slot.0 += slot_offset;
                        code.0 += code_offset;
                    }
                    Op::Export(slot) => slot.0 += slot_offset,
                    _ => {}
                }
            }

            for source in code.sources.iter_mut().flatten() {
                source.file.0 += file_offset;
            }

            self.codes.push(code);
        }

        self.files.extend(other.files);
        self.import_count += other.import_count;
        self.warnings.extend(other.warnings);

        entry
    }
}
