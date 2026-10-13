use std::ops::{BitAnd, BitAndAssign, BitOr, Not};

use greybel_core::{BinaryOp, LiteralValue, LogicalOp, NodeId, NodeKind, UnaryOp};
use greybel_symbols::types;
use rustc_hash::FxHashMap;

use super::Inference;
use crate::model::SlotId;
use crate::types::{FunctionRef, Type, TypeId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KindSet(u8);

impl KindSet {
    const ALL: Self = Self((1 << 7) - 1);
    const CLASS: Self = Self(1 << 6);
    const FUNCTION: Self = Self(1 << 4);
    const LIST: Self = Self(1 << 2);
    const MAP: Self = Self(1 << 3);
    const NONE: Self = Self(0);
    const NULL: Self = Self(1 << 5);
    const NUMBER: Self = Self(1);
    const STRING: Self = Self(1 << 1);

    fn of(ty: &Type) -> Self {
        match ty {
            Type::Number => Self::NUMBER,
            Type::String => Self::STRING,
            Type::List(_) => Self::LIST,
            Type::Map { .. } | Type::Object(_) => Self::MAP,
            Type::Function(_) => Self::FUNCTION,
            Type::Null => Self::NULL,
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

#[derive(Debug, Clone, Default)]
pub(crate) struct FactTable(FxHashMap<SlotId, KindSet>);

impl FactTable {
    fn for_slot(slot: SlotId, kinds: KindSet) -> Self {
        Self(FxHashMap::from_iter([(slot, kinds)]))
    }

    pub(crate) fn intersection(&self, other: &Self) -> Self {
        let mut facts = self.0.clone();

        for (&slot, &kinds) in &other.0 {
            *facts.entry(slot).or_insert(KindSet::ALL) &= kinds;
        }

        Self(facts)
    }

    fn union(&self, other: &Self) -> Self {
        Self(
            self.0
                .iter()
                .filter_map(|(&slot, &kinds)| Some((slot, kinds | *other.0.get(&slot)?)))
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
    fn for_slot(slot: SlotId, kinds: KindSet) -> Self {
        Self {
            when_true: FactTable::for_slot(slot, kinds),
            when_false: FactTable::for_slot(slot, !kinds),
        }
    }

    fn negate(self) -> Self {
        Self {
            when_true: self.when_false,
            when_false: self.when_true,
        }
    }
}

#[derive(Default)]
pub(crate) struct NarrowStack {
    frames: Vec<FxHashMap<SlotId, Option<KindSet>>>,
}

impl NarrowStack {
    fn allowed_kinds(&self, slot: SlotId) -> KindSet {
        let mut kinds = KindSet::ALL;

        for frame in self.frames.iter().rev() {
            match frame.get(&slot) {
                Some(None) => break,
                Some(Some(frame_kinds)) => kinds &= *frame_kinds,
                None => {}
            }
        }

        kinds
    }

    pub(crate) fn clear_facts(&mut self, slot: SlotId) {
        for frame in &mut self.frames {
            frame.insert(slot, None);
        }
    }

    fn push_frame(&mut self, facts: &FactTable) {
        self.frames.push(
            facts
                .0
                .iter()
                .map(|(&slot, &kinds)| (slot, Some(kinds)))
                .collect(),
        );
    }

    fn pop_frame(&mut self) {
        self.frames.pop();
    }
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
                        when_true: left.when_true.intersection(&right.when_true),
                        when_false: self.union_facts(&left.when_false, &right.when_false),
                    },
                    LogicalOp::Or => Guard {
                        when_true: self.union_facts(&left.when_true, &right.when_true),
                        when_false: left.when_false.intersection(&right.when_false),
                    },
                }
            }
            NodeKind::Isa(isa) => {
                let (Some(slot), Some(kinds)) =
                    (self.identifier_slot(isa.left), self.isa_kinds(isa.right))
                else {
                    return Guard::default();
                };

                Guard::for_slot(slot, kinds)
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
            _ => Guard::default(),
        }
    }

    fn equality_guard(&self, operator: BinaryOp, left: NodeId, right: NodeId) -> Guard {
        let comparison = match operator {
            BinaryOp::Equal | BinaryOp::NotEqual => self
                .comparison_kinds(left, right)
                .or_else(|| self.comparison_kinds(right, left)),
            _ => None,
        };
        let Some((slot, kinds)) = comparison else {
            return Guard::default();
        };
        let guard = Guard::for_slot(slot, kinds);

        if operator == BinaryOp::NotEqual {
            guard.negate()
        } else {
            guard
        }
    }

    fn comparison_kinds(&self, subject: NodeId, against: NodeId) -> Option<(SlotId, KindSet)> {
        let ast = self.ast();
        let NodeKind::Literal(literal) = ast.kind(against) else {
            return None;
        };

        match (&literal.value, ast.kind(subject)) {
            (LiteralValue::Nil, _) => Some((self.identifier_slot(subject)?, KindSet::NULL)),
            (LiteralValue::String(name), NodeKind::Call(call)) => {
                if self.identifier_name(call.base)? != "typeof" || call.arguments.len() != 1 {
                    return None;
                }

                Some((
                    self.identifier_slot(ast.list(call.arguments)[0])?,
                    KindSet::from_type_name(&ast.read(*name)),
                ))
            }
            _ => None,
        }
    }

    fn union_facts(&self, left: &FactTable, right: &FactTable) -> FactTable {
        match (self.is_impossible(left), self.is_impossible(right)) {
            (true, false) => right.clone(),
            (false, true) => left.clone(),
            (true, true) => left.intersection(right),
            (false, false) => left.union(right),
        }
    }

    fn is_impossible(&self, facts: &FactTable) -> bool {
        facts.0.iter().any(|(&slot, &kinds)| {
            let ty = self.model.slot_type(slot);

            if ty == TypeId::ANY || ty == TypeId::UNKNOWN {
                return kinds & self.narrow_stack.allowed_kinds(slot) == KindSet::NONE;
            }

            let present = self.types.variants(ty).fold(KindSet::NONE, |acc, variant| {
                match self.types.get(variant) {
                    Type::Any | Type::Unknown => acc | KindSet::ALL,
                    other => acc | KindSet::of(other),
                }
            });

            present & self.narrow_stack.allowed_kinds(slot) & kinds == KindSet::NONE
        })
    }

    fn identifier_slot(&self, node: NodeId) -> Option<SlotId> {
        self.resolve_variable(self.identifier_name(node)?)
    }

    fn isa_kinds(&self, node: NodeId) -> Option<KindSet> {
        let name = self.identifier_name(node)?;

        if self.resolve_variable(name).is_none() {
            return KindSet::from_type_map_name(name);
        }

        Some(KindSet::MAP | KindSet::CLASS)
    }

    pub(crate) fn narrowed_type(&mut self, slot: SlotId) -> TypeId {
        let ty = self.model.slot_type(slot);
        let kinds = self.narrow_stack.allowed_kinds(slot);

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

        let variants: Vec<TypeId> = self
            .types
            .variants(ty)
            .filter(|&variant| match self.types.get(variant) {
                Type::Any => true,
                other => KindSet::of(other) & kinds != KindSet::NONE,
            })
            .collect();

        self.types.union_all(variants)
    }

    pub(crate) fn infer_block_with_facts(&mut self, facts: &FactTable, body: &[NodeId]) {
        self.narrow_stack.push_frame(facts);
        self.infer_block(body);
        self.narrow_stack.pop_frame();
    }
}
