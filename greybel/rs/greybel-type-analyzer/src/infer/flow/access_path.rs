use std::mem;

use greybel_core::{LiteralValue, NodeId, NodeKind};
use greybel_symbols::names::SELF;
use rustc_hash::FxHashMap;

use super::Fact;
use super::guard::{IsaTest, KindSet, LiteralTest};
use crate::infer::Inference;
use crate::model::{NameId, ScopeId, SlotId};
use crate::types::{Type, TypeId};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct AccessPath {
    base: SlotId,
    members: Box<[NameId]>,
}

impl AccessPath {
    pub(crate) fn variable(slot: SlotId) -> Self {
        Self {
            base: slot,
            members: Box::new([]),
        }
    }

    pub(crate) fn variable_slot(&self) -> Option<SlotId> {
        self.members.is_empty().then_some(self.base)
    }

    pub(crate) fn with_member(mut self, name: NameId) -> Self {
        let mut members = mem::take(&mut self.members).into_vec();

        members.push(name);
        self.members = members.into_boxed_slice();

        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct AccessPathId(u32);

#[derive(Debug, Default)]
pub(crate) struct AccessPathTable {
    ids: FxHashMap<AccessPath, AccessPathId>,
    paths: Vec<AccessPath>,
}

impl AccessPathTable {
    pub(crate) fn insert(&mut self, path: AccessPath) -> AccessPathId {
        if let Some(&id) = self.ids.get(&path) {
            return id;
        }

        let id =
            AccessPathId(u32::try_from(self.paths.len()).expect("fewer than 2^32 access paths"));

        self.paths.push(path.clone());
        self.ids.insert(path, id);

        id
    }

    fn id(&self, path: &AccessPath) -> Option<AccessPathId> {
        self.ids.get(path).copied()
    }

    fn members(&self, id: AccessPathId) -> &[NameId] {
        &self.paths[id.0 as usize].members
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NameMask(u64);

impl NameMask {
    fn bit(name: NameId) -> Self {
        Self(1 << (name.index() % 64))
    }

    fn of(names: &[NameId]) -> Self {
        Self(names.iter().fold(0, |mask, &name| mask | Self::bit(name).0))
    }

    const fn overlaps(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MemberFact {
    path: AccessPathId,
    base: SlotId,
    last: NameId,
    names: NameMask,
    fact: Fact,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct MemberFactList(Vec<MemberFact>);

impl MemberFactList {
    fn position(&self, path: AccessPathId) -> Result<usize, usize> {
        self.0.binary_search_by_key(&path, |member| member.path)
    }

    fn get(&self, path: AccessPathId) -> Option<Fact> {
        self.position(path).ok().map(|index| self.0[index].fact)
    }

    pub(super) fn get_or_insert(
        &mut self,
        path: AccessPathId,
        access_path: &AccessPath,
    ) -> &mut Fact {
        let index = match self.position(path) {
            Ok(index) => index,
            Err(index) => {
                let members = &access_path.members;

                self.0.insert(
                    index,
                    MemberFact {
                        path,
                        base: access_path.base,
                        last: *members.last().expect("a member access path"),
                        names: NameMask::of(members),
                        fact: Fact::DECLARED_TYPE,
                    },
                );

                index
            }
        };

        &mut self.0[index].fact
    }

    pub(super) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(super) fn clear(&mut self) {
        self.0.clear();
    }

    pub(super) fn clear_base(&mut self, slot: SlotId) {
        self.0.retain(|member| member.base != slot);
    }

    pub(crate) fn clear_name(&mut self, name: NameId, paths: &AccessPathTable) {
        let bit = NameMask::bit(name);

        self.0.retain(|member| {
            !member.names.overlaps(bit) || !paths.members(member.path).contains(&name)
        });
    }

    fn ends_with(&self, name: NameId) -> bool {
        self.0.iter().any(|member| member.last == name)
    }
}

impl Inference<'_> {
    pub(crate) fn member_access_path(&mut self, node: NodeId) -> Option<AccessPath> {
        let ast = self.ast();
        let (base, name) = match ast.kind(node) {
            NodeKind::Member(member) => (member.base, self.identifier_name_id(member.identifier)?),
            NodeKind::Index(index) => {
                let NodeKind::Literal(literal) = ast.kind(index.index) else {
                    return None;
                };
                let LiteralValue::String(text) = &literal.value else {
                    return None;
                };

                (index.base, self.model.insert_name(&ast.read(*text)))
            }
            NodeKind::Parenthesis(parenthesis) => {
                return self.member_access_path(parenthesis.expression);
            }
            _ => return None,
        };

        if self.scope_for_name(base).is_some() {
            return None;
        }

        let mut path = match ast.kind(base) {
            NodeKind::Identifier(identifier) => AccessPath::variable(self.path_base(
                self.atom_name(identifier.name),
                ast.resolve(identifier.name),
            )?),
            _ => self.member_access_path(base)?,
        };
        let mut members = mem::take(&mut path.members).into_vec();

        members.push(name);
        path.members = members.into_boxed_slice();

        Some(path)
    }

    fn path_base(&self, name_id: NameId, name: &str) -> Option<SlotId> {
        if let Some(slot) = self.resolve_variable(name_id) {
            return Some(slot);
        }

        match self.current_scope {
            ScopeId::Function(function) if name == SELF => {
                Some(self.model.function(function).receiver_slot)
            }
            _ => None,
        }
    }

    pub(crate) fn member_fact(&mut self, node: NodeId) -> Option<(AccessPath, Fact)> {
        if self.flow.members.is_empty() {
            return None;
        }

        let ast = self.ast();
        let name = match ast.kind(node) {
            NodeKind::Member(member) => self.identifier_name_id(member.identifier),
            NodeKind::Index(index) => match ast.kind(index.index) {
                NodeKind::Literal(literal) => match &literal.value {
                    LiteralValue::String(text) => self.model.name_id(&ast.read(*text)),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        };

        if !self.flow.members.ends_with(name?) {
            return None;
        }

        let path = self.member_access_path(node)?;
        let fact = self.flow.members.get(self.access_paths.id(&path)?)?;

        Some((path, fact))
    }

    pub(crate) fn member_fact_type(&mut self, node: NodeId, ty: TypeId) -> TypeId {
        let Some((_, fact)) = self.member_fact(node) else {
            return ty;
        };

        match fact.assignment {
            Some(assignment) => self.restricted_type(assignment, fact),
            None if self
                .types
                .variants(ty)
                .any(|variant| matches!(self.types.get(variant), Type::Function(_))) =>
            {
                ty
            }
            None => self.restricted_type(ty, fact),
        }
    }

    pub(crate) fn assign_member(&mut self, target: NodeId, name: Option<NameId>, value: TypeId) {
        let Some(name) = name else {
            self.flow.members.clear();
            self.add_member_assignment(None);

            return;
        };

        self.flow.members.clear_name(name, &self.access_paths);
        self.add_member_assignment(Some(name));

        if value == TypeId::UNKNOWN {
            return;
        }

        if let Some(path) = self.member_access_path(target) {
            self.set_member_fact(&path, value);
        }
    }

    fn set_member_fact(&mut self, path: &AccessPath, ty: TypeId) {
        let id = self.access_paths.insert(path.clone());

        *self.flow.members.get_or_insert(id, path) = Fact {
            assignment: Some(ty),
            kinds: KindSet::ALL,
            isa: IsaTest::Unknown,
            literal: LiteralTest::Unknown,
        };
    }

    pub(crate) fn widen_member_fact(&mut self, node: NodeId, ty: TypeId) {
        let Some((
            path,
            Fact {
                assignment: Some(assignment),
                ..
            },
        )) = self.member_fact(node)
        else {
            return;
        };
        let union = self.types.union(assignment, ty);
        let wider = self.merge_generic_instances(union);

        self.set_member_fact(&path, wider);
    }

    pub(super) fn join_members(
        &mut self,
        left: MemberFactList,
        right: &MemberFactList,
    ) -> MemberFactList {
        if left.0.is_empty() || right.0.is_empty() {
            return MemberFactList::default();
        }

        let members = left
            .0
            .into_iter()
            .filter_map(|member| {
                let right_fact = right.get(member.path)?;
                let fact = self.join_member_facts(member.fact, right_fact)?;

                Some(MemberFact { fact, ..member })
            })
            .collect();

        MemberFactList(members)
    }

    fn join_member_facts(&mut self, left: Fact, right: Fact) -> Option<Fact> {
        if left == right {
            return Some(left);
        }

        match (left.assignment, right.assignment) {
            (None, None) => Some(Fact {
                assignment: None,
                kinds: left.kinds | right.kinds,
                isa: left.isa.or(right.isa),
                literal: left.literal.or(right.literal, &mut self.types),
            }),
            (Some(left_type), Some(right_type)) => {
                let left_type = self.restricted_type(left_type, left);
                let right_type = self.restricted_type(right_type, right);
                let union = self.types.union(left_type, right_type);

                Some(Fact {
                    assignment: Some(self.merge_generic_instances(union)),
                    kinds: KindSet::ALL,
                    isa: IsaTest::Unknown,
                    literal: LiteralTest::Unknown,
                })
            }
            _ => None,
        }
    }
}
