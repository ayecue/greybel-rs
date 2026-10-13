mod content;

use content::InstallerContent;

use crate::error::{BuildError, Result};
use crate::output::BuildOutput;
use crate::scripts::{AUTO_COMPILE, GreyScriptText, HEADER};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallerScript {
    pub name: String,
    pub code: String,
}

#[derive(Debug, Clone, Copy)]
pub struct Installer<'a> {
    output: &'a BuildOutput,
    max_chars: usize,
}

impl<'a> Installer<'a> {
    pub const DEFAULT_MAX_CHARS: usize = 160_000;
    const SELF_DELETE: &str = "d";

    #[must_use]
    pub const fn new(output: &'a BuildOutput, max_chars: usize) -> Self {
        Self { output, max_chars }
    }

    pub fn scripts(&self) -> Result<Vec<InstallerScript>> {
        let header = self.header();
        let mut chunks: Vec<String> = Vec::new();
        let mut current = header.clone();

        for file in &self.output.files {
            let path = GreyScriptText::quote(&self.output.ingame_path(file));
            let content = InstallerContent::new(&file.code);
            let mut position = 0;
            let mut new = true;

            loop {
                let prefix = format!("m({path},\"");
                let suffix = format!("\",{});", u8::from(new));
                let overhead = CharCount::of(&prefix)
                    + CharCount::of(&suffix)
                    + CharCount::of(Self::SELF_DELETE);
                let available_chars = self.max_chars.saturating_sub(CharCount::of(&current));
                let end = if available_chars > overhead {
                    content.cut(position, available_chars - overhead)
                } else {
                    position
                };

                if end == position && position < content.len() || available_chars <= overhead {
                    if current == header {
                        return Err(BuildError::MaxCharsTooSmall {
                            max_chars: self.max_chars,
                            required_chars: CharCount::of(&header) + overhead + 1,
                        });
                    }

                    chunks.push(current + Self::SELF_DELETE);
                    current = header.clone();
                    continue;
                }

                current.push_str(&prefix);
                current.push_str(content.slice(position..end));
                current.push_str(&suffix);
                position = end;
                new = false;
                if position == content.len() {
                    break;
                }

                chunks.push(current + Self::SELF_DELETE);
                current = header.clone();
            }
        }

        let footer = self.footer();

        if CharCount::of(&current) + CharCount::of(&footer) > self.max_chars {
            chunks.push(current + Self::SELF_DELETE);
            current = header + &footer;
        } else {
            current.push_str(&footer);
        }

        chunks.push(current);

        Ok(chunks
            .into_iter()
            .enumerate()
            .map(|(index, code)| InstallerScript {
                name: format!("installer{index}.src"),
                code,
            })
            .collect())
    }

    fn header(&self) -> String {
        let destination = &self.output.destination;
        let auto_compile = if self.output.auto_compile.is_some() {
            "1"
        } else {
            "0"
        };

        HEADER.render(&[
            (
                "DESTINATION",
                &GreyScriptText::quote(&destination.directory),
            ),
            (
                "RESOURCE_DESTINATION",
                &GreyScriptText::quote(&destination.resources),
            ),
            ("AUTO_COMPILE", auto_compile),
        ]) + ";"
    }

    fn footer(&self) -> String {
        let Some(auto_compile) = self.output.auto_compile else {
            return format!("{};", Self::SELF_DELETE);
        };
        let roots: Vec<String> = self
            .output
            .entries
            .iter()
            .map(|path| format!("/{path}"))
            .collect();
        let code = AUTO_COMPILE.render(&[
            (
                "ROOT_FILE_PATHS",
                &GreyScriptText::list(roots.iter().map(String::as_str)),
            ),
            (
                "TEMPORARY",
                &GreyScriptText::quote(&self.output.destination.temporary_folder_name),
            ),
            (
                "ALLOW_IMPORT",
                if auto_compile.allow_import { "1" } else { "0" },
            ),
        ]);

        format!("{};{code};", Self::SELF_DELETE)
    }
}

pub(crate) struct CharCount;

impl CharCount {
    pub(crate) fn of(text: &str) -> usize {
        text.encode_utf16().count()
    }
}
