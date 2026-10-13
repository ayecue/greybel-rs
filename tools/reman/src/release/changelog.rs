//! Project's `CHANGELOG.md`: a section per release, newest first, its notes grouped by how far each moved the
//! project; a change made in a dependency is prefixed with the project it was made in.

use std::ops::Range;

use jiff::civil::Date;
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};

use crate::changes::Level;
use crate::release::plan::Release;
use crate::repository::Repository;

/// Text of a changelog.
pub struct Changelog<'a> {
    text: &'a str,
}

/// Release's heading in a changelog.
struct Heading {
    /// Heading's bytes.
    span: Range<usize>,
    /// Version extracted from the first word (`1.2.0` in `1.2.0 · 2026-11-07`).
    version: String,
}

impl<'a> Changelog<'a> {
    pub fn new(text: &'a str) -> Self {
        Changelog { text }
    }

    /// New changelog titled `title`, without releases.
    pub fn empty(title: &str) -> String {
        format!("# {title}\n")
    }

    /// Changelog with `section` above its newest release (at its end without one).
    pub fn with_section(&self, section: &str) -> String {
        let text = self.text;

        if let Some(newest) = self.releases().first() {
            let start = newest.span.start;

            return format!("{}{section}\n{}", &text[..start], &text[start..]);
        }

        let separator = if text.ends_with("\n\n") || text.is_empty() {
            ""
        } else if text.ends_with('\n') {
            "\n"
        } else {
            "\n\n"
        };

        format!("{text}{separator}{section}")
    }

    /// Section of `version`, without its heading.
    pub fn notes_of(&self, version: &str) -> Option<String> {
        let releases = self.releases();
        let index = releases
            .iter()
            .position(|release| release.version == version)?;
        let end = releases
            .get(index + 1)
            .map_or(self.text.len(), |next| next.span.start);

        Some(self.text[releases[index].span.end..end].trim().to_owned())
    }

    /// Level-two headings starting with a version (a digit), in order: the releases.
    fn releases(&self) -> Vec<Heading> {
        let mut headings = Vec::new();
        let mut open: Option<(Range<usize>, String)> = None;

        for (event, span) in Parser::new(self.text).into_offset_iter() {
            match event {
                Event::Start(Tag::Heading {
                    level: HeadingLevel::H2,
                    ..
                }) => open = Some((span, String::new())),
                Event::Text(text) | Event::Code(text) => {
                    if let Some((_, heading)) = &mut open {
                        heading.push_str(&text);
                    }
                }
                Event::End(TagEnd::Heading(HeadingLevel::H2)) => {
                    if let Some((span, text)) = open.take()
                        && let Some(version) = text.split_whitespace().next()
                        && version.starts_with(|character: char| character.is_ascii_digit())
                    {
                        headings.push(Heading {
                            span,
                            version: version.to_owned(),
                        });
                    }
                }
                _ => {}
            }
        }

        headings
    }
}

impl Release {
    /// Changelog section: `## <version> · <date>` followed by notes grouped by level.
    pub fn section(&self, repository: &Repository, date: Date) -> String {
        let mut text = format!("## {} · {date}\n", self.version);

        for level in [Level::Major, Level::Minor, Level::Patch] {
            let notes = self
                .notes
                .iter()
                .filter(|note| note.level.max(Level::Patch) == level)
                .collect::<Vec<_>>();

            if notes.is_empty() {
                continue;
            }

            text.push_str(&format!("\n### {}\n\n", level.heading()));

            for note in notes {
                let prefix = note
                    .through
                    .iter()
                    .map(|node| repository.display_name(*node))
                    .collect::<Vec<_>>();
                let mut lines = note.text.lines();
                let first = lines.next().unwrap_or_default();

                if prefix.is_empty() {
                    text.push_str(&format!("- {first}\n"));
                } else {
                    text.push_str(&format!("- {}: {first}\n", prefix.join(", ")));
                }

                for line in lines {
                    if line.trim().is_empty() {
                        text.push('\n');
                    } else {
                        text.push_str(&format!("  {line}\n"));
                    }
                }
            }
        }

        text
    }
}
