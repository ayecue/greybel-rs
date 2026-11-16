use std::ops::{BitAnd, BitAndAssign, BitOr, Not};

use greybel_core::syntax::Call;
use greybel_core::{BinaryOp, LiteralValue, LogicalOp, NodeId, NodeKind, UnaryOp};
use greybel_symbols::names::HAS_INDEX;
use greybel_symbols::types;
use rustc_hash::FxHashMap;

use super::AccessPath;
use crate::infer::Inference;
use crate::model::{MapId, SlotId};
use crate::types::{FunctionRef, LiteralType, Type, TypeId, TypeTable};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct KindSet(u8);

impl KindSet {
    pub(crate) const ALL: Self = Self((1 << 7) - 1);
    const CLASS: Self = Self(1 << 6);
    const FUNCTION: Self = Self(1 << 4);
    const LIST: Self = Self(1 << 2);
    const MAP: Self = Self(1 << 3);
    pub(crate) const NONE: Self = Self(0);
    pub(crate) const NULL: Self = Self(1 << 5);
    const NUMBER: Self = Self(1);
    const STRING: Self = Self(1 << 1);

    pub(crate) fn of(ty: &Type) -> Self {
        match ty {
            Type::Number | Type::Literal(LiteralType::Number(_)) => Self::NUMBER,
            Type::String | Type::Literal(LiteralType::String(_)) => Self::STRING,
            Type::List(_) => Self::LIST,
            Type::Map { .. } | Type::Object(_) => Self::MAP,
            Type::Function(_) => Self::FUNCTION,
            Type::Null => Self::NULL,
            Type::Never => Self::NONE,
            Type::Class(_) => Self::CLASS,
            Type::Unknown | Type::Any | Type::Union(_) | Type::TypeParameter { .. } => Self::ALL,
        }
    }

    fn from_type_name(name: &str) -> Self {
        match name {
            types::NUMBER => Self::NUMBER,
            types::STRING => Self::STRING,
            types::LIST => Self::LIST,
            types::MAP => Self::MAP,
            types::FUNCTION => Self::FUNCTION,
            types::NULL => Self::NULL,
            _ => Self::CLASS,
        }
    }

    fn from_type_map_name(name: &str) -> Option<Self> {
        Some(match name {
            types::NUMBER => Self::NUMBER,
            types::STRING => Self::STRING,
            types::LIST => Self::LIST,
            types::MAP => Self::MAP | Self::CLASS,
            types::FUNC_REF => Self::FUNCTION,
            _ => return None,
        })
    }
}

impl BitAnd for KindSet {
    type Output = Self;

    fn bitand(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }
}

impl BitAndAssign for KindSet {
    fn bitand_assign(&mut self, other: Self) {
        self.0 &= other.0;
    }
}

impl BitOr for KindSet {
    type Output = Self;

