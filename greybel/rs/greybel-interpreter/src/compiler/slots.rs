use std::borrow::Cow;
use std::rc::Rc;

use greybel_core::{IdentifierKind, NodeId, NodeKind};
use greybel_symbols::names::{GLOBALS, ISA, LOCALS, OUTER, SELF, SUPER};
use rustc_hash::FxHashMap;

use super::Compiler;
use crate::bytecode::SlotLayout;
use crate::value::{Key, Param};

const MAP_ONLY: [&str; 4] = [LOCALS, OUTER, GLOBALS, ISA];

pub(crate) struct SlotBuilder {
    slots: FxHashMap<Rc<str>, u32>,
    names: Vec<Key>,
}

impl SlotBuilder {
    pub fn slot(&self, name: &str) -> Option<u32> {
        self.slots.get(name).copied()
    }

    fn add(&mut self, name: Rc<str>) -> u32 {
        if let Some(&slot) = self.slots.get(&name) {
            return slot;
        }

        let slot = u32::try_from(self.names.len()).expect("fewer than 2^32 locals");

        self.names.push(Key::new(Rc::clone(&name)));
        self.slots.insert(name, slot);

        slot
    }

    pub fn finish(&self, params: &[Param]) -> SlotLayout {
        let slot = |name: &str| self.slot(name).expect("every parameter has a slot");

        SlotLayout {
            params: params.iter().map(|param| slot(&param.name)).collect(),
            self_slot: self.slot(SELF),
            super_slot: self.slot(SUPER),
            names: self.names.clone(),
        }
    }
}

#[derive(Default)]
struct LocalTable<'g> {
    names: Vec<Cow<'g, str>>,
    uses_self: bool,
    uses_super: bool,
}

impl<'g> Compiler<'g> {
    pub(crate) fn slot_layout(
        &mut self,
        function: NodeId,
        params: &[Param],
    ) -> Option<SlotBuilder> {
        if self.debug() {
            return None;
        }

        let LocalTable {
            names,
            uses_self,
            uses_super,
        } = self.locals(function)?;

        if names.iter().any(|name| MAP_ONLY.contains(&name.as_ref())) {
            return None;
        }

        let mut layout = SlotBuilder {
            slots: FxHashMap::default(),
            names: Vec::new(),
        };

        for param in params {
            layout.add(Rc::clone(&param.name));
        }

        if uses_self {
            layout.add(self.shared_str(SELF));
        }

        if uses_super {
            layout.add(self.shared_str(SUPER));
        }

        for name in names {
            let name = self.shared_str(&name);

            layout.add(name);
        }

        Some(layout)
    }

    fn locals(&self, function: NodeId) -> Option<LocalTable<'g>> {
        let ast = self.ast();
        let NodeKind::Function(node) = ast.kind(function) else {
            return None;
        };
        let directives = [
            &ast.imports,
            &ast.includes,
            &ast.injects,
            &ast.native_imports,
            &ast.debuggers,
        ];

        if directives
            .into_iter()
            .flatten()
            .any(|&id| ast.node(id).scope == Some(function))
        {
            return None;
        }

        let name = |id| match ast.kind(id) {
            NodeKind::Identifier(node) => Some(ast.resolve(node.name)),
            _ => None,
        };
        let unwrap = |mut id| {
            while let NodeKind::Parenthesis(node) = ast.kind(id) {
                id = node.expression;
            }

            id
        };
        let mut locals = LocalTable::default();

        for &id in ast.list(node.scope.namespaces) {
            let name = match ast.kind(id) {
                NodeKind::Identifier(node) if node.kind != IdentifierKind::Argument => {
                    Some(ast.resolve(node.name))
                }
                _ => None,
            };

            match name {
                Some(LOCALS) => return None,
                Some(SELF) => locals.uses_self = true,
                Some(SUPER) => locals.uses_super = true,
                _ => {}
            }
        }

        let parameters = ast.list(node.parameters);

        for &id in ast.list(node.scope.definitions) {
            match ast.kind(id) {
                NodeKind::Assignment(node)
                    if parameters.contains(&id) || parameters.contains(&node.variable) => {}
                NodeKind::Assignment(node) => {
                    if matches!(ast.kind(unwrap(node.init)), NodeKind::Function(_)) {
                        return None;
                    }

                    let mut target = unwrap(node.variable);

                    if let NodeKind::Unary(unary) = ast.kind(target) {
                        target = unwrap(unary.argument);
                    }

                    if let Some(name) = name(target) {
                        locals.names.push(Cow::Borrowed(name));
                    }
                }
                NodeKind::For(node) => {
                    if let Some(name) = name(node.variable) {
                        locals.names.push(Cow::Borrowed(name));
                        locals.names.push(Cow::Owned(Self::loop_index(name)));
                    }
                }
                _ => {}
            }
        }

        Some(locals)
    }
}
