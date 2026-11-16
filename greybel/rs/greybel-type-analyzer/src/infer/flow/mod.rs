mod access_path;
mod guard;
mod side_effect;

use std::cmp::Ordering;
use std::mem;

use self::access_path::MemberFactList;
pub(crate) use self::access_path::{AccessPath, AccessPathTable};
pub(crate) use self::guard::{FactTable, Guard, KindSet};
use self::guard::{IsaTest, LiteralTest};
pub(crate) use self::side_effect::{SideEffect, SideEffectFrame};
use crate::infer::Inference;
use crate::model::{ScopeId, SlotId};
use crate::types::TypeId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Fact {
    pub assignment: Option<TypeId>,
    pub kinds: KindSet,
    pub isa: IsaTest,
    pub literal: LiteralTest,
}

impl Fact {
    const DECLARED_TYPE: Self = Self {
        assignment: None,
        kinds: KindSet::ALL,
        isa: IsaTest::Unknown,
        literal: LiteralTest::Unknown,
    };
    const NO_ASSIGNMENT: Self = Self {
        assignment: Some(TypeId::NEVER),
        kinds: KindSet::ALL,
        isa: IsaTest::Unknown,
        literal: LiteralTest::Unknown,
    };
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct FactList(Vec<(SlotId, Fact)>);

impl FactList {
    fn position(&self, slot: SlotId) -> Result<usize, usize> {
        self.0
            .binary_search_by_key(&slot, |&(candidate, _)| candidate)
    }

    fn get(&self, slot: SlotId) -> Option<Fact> {
        self.position(slot).ok().map(|index| self.0[index].1)
    }

    fn get_or_insert(&mut self, slot: SlotId, fact: Fact) -> &mut Fact {
        let index = match self.position(slot) {
            Ok(index) => index,
            Err(index) => {
                self.0.insert(index, (slot, fact));

                index
            }
        };

        &mut self.0[index].1
    }

    fn insert(&mut self, slot: SlotId, fact: Fact) {
        *self.get_or_insert(slot, fact) = fact;
    }

