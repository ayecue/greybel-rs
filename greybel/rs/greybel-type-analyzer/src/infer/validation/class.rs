use greybel_core::syntax::greybel::Class;
use greybel_core::{NodeId, NodeKind, Span};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::diagnostic::DiagnosticKind;
use crate::infer::Inference;
use crate::model::{DocumentIndex, Key, KeyRef, MapId, MapOrigin, ScopeId, Site};
use crate::types::{Type, TypeId, TypeNotation};

#[derive(Debug, Default)]
pub(crate) struct ClassTable {
    names: FxHashMap<MapId, String>,
    private: FxHashSet<(MapId, Box<str>)>,
    methods: FxHashMap<Site, MapId>,
    declarations: FxHashSet<Site>,
    implementations: Vec<Implementation>,
}

#[derive(Debug)]
struct Implementation {
    map: MapId,
    document: DocumentIndex,
    type_name: String,
    span: Span,
}

impl Inference<'_> {
    pub(crate) fn register_class(&mut self, class: &Class) {
        let ast = self.ast();
        let Some(&head) = ast.list(class.statements).first() else {
            return;
        };
        let NodeKind::Assignment(assignment) = ast.kind(head) else {
            return;
        };
        let Some(ty) = self.node_types[self.current_document as usize][assignment.init.index()]
        else {
            return;
        };
        let maps: Vec<MapId> = self.types.object_maps(ty).collect();
        let name = ast.text(class.name).unwrap_or_default().to_owned();

        for map in maps {
            self.class_statements.names.insert(map, name.clone());

            for member in &class.members {
                if !member.private {
                    continue;
                }

                if let Some(member_name) = Self::member_name(ast, member.assignment) {
                    self.class_statements
                        .private
                        .insert((map, member_name.into()));
                }
            }

            for &statement in ast.list(class.statements) {
                let NodeKind::Assignment(assignment) = ast.kind(statement) else {
                    continue;
                };

                if let NodeKind::Member(target) = ast.kind(assignment.variable) {
                    let site = self.site(target.identifier);

                    self.class_statements.declarations.insert(site);
                }

                if matches!(ast.kind(assignment.init), NodeKind::Function(_)) {
                    let site = self.site(assignment.init);

                    self.class_statements.methods.insert(site, map);
                }
            }

            for &span in &class.implements {
                if self.context.current.is_some() {
                    continue;
                }

                let implementation = Implementation {
                    map,
                    document: self.current_document,
                    type_name: ast.slice(span).to_owned(),
                    span,
                };
                let is_listed = self.class_statements.implementations.iter().any(|entry| {
                    entry.map == map
                        && entry.span == span
                        && entry.document == implementation.document
                });

                if !is_listed {
                    self.class_statements.implementations.push(implementation);
                }
            }
        }
    }

    fn member_name(ast: &greybel_core::Ast, assignment: NodeId) -> Option<&str> {
        let NodeKind::Assignment(assignment) = ast.kind(assignment) else {
            return None;
        };
        let NodeKind::Member(target) = ast.kind(assignment.variable) else {
            return None;
        };

        ast.text(target.identifier)
    }

    pub(crate) fn validate_optional_member(
        &mut self,
        node: NodeId,
        identifier: NodeId,
        base: TypeId,
        name: &str,
    ) {
        if self.optional_members.is_empty() {
            return;
        }

        let is_optional = self.types.object_maps(base).any(|map| {
            self.model
                .isa_chain(&self.types, map)
                .into_iter()
                .any(|map| self.optional_members.contains(&(map, name.into())))
        });

        if !is_optional || self.member_fact(node).is_some() {
            return;
        }

        self.report(
            DiagnosticKind::OptionalMember,
            identifier,
            format!("\"{name}\" is optional; check hasIndex(\"{name}\") before reading it."),
        );
    }

    pub(crate) fn validate_private_member(&mut self, node: NodeId, base: TypeId, name: &str) {
        if self.class_statements.private.is_empty()
            || self
                .class_statements
                .declarations
                .contains(&self.site(node))
        {
            return;
        }

        let owner = self.types.object_maps(base).find_map(|map| {
            self.model
                .isa_chain(&self.types, map)
                .into_iter()
                .find(|&map| self.class_statements.private.contains(&(map, name.into())))
        });
        let Some(owner) = owner else {
            return;
        };

        if self.is_in_method_of(owner) {
            return;
        }

        let class = self
            .class_statements
            .names
            .get(&owner)
            .cloned()
            .unwrap_or_default();

        self.report(
            DiagnosticKind::PrivateMember,
            node,
            format!("\"{name}\" is private to \"{class}\"."),
        );
    }

    pub(crate) fn validate_readonly_member(&mut self, node: NodeId, base: TypeId, name: &str) {
        if self.readonly_members.is_empty() {
            return;
        }

        let owner = self.types.object_maps(base).find_map(|map| {
            self.model
                .isa_chain(&self.types, map)
                .into_iter()
                .find(|&map| self.readonly_members.contains(&(map, name.into())))
        });
        let Some(owner) = owner else {
            return;
        };
        let type_name = self
            .model
            .custom_type_of(owner)
            .map(|ty| ty.name.clone())
            .unwrap_or_default();

        self.report(
            DiagnosticKind::ReadonlyMember,
            node,
            format!("\"{name}\" is readonly in \"{type_name}\"."),
        );
    }

    fn is_in_method_of(&self, owner: MapId) -> bool {
        let mut scope = self.current_scope;

        while let ScopeId::Function(function) = scope {
            let data = self.model.function(function);

            if let Some(&class) = self.class_statements.methods.get(&data.site)
                && self.model.isa_chain(&self.types, class).contains(&owner)
            {
                return true;
            }

            scope = data.outer_scope;
        }

        false
    }

    pub(crate) fn validate_implementations(&mut self) {
        let implementations = std::mem::take(&mut self.class_statements.implementations);
        let notation = TypeNotation::new(&self.model, &self.types);
        let mut reports = Vec::new();

        for implementation in &implementations {
            let class = self
                .class_statements
                .names
                .get(&implementation.map)
                .cloned()
                .unwrap_or_default();
            let Some(custom) = self.model.custom_type(&implementation.type_name) else {
                reports.push((
                    implementation,
                    DiagnosticKind::MissingImplementation,
                    format!("Cannot find the type \"{}\".", implementation.type_name),
                ));

                continue;
            };
            let chain = self.model.isa_chain(&self.types, implementation.map);

            for type_map in self.model.isa_chain(&self.types, custom.map) {
                for (key, &type_slot) in &self.model.map(type_map).entries {
                    let Key::String(member) = key else {
                        continue;
                    };
                    let key = KeyRef::String(member);
                    let class_slot = chain.iter().find_map(|&map| {
                        self.model.map(map).own_entry(key).or_else(|| {
                            self.model
                                .map_of(MapOrigin::Instance(map))
                                .and_then(|instances| self.model.map(instances).own_entry(key))
                        })
                    });
                    let Some(class_slot) = class_slot else {
                        if self
                            .optional_members
                            .contains(&(type_map, member.as_ref().into()))
                        {
                            continue;
                        }

                        reports.push((
                            implementation,
                            DiagnosticKind::MissingImplementation,
                            format!(
                                "\"{class}\" implements \"{}\" but has no \"{member}\".",
                                implementation.type_name
                            ),
                        ));

                        continue;
                    };
                    let (actual, target) = (
                        self.model.slot_type(class_slot),
                        self.model.slot_type(type_slot),
                    );

                    if !self.implements_member_type(actual, target) {
                        reports.push((
                            implementation,
                            DiagnosticKind::MissingImplementation,
                            format!(
                                "\"{member}\" is {} in \"{class}\" but {} in \"{}\".",
                                notation.display(actual),
                                notation.display(target),
                                implementation.type_name
                            ),
                        ));
                    }
                }
            }
        }

        let reports: Vec<_> = reports
            .into_iter()
            .map(|(implementation, kind, message)| {
                (implementation.document, kind, implementation.span, message)
            })
            .collect();

        for (document, kind, span, message) in reports {
            self.report_span(document, kind, span, message);
        }

        self.class_statements.implementations = implementations;
    }

    fn implements_member_type(&self, actual: TypeId, target: TypeId) -> bool {
        let is_function = |ty: TypeId| {
            self.types
                .variants(ty)
                .all(|variant| matches!(self.types.get(variant), Type::Function(_)))
        };

        if is_function(actual) && is_function(target) {
            return true;
        }

        self.types.variants(actual).all(|variant| {
            matches!(self.types.get(variant), Type::Null) || self.is_assignable(variant, target)
        })
    }
}
