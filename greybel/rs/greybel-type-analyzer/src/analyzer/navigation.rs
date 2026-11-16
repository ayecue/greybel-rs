use greybel_core::{ColumnSettings, Position, Range};
use greybel_documents::{SourceFile, Uri};

use super::Analyzer;
use crate::analysis::Analysis;
use crate::ide::{Context, Location};
use crate::model::{DocumentIndex, Site};

impl<'w> Analyzer<'w> {
    #[must_use]
    pub fn definition(&self, uri: &Uri, position: Position) -> Vec<Location> {
        if let Some(location) = self.import_definition(uri, position) {
            return vec![location];
        }

        let Some((file, ast, offset)) = self.locate(uri, position) else {
            return Vec::new();
        };
        let mut locations = Vec::new();

        for (analysis, document) in self.analyses_of(file) {
            let context = Context::new(ast, analysis, document, offset);
            let slots = match context.referenced_slots() {
                Some((_, slots)) => slots.to_vec(),
                None => context.singleton_entry_slots(),
            };
            let references = analysis.references();
            let sites = slots
                .iter()
                .flat_map(|&slot| references.definition_sites(slot).iter().copied())
                .collect();

            locations.extend(self.locations(analysis, sites));
        }

        Self::sort_locations(locations)
    }

    /// The range a definition at `position` starts from: the import path or the name under the cursor.
    #[must_use]
    pub fn definition_origin(&self, uri: &Uri, position: Position) -> Option<Range> {
        if let Some(range) = self.import_path_range(uri, position) {
            return Some(range);
        }

        let (file, ast, offset) = self.locate(uri, position)?;

        self.analyses_of(file)
            .into_iter()
            .find_map(|(analysis, document)| {
                let (node, _) = Context::new(ast, analysis, document, offset).referenced_slots()?;

                Some(ast.lines().range(ast.span(node), ColumnSettings::default()))
            })
    }

    /// Where the types of the value at `position` are declared.
    #[must_use]
    pub fn type_definition(&self, uri: &Uri, position: Position) -> Vec<Location> {
        let Some((file, ast, offset)) = self.locate(uri, position) else {
            return Vec::new();
        };
        let db = self.workspace.db();
        let mut locations = Vec::new();

        for (analysis, document) in self.analyses_of(file) {
            for definition in Context::new(ast, analysis, document, offset).type_definitions() {
                let Some(&target) = analysis.documents().get(definition.document() as usize) else {
                    continue;
                };
                let Some(target_ast) = target.parse(db).ast() else {
                    continue;
                };
                let span = definition.span(target_ast);

                locations.push(Location {
                    uri: target.uri(db).clone(),
                    range: target_ast.lines().range(span, ColumnSettings::default()),
                });
            }
        }

        Self::sort_locations(locations)
    }

    #[must_use]
    pub fn references(&self, uri: &Uri, position: Position) -> Vec<Location> {
        let Some((file, _, offset)) = self.locate(uri, position) else {
            return Vec::new();
        };
        let db = self.workspace.db();
        let mut places = vec![(file, offset)];
        let mut analysis_list: Vec<&Analysis> = Vec::new();
        let mut locations = Vec::new();
        let mut next = 0;

        while let Some(&(file, offset)) = places.get(next) {
            next += 1;

            let Some(ast) = file.parse(db).ast() else {
                continue;
            };

            for (analysis, document) in self.analyses_of(file) {
                if analysis_list
                    .iter()
                    .any(|&previous| std::ptr::eq(previous, analysis))
                {
                    continue;
                }

                let Some((_, slots)) =
                    Context::new(ast, analysis, document, offset).referenced_slots()
                else {
                    continue;
                };

                analysis_list.push(analysis);

                let references = analysis.references();

                locations.extend(
                    self.locations(analysis, references.find_referring_sites(slots).collect()),
                );

                let definitions = slots
                    .iter()
                    .flat_map(|&slot| references.definition_sites(slot).iter().copied());

                for place in definitions.filter_map(|site| self.site_location(analysis, site)) {
                    if !places.contains(&place) {
                        places.push(place);
                    }
                }
            }
        }

        Self::sort_locations(locations)
    }

    pub(super) fn sort_locations(mut locations: Vec<Location>) -> Vec<Location> {
        locations.sort_by(|a, b| (&a.uri, a.range.start).cmp(&(&b.uri, b.range.start)));
        locations.dedup();

        locations
    }

    fn site_location(self, analysis: &Analysis, site: Site) -> Option<(SourceFile, u32)> {
        let file = *analysis.documents().get(site.document as usize)?;
        let ast = file.parse(self.workspace.db()).ast()?;

        Some((file, ast.span(site.node).start))
    }

    pub(super) fn dependent_closure(self, file: SourceFile) -> Vec<SourceFile> {
        let mut files = vec![file];
        let mut next = 0;

        while let Some(&current) = files.get(next) {
            next += 1;

            let uri = current.uri(self.workspace.db());

            for dependent in self.workspace.dependents(uri) {
                let Some(dependent) = self.workspace.file(&dependent) else {
                    continue;
                };

                if !files.contains(&dependent) {
                    files.push(dependent);
                }
            }
        }

        files.remove(0);

        files
    }

    pub(super) fn analyses_of(self, file: SourceFile) -> Vec<(&'w Analysis, DocumentIndex)> {
        std::iter::once(file)
            .chain(self.dependent_closure(file))
            .filter_map(|candidate_root| {
                let analysis = self.file_analysis(candidate_root)?;
                let document = analysis
                    .documents()
                    .iter()
                    .position(|&candidate| candidate == file)?;

                Some((analysis, DocumentIndex::try_from(document).ok()?))
            })
            .collect()
    }

    fn locations(self, analysis: &Analysis, sites: Vec<Site>) -> Vec<Location> {
        let db = self.workspace.db();

        sites
            .into_iter()
            .filter_map(|site| {
                let file = *analysis.documents().get(site.document as usize)?;
                let ast = file.parse(db).ast()?;

                Some(Location {
                    uri: file.uri(db).clone(),
                    range: ast
                        .lines()
                        .range(ast.span(site.node), ColumnSettings::default()),
                })
            })
            .collect()
    }
}
