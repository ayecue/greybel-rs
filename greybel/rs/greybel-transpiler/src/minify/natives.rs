use std::borrow::Cow;

use greybel_core::syntax::{Identifier, IdentifierKind, Literal};
use greybel_core::{Ast, LiteralValue, Node, NodeId, NodeKind, Visitor};
use greybel_symbols::Keyword;
use greybel_symbols::names::{GLOBALS, SCOPES};
use greybel_symbols::types::{FUNC_REF, LIST, MAP, NUMBER, STRING};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::names::NameTable;

/// Type maps, which are values, not functions; `import_code` is skipped as well, since the build resolves it.
const SKIPPED_NAMES: &[&str] = &[LIST, MAP, STRING, NUMBER, FUNC_REF];
const ADDRESS_OF: &str = "=@";
const NEW_LINE: usize = 1;

/// General API functions that uglify calls through short global aliases, assigned in the header as
/// `globals.<alias>=@<name>`. A name gets an alias when the characters its uses save exceed the length of that line.
/// A name the build assigns (sugar such as classes and destructuring lowers to assignments), binds with `#import`,
/// takes as a parameter or loop variable, or writes through `globals`, `locals` or `outer` gets none; a write through
/// those with a computed key turns off every alias.
#[derive(Debug, Clone, Default)]
pub(super) struct NativeAliasTable {
    aliases: FxHashMap<Box<str>, Box<str>>,
    order: Vec<(Box<str>, Box<str>)>,
}

impl NativeAliasTable {
    pub(super) fn of(
        asts: &[&Ast],
        natives: &[String],
        excluded_names: &[String],
        variables: &mut NameTable,
    ) -> Self {
        let natives: FxHashSet<&str> = natives
            .iter()
            .map(String::as_str)
            .filter(|name| {
                !SKIPPED_NAMES.contains(name)
                    && *name != Keyword::ImportCode.as_str()
                    && !excluded_names.iter().any(|other| other == name)
            })
            .collect();
        let mut usage = NativeUsage {
            ast: asts.first().copied(),
            natives: &natives,
            counts: FxHashMap::default(),
            exclusions: FxHashSet::default(),
            is_dynamic: false,
        };

        for &ast in asts {
            usage.ast = Some(ast);
            ast.walk(ast.root(), &mut usage);
        }

        if usage.is_dynamic {
            return Self::default();
        }

        let globals_length = variables.get(GLOBALS).unwrap_or(GLOBALS).chars().count();
        let mut candidates: Vec<(&str, usize)> = usage
            .counts
            .into_iter()
            .filter(|(name, _)| !usage.exclusions.contains(name))
            .collect();

        candidates.sort_by(|(left, left_count), (right, right_count)| {
            (right_count * right.len())
                .cmp(&(left_count * left.len()))
                .then_with(|| left.cmp(right))
        });

        let mut table = Self::default();

        for (name, count) in candidates {
            let alias_length = variables.next_length();
            let name_length = name.chars().count();
            let line_length =
                globals_length + 1 + alias_length + ADDRESS_OF.len() + name_length + NEW_LINE;

            if count * name_length.saturating_sub(alias_length) <= line_length {
                continue;
            }

            let Some(alias) = variables.create(&format!("\0native:{name}")) else {
                continue;
            };
            let alias: Box<str> = alias.into();

            table.aliases.insert(name.into(), alias.clone());
            table.order.push((name.into(), alias));
        }

        table
    }

    pub(super) fn alias(&self, name: &str) -> Option<&str> {
        self.aliases.get(name).map(AsRef::as_ref)
    }

    /// `(name, alias)` in the order the aliases were assigned.
    pub(super) fn entries(&self) -> impl Iterator<Item = (&str, &str)> {
        self.order
            .iter()
            .map(|(name, alias)| (name.as_ref(), alias.as_ref()))
    }
}

struct NativeUsage<'a> {
    ast: Option<&'a Ast>,
    natives: &'a FxHashSet<&'a str>,
    counts: FxHashMap<&'a str, usize>,
    exclusions: FxHashSet<&'a str>,
    is_dynamic: bool,
}

impl<'a> NativeUsage<'a> {
    fn native(&self, name: &str) -> Option<&'a str> {
        self.natives.get(name).copied()
    }

    fn add_identifier(&mut self, ast: &Ast, identifier: &Identifier) {
        let Some(name) = self.native(ast.resolve(identifier.name)) else {
            return;
        };

        match identifier.kind {
            IdentifierKind::Variable => *self.counts.entry(name).or_default() += 1,
            IdentifierKind::Argument | IdentifierKind::ForInVariable => {
                self.exclusions.insert(name);
            }
            IdentifierKind::Property => {}
        }
    }

    /// Blocks `name` in `name = ...`, `globals.name = ...` and `globals["name"] = ...`; a computed key in a write
    /// through `globals`, `locals` or `outer` turns off every alias.
    fn add_write(&mut self, ast: &Ast, target: NodeId) {
        let (base, key) = match ast.kind(target) {
            NodeKind::Identifier(identifier) => {
                if let Some(name) = self.native(ast.resolve(identifier.name)) {
                    self.exclusions.insert(name);
                }

                return;
            }
            NodeKind::Member(member) => (
                member.base,
                Self::identifier_name(ast, member.identifier).map(Cow::Borrowed),
            ),
            NodeKind::Index(index) => (index.base, Self::string_literal(ast, index.index)),
            _ => return,
        };

        if !Self::identifier_name(ast, base).is_some_and(|name| SCOPES.contains(&name)) {
            return;
        }

        match key {
            Some(key) => {
                if let Some(name) = self.native(&key) {
                    self.exclusions.insert(name);
                }
            }
            None => self.is_dynamic = true,
        }
    }

    fn identifier_name(ast: &Ast, id: NodeId) -> Option<&str> {
        match ast.kind(id) {
            NodeKind::Identifier(identifier) => Some(ast.resolve(identifier.name)),
            _ => None,
        }
    }

    fn string_literal(ast: &Ast, id: NodeId) -> Option<Cow<'_, str>> {
        match ast.kind(id) {
            NodeKind::Literal(Literal {
                value: LiteralValue::String(text),
                ..
            }) => Some(text.read(ast.source())),
            _ => None,
        }
    }
}

impl Visitor for NativeUsage<'_> {
    fn enter(&mut self, _id: NodeId, node: &Node) -> bool {
        let Some(ast) = self.ast else {
            return false;
        };

        match &node.kind {
            NodeKind::Identifier(identifier) => self.add_identifier(ast, identifier),
            NodeKind::Assignment(assignment) => self.add_write(ast, assignment.variable),
            NodeKind::Import(import) => self.add_write(ast, import.name),
            _ => {}
        }

        true
    }
}