    fn remove(&mut self, slot: SlotId) {
        if let Ok(index) = self.position(slot) {
            self.0.remove(index);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FlowState {
    facts: FactList,
    members: MemberFactList,
    unassigned_by_default: bool,
    reachable: bool,
}

impl Default for FlowState {
    fn default() -> Self {
        Self {
            facts: FactList::default(),
            members: MemberFactList::default(),
            unassigned_by_default: false,
            reachable: true,
        }
    }
}

impl FlowState {
    pub(crate) fn program_start() -> Self {
        Self {
            unassigned_by_default: true,
            ..Self::default()
        }
    }

    pub(crate) fn unreachable() -> Self {
        Self {
            reachable: false,
            ..Self::default()
        }
    }

    fn default_fact(&self) -> Fact {
        if self.unassigned_by_default {
            Fact::NO_ASSIGNMENT
        } else {
            Fact::DECLARED_TYPE
        }
    }

    pub(crate) fn is_reachable(&self) -> bool {
        self.reachable
    }

    pub(crate) fn set_unreachable(&mut self) {
        self.reachable = false;
    }

    pub(crate) fn fact(&self, slot: SlotId) -> Option<Fact> {
        let fact = self.facts.get(slot).unwrap_or(self.default_fact());

        (fact != Fact::DECLARED_TYPE).then_some(fact)
    }

    pub(crate) fn is_unassigned(&self, slot: SlotId) -> bool {
        self.fact(slot)
            .is_some_and(|fact| fact.assignment == Some(TypeId::NEVER))
    }

    pub(crate) fn has_members(&self) -> bool {
        !self.members.is_empty()
    }

    pub(crate) fn clear_members(&mut self) {
        self.members.clear();
    }

    pub(crate) fn clear(&mut self, slot: SlotId) {
        self.members.clear_base(slot);

        if self.unassigned_by_default {
            self.facts.insert(slot, Fact::DECLARED_TYPE);
        } else {
            self.facts.remove(slot);
        }
    }

    pub(crate) fn clear_all(&mut self) {
        self.facts.0.clear();
        self.members.clear();
        self.unassigned_by_default = false;
    }

    /// A call the analyzer can't see into can assign members of any map, so member facts are cleared too.
    pub(crate) fn clear_all_except(&mut self, retain: impl Fn(SlotId) -> bool) {
        self.facts.0.retain(|&(slot, _)| retain(slot));
        self.members.clear();
        self.unassigned_by_default = false;
    }

    pub(crate) fn assign(&mut self, slot: SlotId, ty: TypeId) {
        self.members.clear_base(slot);
        self.facts.insert(
            slot,
            Fact {
                assignment: Some(ty),
                kinds: KindSet::ALL,
                isa: IsaTest::Unknown,
                literal: LiteralTest::Unknown,
            },
        );
    }
}

pub(crate) struct OuterBody {
    flow: FlowState,
    loops: Vec<LoopFrame>,
}

pub(crate) struct LoopFrame {
    pub breaks: Vec<FlowState>,
    pub continues: Vec<FlowState>,
}

impl Inference<'_> {
    pub(crate) fn enter_body(&mut self) -> OuterBody {
        OuterBody {
            flow: mem::take(&mut self.flow),
            loops: mem::take(&mut self.loop_stack),
        }
    }

    pub(crate) fn exit_body(&mut self, outer: OuterBody) {
        self.flow = outer.flow;
        self.loop_stack = outer.loops;
    }

    pub(crate) fn enter_loop(&mut self) {
        self.loop_stack.push(LoopFrame {
            breaks: Vec::new(),
            continues: Vec::new(),
        });
    }

    pub(crate) fn exit_loop(&mut self) -> LoopFrame {
        self.loop_stack.pop().expect("entered the loop")
    }

    pub(crate) fn exit_iteration(&mut self, is_break: bool) {
        let state = mem::replace(&mut self.flow, FlowState::unreachable());

        if let Some(frame) = self.loop_stack.last_mut() {
            if is_break {
                frame.breaks.push(state);
            } else {
                frame.continues.push(state);
            }
        }
    }

    pub(crate) fn narrowed_type(&mut self, slot: SlotId) -> TypeId {
        match self.flow.fact(slot) {
            Some(fact) => self.fact_type(slot, fact),
            None => self.model.slot_type(slot),
        }
    }

    pub(crate) fn unassign_variables(&mut self, scope: ScopeId, bound: &[SlotId]) {
        for (_, slot) in self.model.variables(scope) {
            if !bound.contains(&slot) {
                self.flow.facts.insert(slot, Fact::NO_ASSIGNMENT);
            }
        }
    }

    pub(crate) fn refine_flow(&mut self, facts: &FactTable) {
        let mut flow = mem::take(&mut self.flow);

        self.refine_state(&mut flow, facts);
        self.flow = flow;
    }

    pub(crate) fn with_facts(&mut self, state: &FlowState, facts: &FactTable) -> FlowState {
        let mut state = state.clone();

        self.refine_state(&mut state, facts);

        state
    }

    fn refine_state(&mut self, state: &mut FlowState, facts: &FactTable) {
        let default_fact = state.default_fact();

        for (path, restriction) in facts.iter() {
            let fact = match path.variable_slot() {
                Some(slot) => state.facts.get_or_insert(slot, default_fact),
                None => {
                    let id = self.access_paths.insert(path.clone());

                    state.members.get_or_insert(id, path)
                }
            };

            fact.kinds &= restriction.kinds;
            fact.isa = fact.isa.and(restriction.isa);
            fact.literal = fact.literal.and(restriction.literal, &mut self.types);
        }
    }

    pub(crate) fn fact_type(&mut self, slot: SlotId, fact: Fact) -> TypeId {
        let ty = fact
            .assignment
            .unwrap_or_else(|| self.model.slot_type(slot));

        self.restricted_type(ty, fact)
    }

    pub(crate) fn join_flows(&mut self, left: FlowState, right: &FlowState) -> FlowState {
        if !left.reachable {
            return right.clone();
        }

        if !right.reachable {
            return left;
        }

        let unassigned_by_default = left.unassigned_by_default && right.unassigned_by_default;
        let default_fact = if unassigned_by_default {
            Fact::NO_ASSIGNMENT
        } else {
            Fact::DECLARED_TYPE
        };
        let (left_default, right_default) = (left.default_fact(), right.default_fact());
        let mut facts = Vec::with_capacity(left.facts.0.len().max(right.facts.0.len()));
        let mut left_facts = left.facts.0.into_iter().peekable();
        let mut right_facts = right.facts.0.iter().copied().peekable();

        loop {
            let order = match (left_facts.peek(), right_facts.peek()) {
                (None, None) => break,
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (Some(&(left_slot, _)), Some(&(right_slot, _))) => left_slot.cmp(&right_slot),
            };
            let (slot, left_fact, right_fact) = match order {
                Ordering::Less => {
                    let (slot, fact) = left_facts.next().expect("peeked");

                    (slot, fact, right_default)
                }
                Ordering::Greater => {
                    let (slot, fact) = right_facts.next().expect("peeked");

                    (slot, left_default, fact)
                }
                Ordering::Equal => {
                    let (slot, left_fact) = left_facts.next().expect("peeked");
                    let (_, right_fact) = right_facts.next().expect("peeked");

                    (slot, left_fact, right_fact)
                }
            };
            let fact = if left_fact == right_fact {
                left_fact
            } else {
                self.join_facts(slot, left_fact, right_fact)
            };

            if fact != default_fact {
                facts.push((slot, fact));
            }
        }

        let facts = FactList(facts);
        let members = self.join_members(left.members, &right.members);

        FlowState {
            facts,
            members,
            unassigned_by_default,
            reachable: true,
        }
    }

    pub(super) fn restricted_type(&mut self, ty: TypeId, fact: Fact) -> TypeId {
        let ty = self.restrict_to_kinds(ty, fact.kinds);
        let ty = self.restrict_to_isa(ty, fact.isa);

        self.restrict_to_literals(ty, fact.literal)
    }

    fn join_facts(&mut self, slot: SlotId, left: Fact, right: Fact) -> Fact {
        if left == Fact::DECLARED_TYPE || right == Fact::DECLARED_TYPE {
            return Fact::DECLARED_TYPE;
        }

        if left.assignment.is_none() && right.assignment.is_none() {
            return Fact {
                assignment: None,
                kinds: left.kinds | right.kinds,
                isa: left.isa.or(right.isa),
                literal: left.literal.or(right.literal, &mut self.types),
            };
        }

        let left_type = self.fact_type(slot, left);
        let right_type = self.fact_type(slot, right);
        let union = self.types.union(left_type, right_type);

        Fact {
            assignment: Some(self.merge_generic_instances(union)),
            kinds: KindSet::ALL,
            isa: IsaTest::Unknown,
            literal: LiteralTest::Unknown,
        }
    }

    pub(crate) fn join_all_flows(
        &mut self,
        states: impl IntoIterator<Item = FlowState>,
    ) -> FlowState {
        let mut states = states.into_iter();
        let Some(mut state_union) = states.next() else {
            return FlowState::unreachable();
        };

        for state in states {
            state_union = self.join_flows(state_union, &state);
        }

        state_union
    }

    pub(crate) fn widen_fact(&mut self, slot: SlotId, ty: TypeId) {
        self.flow.members.clear_base(slot);

        let Some(Fact {
            assignment: Some(assignment),
            kinds,
            isa,
            literal,
        }) = self.flow.fact(slot)
        else {
            return;
        };
        let union = self.types.union(assignment, ty);
        let wider = self.merge_generic_instances(union);

        self.flow.facts.insert(
            slot,
            Fact {
                assignment: Some(wider),
                kinds,
                isa,
                literal,
            },
        );
    }
}
