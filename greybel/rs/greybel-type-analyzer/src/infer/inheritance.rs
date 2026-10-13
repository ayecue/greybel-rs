use greybel_core::Span;
use greybel_symbols::types::MAP;

use super::Inference;
use crate::diagnostic::DiagnosticKind;
use crate::model::{DocumentIndex, MapId, MapOrigin, ScopeId};
use crate::types::Type;

impl Inference<'_> {
    pub(super) fn validate_inheritance(&mut self) {
        let mut cycles: Vec<Vec<MapId>> = self
            .isa_cycles()
            .into_iter()
            .filter_map(|mut cycle| {
                let (at, _) = cycle
                    .iter()
                    .enumerate()
                    .filter(|(_, map)| self.model.map(**map).context.is_none())
                    .min_by_key(|(_, map)| **map)?;

                cycle.rotate_left(at);

                Some(cycle)
            })
            .collect();

        cycles.sort_unstable_by_key(|cycle| cycle[0]);
        for cycle in cycles {
            self.report_cycle(&cycle);
        }
    }

    fn parent_map(&self, map: MapId) -> Option<MapId> {
        let parent = self.model.slot_type(self.model.map(map).isa);

        match self.types.get(parent) {
            Type::Object(parent) => Some(*parent),
            _ => None,
        }
    }

    fn isa_cycles(&self) -> Vec<Vec<MapId>> {
        const UNSEEN: u8 = 0;
        const ON_PATH: u8 = 1;
        const COMPLETE: u8 = 2;
        let mut state = vec![UNSEEN; self.model.map_count()];
        let mut cycles = Vec::new();
        let mut path = Vec::new();

        for start in 0..state.len() {
            let mut current = MapId::new(start);

            loop {
                match state[current.index()] {
                    UNSEEN => {
                        state[current.index()] = ON_PATH;
                        path.push(current);

                        match self.parent_map(current) {
                            Some(parent) => current = parent,
                            None => break,
                        }
                    }
                    ON_PATH => {
                        let position = path
                            .iter()
                            .position(|&map| map == current)
                            .expect("a map on the path was pushed");

                        cycles.push(path[position..].to_vec());
                        break;
                    }
                    _ => break,
                }
            }

            for map in path.drain(..) {
                state[map.index()] = COMPLETE;
            }
        }

        cycles
    }

    fn report_cycle(&mut self, cycle: &[MapId]) {
        let custom = cycle
            .iter()
            .all(|&map| matches!(self.model.map(map).origin, MapOrigin::Custom(_)));
        let Some((document, span)) = cycle.iter().find_map(|&map| self.isa_span(map)) else {
            return;
        };
        let names: Vec<String> = cycle
            .iter()
            .chain(&cycle[..1])
            .map(|&map| self.map_name(map))
            .collect();
        let (kind, label) = if custom {
            (
                DiagnosticKind::CyclicTypeDefinition,
                "Cyclic type definition",
            )
        } else {
            (DiagnosticKind::CyclicInheritance, "Cyclic inheritance")
        };

        self.report_span(
            document,
            kind,
            span,
            format!("{label}: {}", names.join(" -> ")),
        );
    }

    fn isa_span(&self, map: MapId) -> Option<(DocumentIndex, Span)> {
        if let MapOrigin::Custom(_) = self.model.map(map).origin {
            return self
                .type_declarations
                .iter()
                .filter(|declaration| declaration.map == map)
                .min_by_key(|declaration| declaration.document)
                .map(|declaration| (declaration.document, declaration.span));
        }

        let site = self
            .references
            .definition_sites(self.model.map(map).isa)
            .iter()
            .min_by_key(|site| site.document)?;
        let ast = self.documents[site.document as usize].ast;

        Some((site.document, ast.span(site.node)))
    }

    fn map_name(&mut self, map: MapId) -> String {
        if let Some(custom) = self.model.custom_type_of(map) {
            return custom.name.clone();
        }

        let object = self.object_type(map);

        self.model
            .variables(ScopeId::Global)
            .find(|&(_, slot)| self.model.slot_type(slot) == object)
            .map_or_else(|| MAP.to_owned(), |(name, _)| name.to_owned())
    }
}
