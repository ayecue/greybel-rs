use rustc_hash::{FxHashMap, FxHashSet};

use crate::infer::Inference;
use crate::model::{DocumentIndex, NameId, ScopeId, Site, SlotId};
use crate::types::FunctionRef;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SideEffect {
    outer_slots: FxHashSet<SlotId>,
    any_variable: bool,
    member_names: FxHashSet<NameId>,
    any_member: bool,
    callees: FxHashSet<Site>,
}

impl SideEffect {
    fn extend(&mut self, other: &Self) {
        self.outer_slots.extend(&other.outer_slots);
        self.any_variable |= other.any_variable;
        self.member_names.extend(&other.member_names);
        self.any_member |= other.any_member;
    }

    fn same_variables(&self, other: &Self) -> bool {
        self.outer_slots == other.outer_slots && self.any_variable == other.any_variable
    }
}

#[derive(Debug, Default)]
pub(crate) struct SideEffectTable(FxHashMap<Site, SideEffect>);

impl SideEffectTable {
    fn get(&self, site: Site) -> Option<&SideEffect> {
        self.0.get(&site)
    }

    fn add(&mut self, site: Site, side_effect: &SideEffect) {
        self.0.entry(site).or_default().extend(side_effect);
    }

    fn same_variables(&self, other: &Self) -> bool {
        self.0.len() == other.0.len()
            && self.0.iter().all(|(site, left)| {
                other
                    .0
                    .get(site)
                    .is_some_and(|right| left.same_variables(right))
            })
    }

    fn close_member_writes(&mut self) {
        let side_effects = &mut self.0;
        let sites: Vec<Site> = side_effects.keys().copied().collect();
        let mut names: Vec<NameId> = Vec::new();
        let mut changed = true;

        while changed {
            changed = false;

            for &site in &sites {
                let mut any_member = false;

                names.clear();

                for callee in &side_effects[&site].callees {
                    if let Some(callee) = side_effects.get(callee) {
                        names.extend(&callee.member_names);
                        any_member |= callee.any_member;
                    }
                }

                let side_effect = side_effects.get_mut(&site).expect("a site of the map");
                let count = side_effect.member_names.len();

                side_effect.member_names.extend(names.drain(..));
                changed |= side_effect.member_names.len() != count
                    || any_member && !side_effect.any_member;
                side_effect.any_member |= any_member;
            }
        }
    }
}

pub(crate) struct SideEffectFrame {
    scope: Option<ScopeId>,
    side_effect: SideEffect,
}

impl Inference<'_> {
    pub(crate) fn begin_side_effect(&mut self, scope: Option<ScopeId>) {
        self.side_effect_stack.push(SideEffectFrame {
            scope,
            side_effect: SideEffect::default(),
        });
    }

    fn end_side_effect(&mut self) -> SideEffect {
        self.side_effect_stack
            .pop()
            .expect("began the side effect")
            .side_effect
    }

    pub(crate) fn end_function_side_effect(&mut self, site: Site) {
        let side_effect = self.end_side_effect();

        self.pass.function_side_effects.add(site, &side_effect);
    }

    pub(crate) fn end_document_side_effect(&mut self, document: DocumentIndex) -> SideEffect {
        let side_effect = self.end_side_effect();

        self.extend_side_effect(&side_effect);
        self.pass
            .document_side_effects
            .insert(document, side_effect.clone());

        side_effect
    }

    pub(crate) fn document_side_effect(&self, document: DocumentIndex) -> Option<SideEffect> {
        self.pass.document_side_effects.get(&document).cloned()
    }

    pub(crate) fn finish_pass_side_effects(&mut self) {
        if !self
            .pass
            .function_side_effects
            .same_variables(&self.previous.function_side_effects)
        {
            self.pass.has_changes = true;
        }

        self.previous.function_side_effects = std::mem::take(&mut self.pass.function_side_effects);
        self.previous.function_side_effects.close_member_writes();
    }

    pub(crate) fn add_assignment(&mut self, scope: ScopeId, slot: SlotId) {
        if let Some(frame) = self.side_effect_stack.last_mut()
            && frame.scope != Some(scope)
        {
            frame.side_effect.outer_slots.insert(slot);
        }
    }

    pub(crate) fn add_member_assignment(&mut self, name: Option<NameId>) {
        let Some(frame) = self.side_effect_stack.last_mut() else {
            return;
        };

        match name {
            Some(name) => {
                frame.side_effect.member_names.insert(name);
            }
            None => frame.side_effect.any_member = true,
        }
    }

    pub(crate) fn add_unknown_assignment(&mut self) {
        self.clear_outer_facts();

        if let Some(frame) = self.side_effect_stack.last_mut() {
            frame.side_effect.any_variable = true;
        }
    }

    pub(crate) fn apply_call_side_effect(&mut self, function: FunctionRef) {
        let site = match function {
            FunctionRef::Any => return self.add_unknown_assignment(),
            FunctionRef::Body(id) => self.model.function(id).site,
            FunctionRef::Api(_) | FunctionRef::ApiMethod(_) | FunctionRef::Signature(_) => return,
        };

        if let Some(frame) = self.side_effect_stack.last_mut() {
            frame.side_effect.callees.insert(site);
        }

        let any_variable = match self
            .pass
            .function_side_effects
            .get(site)
            .or_else(|| self.previous.function_side_effects.get(site))
        {
            Some(side_effect) => side_effect.any_variable,
            None => return,
        };

        if any_variable {
            self.add_unknown_assignment();
        }

        let current = self.pass.function_side_effects.get(site);
        let previous = self.previous.function_side_effects.get(site);

        if !any_variable && let Some(variables) = current.or(previous) {
            for &slot in &variables.outer_slots {
                self.flow.clear(slot);
            }

            if let Some(frame) = self.side_effect_stack.last_mut() {
                frame.side_effect.outer_slots.extend(&variables.outer_slots);
            }
        }

        if !self.flow.has_members() {
            return;
        }

        for side_effect in [current, previous].into_iter().flatten() {
            if side_effect.any_member {
                return self.flow.clear_members();
            }

            for &name in &side_effect.member_names {
                self.flow.members.clear_name(name, &self.access_paths);
            }
        }
    }

    pub(crate) fn apply_side_effect(&mut self, side_effect: &SideEffect) {
        if side_effect.any_variable {
            self.clear_outer_facts();
        } else {
            for &slot in &side_effect.outer_slots {
                self.flow.clear(slot);
            }
        }

        if side_effect.any_member {
            self.flow.clear_members();
        } else {
            for &name in &side_effect.member_names {
                self.flow.members.clear_name(name, &self.access_paths);
            }
        }

        self.extend_side_effect(side_effect);
    }

    fn clear_outer_facts(&mut self) {
        let ScopeId::Function(function) = self.current_scope else {
            return self.flow.clear_all();
        };

        if self.binding_use(function).shares_locals {
            return self.flow.clear_all();
        }

        let count = self.model.variable_count(self.current_scope);
        let own = match self.own_slots.get(&function) {
            Some(own) if own.len() == count => own,
            _ => {
                let mut own: Vec<SlotId> = self
                    .model
                    .variables(self.current_scope)
                    .map(|(_, slot)| slot)
                    .collect();

                own.sort_unstable();

                self.own_slots.entry(function).insert_entry(own).into_mut()
            }
        };

        self.flow
            .clear_all_except(|slot| own.binary_search(&slot).is_ok());
    }

    fn extend_side_effect(&mut self, side_effect: &SideEffect) {
        if let Some(frame) = self.side_effect_stack.last_mut() {
            frame.side_effect.extend(side_effect);
        }
    }
}
