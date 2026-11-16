//! Parses `.csproj` and `Directory.Build.props` XML elements, attributes, and values containing
//! `$(Name)` property expressions. Rewrites values in place without changing the remaining file text.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use roxmltree::{Document, Node};

use crate::language::{Dependency, Scope, Target};
use crate::paths::PathExt;

pub struct MsBuildFile<'input> {
    text: &'input str,
    document: Document<'input>,
}

impl<'input> MsBuildFile<'input> {
    /// File MSBuild imports into every project below its folder.
    pub const DIRECTORY_PROPS: &str = "Directory.Build.props";

    /// Checks whether `path` identifies an MSBuild project file.
    pub fn is_project(path: &Path) -> bool {
        path.extension()
            .is_some_and(|extension| extension == "csproj")
    }

    /// `Directory.Build.props` MSBuild finds for a project in `directory` (relative to `root`): the closest one
    /// in it or above.
    pub fn nearest_props(root: &Path, directory: &Path) -> Option<PathBuf> {
        directory
            .ancestors()
            .map(|folder| folder.join(Self::DIRECTORY_PROPS))
            .find(|candidate| root.join(candidate).is_file())
    }

    /// File at `path` (relative to `root`), with its text.
    pub fn read(root: &Path, path: &Path) -> Result<String> {
        std::fs::read_to_string(root.join(path))
            .with_context(|| format!("reading {}", path.display()))
    }

    pub fn parse(text: &'input str, path: &Path) -> Result<Self> {
        let document =
            Document::parse(text).with_context(|| format!("parsing {}", path.display()))?;

        Ok(MsBuildFile { text, document })
    }

    /// Trimmed text of every element named `name` (`Version`, `OutputType`).
    pub fn values<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        self.elements(name)
            .filter_map(|node| node.text())
            .map(str::trim)
    }

    /// Value of `attribute` of every element named `element` (`ProjectReference`'s `Include`).
    pub fn attributes<'a>(
        &'a self,
        element: &'a str,
        attribute: &'a str,
    ) -> impl Iterator<Item = &'a str> + 'a {
        self.elements(element)
            .filter_map(move |node| node.attribute(attribute))
    }

    /// `PropertyGroup` elements and trimmed text for properties defined by the file.
    pub fn properties(&self) -> impl Iterator<Item = (&str, &str)> + '_ {
        self.elements("PropertyGroup")
            .flat_map(|group| group.children().filter(Node::is_element))
            .filter_map(|property| Some((property.tag_name().name(), property.text()?.trim())))
    }

    /// Attribute values and text where MSBuild evaluates property expressions.
    pub fn expressions(&self) -> impl Iterator<Item = &str> + '_ {
        self.document.descendants().flat_map(|node| {
            let text = node.is_text().then(|| node.text()).flatten();

            node.attributes()
                .map(|attribute| attribute.value())
                .chain(text)
        })
    }

    /// File with the text of every element named in `names` whose text is `from` replaced by `to`, if one is.
    pub fn replace_values(&self, names: &[&str], from: &str, to: &str) -> Option<String> {
        let mut spans = Vec::new();

        for name in names {
            for node in self.elements(name) {
                let Some(text) = node.first_child().filter(Node::is_text) else {
                    continue;
                };
                let range = text.range();
                let value = &self.text[range.clone()];

                if value.trim() == from {
                    let start = range.start + (value.len() - value.trim_start().len());

                    spans.push(start..start + from.len());
                }
            }
        }

        if spans.is_empty() {
            return None;
        }

        spans.sort_by_key(|span| span.start);

        let mut output = String::with_capacity(self.text.len());
        let mut position = 0;

        for span in spans {
            output.push_str(&self.text[position..span.start]);
            output.push_str(to);
            position = span.end;
        }

        output.push_str(&self.text[position..]);

        Some(output)
    }

    /// Projects it references by a plain path (those through a property are references, read apart); the
    /// file is in `directory`.
    pub fn project_references(&self, directory: &Path, scope: Scope) -> Vec<Dependency> {
        self.attributes("ProjectReference", "Include")
            .filter(|include| !include.contains("$("))
            .map(|include| {
                let path = directory.join(include.replace('\\', "/"));

                Dependency {
                    target: Target::Path(path.parent().unwrap_or(Path::new("")).normalize()),
                    scope,
                }
            })
            .collect()
    }

    fn elements<'a>(&'a self, name: &'a str) -> impl Iterator<Item = Node<'a, 'input>> + 'a {
        self.document
            .descendants()
            .filter(move |node| node.is_element() && node.tag_name().name() == name)
    }
}