    fn bitor(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl Not for KindSet {
    type Output = Self;

    fn not(self) -> Self {
        Self(Self::ALL.0 & !self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum IsaTest {
    #[default]
    Unknown,
    Has(MapId),
    Missing(MapId),
}

impl IsaTest {
    pub(crate) fn and(self, other: Self) -> Self {
        match self {
            Self::Unknown => other,
            _ => self,
        }
    }

    pub(crate) fn or(self, other: Self) -> Self {
        if self == other { self } else { Self::Unknown }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum LiteralTest {
    #[default]
    Unknown,
    Only(TypeId),
    Except(TypeId),
}

impl LiteralTest {
    pub(crate) fn and(self, other: Self, types: &mut TypeTable) -> Self {
        match (self, other) {
            (Self::Unknown, test) | (test, Self::Unknown) => test,
            (Self::Only(left), Self::Only(right)) => Self::Only(Self::common(types, left, right)),
            (Self::Except(left), Self::Except(right)) => Self::Except(types.union(left, right)),
            (Self::Only(only), Self::Except(except)) | (Self::Except(except), Self::Only(only)) => {
                Self::Only(Self::difference(types, only, except))
            }
        }
    }

    pub(crate) fn or(self, other: Self, types: &mut TypeTable) -> Self {
        match (self, other) {
            (Self::Only(left), Self::Only(right)) => Self::Only(types.union(left, right)),
            (Self::Except(left), Self::Except(right)) => {
                let common = Self::common(types, left, right);

                if common == TypeId::NEVER {
                    Self::Unknown
                } else {
                    Self::Except(common)
                }
            }
            _ => Self::Unknown,
        }
    }

    fn common(types: &mut TypeTable, left: TypeId, right: TypeId) -> TypeId {
        let right: Vec<TypeId> = types.variants(right).collect();
        let common: Vec<TypeId> = types
            .variants(left)
            .filter(|variant| right.contains(variant))
            .collect();

        types.union_all(common)
    }

    fn difference(types: &mut TypeTable, left: TypeId, right: TypeId) -> TypeId {
        let right: Vec<TypeId> = types.variants(right).collect();
        let remainder: Vec<TypeId> = types
            .variants(left)
            .filter(|variant| !right.contains(variant))
            .collect();

        types.union_all(remainder)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Restriction {
    pub kinds: KindSet,
    pub isa: IsaTest,
    pub literal: LiteralTest,
}

impl Restriction {
    const ALL: Self = Self::of_kinds(KindSet::ALL);

    const fn of_kinds(kinds: KindSet) -> Self {
        Self {
            kinds,
            isa: IsaTest::Unknown,
            literal: LiteralTest::Unknown,
        }
    }

    fn and(self, other: Self, types: &mut TypeTable) -> Self {
        Self {
            kinds: self.kinds & other.kinds,
            isa: self.isa.and(other.isa),
            literal: self.literal.and(other.literal, types),
        }
    }

    fn or(self, other: Self, types: &mut TypeTable) -> Self {
        Self {
            kinds: self.kinds | other.kinds,
            isa: self.isa.or(other.isa),
            literal: self.literal.or(other.literal, types),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct FactTable(FxHashMap<AccessPath, Restriction>);

impl FactTable {
    fn for_path(path: AccessPath, restriction: Restriction) -> Self {
        Self(FxHashMap::from_iter([(path, restriction)]))
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&AccessPath, Restriction)> + '_ {
        self.0
            .iter()
            .map(|(path, &restriction)| (path, restriction))
    }

    pub(crate) fn intersection(&self, other: &Self, types: &mut TypeTable) -> Self {
        let mut facts = self.0.clone();

        for (path, &restriction) in &other.0 {
            let fact = facts.entry(path.clone()).or_insert(Restriction::ALL);

            *fact = fact.and(restriction, types);
        }

        Self(facts)
    }

    fn union(&self, other: &Self, types: &mut TypeTable) -> Self {
        Self(
            self.0
                .iter()
                .filter_map(|(path, &restriction)| {
                    Some((path.clone(), restriction.or(*other.0.get(path)?, types)))
                })
                .collect(),
        )
    }
}

#[derive(Default)]
pub(crate) struct Guard {
    pub when_true: FactTable,
    pub when_false: FactTable,
}

impl Guard {
    fn for_path(path: AccessPath, kinds: KindSet) -> Self {
        Self {
            when_true: FactTable::for_path(path.clone(), Restriction::of_kinds(kinds)),
            when_false: FactTable::for_path(path, Restriction::of_kinds(!kinds)),
        }
    }

    fn truthy(path: AccessPath) -> Self {
        Self {
            when_true: FactTable::for_path(path, Restriction::of_kinds(!KindSet::NULL)),
            when_false: FactTable::default(),
        }
    }

    fn isa(path: AccessPath, class: Option<MapId>) -> Self {
        let restriction = |kinds, isa| Restriction {
            kinds,
            isa,
            literal: LiteralTest::Unknown,
        };

        Self {
            when_true: FactTable::for_path(
                path.clone(),
                restriction(
                    KindSet::MAP | KindSet::CLASS,
                    class.map_or(IsaTest::Unknown, IsaTest::Has),
                ),
            ),
            when_false: FactTable::for_path(
                path,
                restriction(
                    KindSet::ALL,
                    class.map_or(IsaTest::Unknown, IsaTest::Missing),
                ),
            ),
        }
    }

    fn negate(self) -> Self {
        Self {
            when_true: self.when_false,
            when_false: self.when_true,
        }
    }
}

enum IsaOperand {
    Kinds(KindSet),
    Map(Option<MapId>),
}

impl Inference<'_> {
    pub(crate) fn condition_guard(&mut self, condition: NodeId) -> Guard {
        let ast = self.ast();

        match ast.kind(condition) {
            NodeKind::Parenthesis(parenthesis) => self.condition_guard(parenthesis.expression),
            NodeKind::Unary(unary) if unary.operator == UnaryOp::Not => {
                self.condition_guard(unary.argument).negate()
            }
            NodeKind::Logical(logical) => {
                let left = self.condition_guard(logical.left);
                let right = self.condition_guard(logical.right);

                match logical.operator {
                    LogicalOp::And => Guard {
                        when_true: left
                            .when_true
                            .intersection(&right.when_true, &mut self.types),
                        when_false: self.union_facts(&left.when_false, &right.when_false),
                    },
                    LogicalOp::Or => Guard {
                        when_true: self.union_facts(&left.when_true, &right.when_true),
                        when_false: left
                            .when_false
                            .intersection(&right.when_false, &mut self.types),
                    },
                }
            }
            NodeKind::Isa(isa) => {
                let Some(path) = self.access_path(isa.left) else {
                    return Guard::default();
                };

                match self.isa_operand(isa.right) {
                    Some(IsaOperand::Kinds(kinds)) => Guard::for_path(path, kinds),
                    Some(IsaOperand::Map(class)) => Guard::isa(path, class),
                    None => Guard::default(),
                }
            }
            NodeKind::Binary(binary) => {
                self.equality_guard(binary.operator, binary.left, binary.right)
            }
            NodeKind::ComparisonGroup(group) => {
                match (&group.operators[..], ast.list(group.expressions)) {
                    ([operator], [left, right]) => self.equality_guard(*operator, *left, *right),
                    _ => Guard::default(),
                }
            }
            NodeKind::Identifier(_) | NodeKind::Member(_) | NodeKind::Index(_) => self
                .access_path(condition)
                .map_or_else(Guard::default, Guard::truthy),
            NodeKind::Call(call) => {
                if let Some(path) = self.has_index_test(call) {
                    return Guard {
                        when_true: FactTable::for_path(path, Restriction::ALL),
                        when_false: FactTable::default(),
                    };
                }

                let Some(&(index, ty)) = self.guard_calls.get(&(self.current_document, condition))
                else {
                    return Guard::default();
                };
                let Some(path) = ast
                    .list(call.arguments)
                    .get(index)
                    .and_then(|&argument| self.access_path(argument))
                else {
                    return Guard::default();
                };

                self.type_guard(path, ty)
            }
            _ => Guard::default(),
        }
    }

    fn type_guard(&self, path: AccessPath, ty: TypeId) -> Guard {
        let maps: Vec<MapId> = self.types.object_maps(ty).collect();

        if let [map] = maps[..]
            && self.types.variants(ty).count() == 1
        {
            return Guard::isa(path, Some(map));
        }

        let kinds = self
            .types
            .variants(ty)
            .fold(KindSet::NONE, |kinds, variant| {
                kinds | KindSet::of(self.types.get(variant))
            });

        Guard::for_path(path, kinds)
    }

    fn equality_guard(&mut self, operator: BinaryOp, left: NodeId, right: NodeId) -> Guard {
        let comparison = match operator {
            BinaryOp::Equal | BinaryOp::NotEqual => self
                .comparison_kinds(left, right)
                .or_else(|| self.comparison_kinds(right, left)),
            _ => None,
        };
        let guard = match comparison {
            Some((path, kinds)) => Guard::for_path(path, kinds),
            None => match operator {
                BinaryOp::Equal | BinaryOp::NotEqual => self
                    .literal_guard(left, right)
                    .or_else(|| self.literal_guard(right, left)),
                _ => None,
            }
            .unwrap_or_default(),
        };

        if operator == BinaryOp::NotEqual {
            guard.negate()
        } else {
            guard
        }
    }

    fn comparison_kinds(
        &mut self,
        subject: NodeId,
        against: NodeId,
    ) -> Option<(AccessPath, KindSet)> {
        let ast = self.ast();
        let NodeKind::Literal(literal) = ast.kind(against) else {
            return None;
        };

        match (&literal.value, ast.kind(subject)) {
            (LiteralValue::Nil, _) => Some((self.access_path(subject)?, KindSet::NULL)),
            (LiteralValue::String(name), NodeKind::Call(call)) => {
                if self.identifier_name(call.base)? != "typeof" || call.arguments.len() != 1 {
                    return None;
                }

                Some((
                    self.access_path(ast.list(call.arguments)[0])?,
                    KindSet::from_type_name(&ast.read(*name)),
                ))
            }
            _ => None,
        }
    }

    fn has_index_test(&mut self, call: &Call) -> Option<AccessPath> {
        let ast = self.ast();
        let NodeKind::Member(member) = ast.kind(call.base) else {
            return None;
        };
        let [argument] = ast.list(call.arguments) else {
            return None;
        };

        if ast.text(member.identifier) != Some(HAS_INDEX) {
            return None;
        }

        let NodeKind::Literal(literal) = ast.kind(*argument) else {
            return None;
        };
        let LiteralValue::String(text) = &literal.value else {
            return None;
        };
        let name = self.model.insert_name(&ast.read(*text));

        Some(self.access_path(member.base)?.with_member(name))
    }

    fn read_type(&self, node: NodeId) -> TypeId {
        let ast = self.ast();
        let node = match ast.kind(node) {
            NodeKind::Parenthesis(parenthesis) => return self.read_type(parenthesis.expression),
            NodeKind::Unary(unary) if unary.operator == UnaryOp::AddressOf => unary.argument,
            _ => node,
        };

        self.node_types[self.current_document as usize]
            .get(node.index())
            .copied()
            .flatten()
            .unwrap_or(TypeId::ANY)
    }

    fn literal_guard(&mut self, subject: NodeId, against: NodeId) -> Option<Guard> {
        let ast = self.ast();
        let NodeKind::Literal(literal) = ast.kind(against) else {
            return None;
        };
        let kinds = match literal.value {
            LiteralValue::String(_) => KindSet::STRING,
            LiteralValue::Number(_) => KindSet::NUMBER,
            _ => return None,
        };
        let path = self.access_path(subject)?;
        let ty = match path.variable_slot() {
            Some(slot) => match self.flow.fact(slot) {
                Some(fact) => self.fact_type(slot, fact),
                None => self.model.slot_type(slot),
            },
            None => self.read_type(subject),
        };
        let (mut has_literals, mut other_kinds) = (false, false);

        for variant in self.types.variants(ty) {
            match self.types.get(variant) {
                Type::Literal(_) => has_literals = true,
                other => other_kinds |= KindSet::of(other) & kinds == KindSet::NONE,
            }
        }

        if !has_literals {
            return other_kinds.then(|| Guard {
                when_true: FactTable::for_path(path, Restriction::of_kinds(kinds)),
                when_false: FactTable::default(),
            });
        }

        let literal = self.literal_type_of(against)?;

        Some(Guard {
            when_true: FactTable::for_path(
                path.clone(),
                Restriction {
                    literal: LiteralTest::Only(literal),
                    ..Restriction::of_kinds(kinds)
                },
            ),
            when_false: FactTable::for_path(
                path,
                Restriction {
                    literal: LiteralTest::Except(literal),
                    ..Restriction::ALL
                },
            ),
        })
    }

    fn union_facts(&mut self, left: &FactTable, right: &FactTable) -> FactTable {
        match (self.is_impossible(left), self.is_impossible(right)) {
            (true, false) => right.clone(),
            (false, true) => left.clone(),
            (true, true) => left.intersection(right, &mut self.types),
            (false, false) => left.union(right, &mut self.types),
        }
    }

    pub(crate) fn restrict_to_literals(&mut self, ty: TypeId, test: LiteralTest) -> TypeId {
        let (literals, only) = match test {
            LiteralTest::Unknown => return ty,
            LiteralTest::Only(literals) => (literals, true),
            LiteralTest::Except(literals) => (literals, false),
        };

        if !self
            .types
            .variants(ty)
            .any(|variant| matches!(self.types.get(variant), Type::Literal(_)))
        {
            return ty;
        }

        let literals: Vec<TypeId> = self.types.variants(literals).collect();
        let matches: Vec<TypeId> = self
            .types
            .variants(ty)
            .filter(|&variant| {
                !matches!(self.types.get(variant), Type::Literal(_))
                    || literals.contains(&variant) == only
            })
            .collect();

        self.types.union_all(matches)
    }

    fn is_impossible(&self, facts: &FactTable) -> bool {
        facts.0.iter().any(|(path, &Restriction { kinds, .. })| {
            let Some(slot) = path.variable_slot() else {
                return false;
            };
            let fact = self.flow.fact(slot);
            let allowed_kinds = fact.map_or(KindSet::ALL, |fact| fact.kinds);
            let ty = fact
                .and_then(|fact| fact.assignment)
                .unwrap_or_else(|| self.model.slot_type(slot));

            if ty == TypeId::ANY || ty == TypeId::UNKNOWN {
                return kinds & allowed_kinds == KindSet::NONE;
            }

            let present = self.types.variants(ty).fold(KindSet::NONE, |acc, variant| {
                match self.types.get(variant) {
                    Type::Any | Type::Unknown => acc | KindSet::ALL,
                    other => acc | KindSet::of(other),
                }
            });

            present & allowed_kinds & kinds == KindSet::NONE
        })
    }

    pub(crate) fn access_path(&mut self, node: NodeId) -> Option<AccessPath> {
        match self.ast().kind(node) {
            NodeKind::Identifier(_) => self.identifier_slot(node).map(AccessPath::variable),
            NodeKind::Parenthesis(parenthesis) => self.access_path(parenthesis.expression),
            NodeKind::Unary(unary) if unary.operator == UnaryOp::AddressOf => {
                self.access_path(unary.argument)
            }
            NodeKind::Member(_) | NodeKind::Index(_) => self.member_access_path(node),
            _ => None,
        }
    }

    fn identifier_slot(&self, node: NodeId) -> Option<SlotId> {
        self.resolve_variable(self.identifier_name_id(node)?)
    }

    fn isa_operand(&self, node: NodeId) -> Option<IsaOperand> {
        let name = self.identifier_name(node)?;
        let Some(slot) = self.resolve_variable(self.identifier_name_id(node)?) else {
            return KindSet::from_type_map_name(name).map(IsaOperand::Kinds);
        };
        let ty = self
            .flow
            .fact(slot)
            .and_then(|fact| fact.assignment)
            .unwrap_or_else(|| self.model.slot_type(slot));
        let mut maps = self.types.object_maps(ty);
        let class = match (maps.next(), maps.next()) {
            (Some(map), None) if self.types.variants(ty).count() == 1 => Some(map),
            _ => None,
        };

        Some(IsaOperand::Map(class))
    }

    pub(crate) fn restrict_to_isa(&mut self, ty: TypeId, test: IsaTest) -> TypeId {
        let (class, has) = match test {
            IsaTest::Unknown => return ty,
            IsaTest::Has(class) => (class, true),
            IsaTest::Missing(class) => (class, false),
        };
        let variants: Vec<TypeId> = self
            .types
            .variants(ty)
            .filter(|&variant| {
                let instance = self.types.object_map(variant).is_some_and(|map| {
                    map != class && self.model.isa_chain(&self.types, map).contains(&class)
                });

                instance == has
            })
            .collect();

        if has && variants.is_empty() {
            return self.object_type(class);
        }

        self.types.union_all(variants)
    }

    pub(crate) fn restrict_to_kinds(&mut self, ty: TypeId, kinds: KindSet) -> TypeId {
        if kinds == KindSet::ALL {
            return ty;
        }

        if ty == TypeId::ANY || ty == TypeId::UNKNOWN {
            return match kinds {
                KindSet::NUMBER => TypeId::NUMBER,
                KindSet::STRING => TypeId::STRING,
                KindSet::NULL => TypeId::NULL,
                KindSet::LIST => self.types.list(TypeId::ANY),
                KindSet::FUNCTION => self.types.function(FunctionRef::Any),
                _ => ty,
            };
        }

        if let Some(&kinds_type) = self.restricted_types.get(&(ty, kinds)) {
            return kinds_type;
        }

        let variants: Vec<TypeId> = self
            .types
            .variants(ty)
            .filter(|&variant| match self.types.get(variant) {
                Type::Any => true,
                other => KindSet::of(other) & kinds != KindSet::NONE,
            })
            .collect();
        let kinds_type = self.types.union_all(variants);

        self.restricted_types.insert((ty, kinds), kinds_type);

        kinds_type
    }
}
