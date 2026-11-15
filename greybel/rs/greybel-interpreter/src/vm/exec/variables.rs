use greybel_symbols::names::SELF;

use super::super::Vm;
use super::super::call::CallContext;
use super::super::implementation::Implementation;
use crate::bytecode::{Chain, EntryCache, Local, LookupCache, Operand, Operator, Scope, Variable};
use crate::error::{RuntimeError, RuntimeErrorKind};
use crate::value::{Key, Map, Value};

impl Vm {
    #[inline(always)]
    fn scope_chain(&self) -> [Option<Map>; 4] {
        let frame = self.frame();
        let globals = self.scopes.globals;
        let outer = frame.outer.filter(|&outer| Some(outer) != frame.locals);
        let globals = Some(globals)
            .filter(|&globals| Some(globals) != frame.locals && Some(globals) != outer);

        [frame.locals, outer, globals, Some(self.scopes.api)]
    }

    #[inline(never)]
    pub(in crate::vm) fn lookup(&self, key: &Key) -> Result<Value, RuntimeError> {
        self.scope_chain()
            .into_iter()
            .flatten()
            .find_map(|scope| scope.get_key(&self.heap, key))
            .cloned()
            .map_or_else(|| Self::missing_variable(key), Ok)
    }

    #[cold]
    fn missing_variable(key: &Key) -> Result<Value, RuntimeError> {
        if key.text() == SELF {
            return Ok(Value::Null);
        }

        Err(RuntimeError::from(RuntimeErrorKind::UndefinedIdentifier(
            key.text().to_owned(),
        )))
    }

    #[inline(always)]
    fn insertions_before(&self, scope: Map) -> Option<u32> {
        let mut insertions = 0u32;

        for before in self.scope_chain() {
            let Some(before) = before else {
                continue;
            };

            if before == scope {
                return Some(insertions);
            }

            let before_insertions = before.insertions(&self.heap);

            insertions = insertions
                .checked_add(before_insertions)
                .filter(|&sum| sum < u32::MAX)?;
        }

        None
    }

    #[inline(never)]
    fn scope_variable(&self, key: &Key, scope: Map) -> Value {
        self.frame()
            .locals
            .and_then(|locals| locals.get_key(&self.heap, key))
            .cloned()
            .unwrap_or(Value::Map(scope))
    }

    #[inline(never)]
    pub(super) fn local_super<I: Implementation>(&self) -> Result<Value, RuntimeError> {
        let key = &self.keys.super_key.value;
        let frame = self.frame();
        let Some(base) = frame.slot_base else {
            return self
                .property::<I>(&Value::Map(self.locals_map()), key)
                .map(|(value, _)| value);
        };
        let slots = self.program.code(frame.code).slots.as_ref();
        let slot = slots.and_then(|slots| slots.super_slot);

        match slot.and_then(|slot| self.calls.slots[base + slot as usize].as_ref()) {
            Some(value) => Ok(value.clone()),
            None => self.map_intrinsic(key).map(|(value, _)| value),
        }
    }

    #[inline(always)]
    fn push_or_invoke(
        &mut self,
        value: Value,
        invoke: bool,
        discard: bool,
    ) -> Result<(), RuntimeError> {
        if invoke && matches!(value, Value::Function(_)) {
            return self.invoke(value, 0, None, discard);
        }

        self.push_unless(discard, value);

        Ok(())
    }

    fn push_or_call(
        &mut self,
        value: Value,
        call: Option<u32>,
        receiver: impl FnOnce(&Self) -> Option<CallContext>,
        discard: bool,
    ) -> Result<(), RuntimeError> {
        match call {
            Some(count) if matches!(value, Value::Function(_)) => {
                let receiver = receiver(self);

                self.invoke(value, count as usize, receiver, discard)
            }
            Some(count) if count > 0 => self.invoke(value, count as usize, None, discard),
            _ => {
                self.push_unless(discard, value);

                Ok(())
            }
        }
    }

    #[inline(always)]
    pub(super) fn variable(&self, key: &Key, cache: &LookupCache) -> Option<&Value> {
        let frame = self.frame();
        let first = frame.locals.or(frame.outer).unwrap_or(self.scopes.globals);

        if let Some((scope, index)) = cache.entry()
            && (scope == first
                || cache
                    .chain()
                    .is_some_and(|chain| self.is_unshadowed(scope, chain)))
            && let Some(value) = scope.value_at(&self.heap, index, key)
        {
            return Some(value);
        }

        if let Some(index) = first.key_index(&self.heap, key) {
            cache.set(first, index, None);

            return first.value_at(&self.heap, index, key);
        }

        self.find_variable(key, cache)
    }

    #[inline(never)]
    fn is_unshadowed(&self, scope: Map, chain: Chain) -> bool {
        let frame = self.frame();

        chain.scope == scope
            && chain.locals == frame.locals
            && chain.outer == frame.outer
            && self.insertions_before(scope) == Some(chain.insertions)
    }

    #[inline(never)]
    fn find_variable(&self, key: &Key, cache: &LookupCache) -> Option<&Value> {
        let frame = self.frame();

        for scope in self.scope_chain().into_iter().flatten().skip(1) {
            let Some(index) = scope.key_index(&self.heap, key) else {
                continue;
            };

            if let Some(insertions) = self.insertions_before(scope) {
                let chain = Chain {
                    scope,
                    locals: frame.locals,
                    outer: frame.outer,
                    insertions,
                };

                cache.set(scope, index, Some(chain));
            }

            return scope.value_at(&self.heap, index, key);
        }

        None
    }

    pub(super) fn load_variable(
        &mut self,
        key: &Key,
        invoke: bool,
        discard: bool,
        cache: &LookupCache,
    ) -> Result<(), RuntimeError> {
        let value = match self.variable(key, cache) {
            Some(value) => value.clone(),
            None => Self::missing_variable(key)?,
        };

        self.push_or_invoke(value, invoke, discard)
    }

    pub(super) fn load_property<I: Implementation>(
        &mut self,
        call: Option<u32>,
        discard: bool,
    ) -> Result<(), RuntimeError> {
        let count = call.unwrap_or(0) as usize;
        let key = self.take_below(count);
        let base = self.take_below(count);
        let (value, origin) = self.property::<I>(&base, &key)?;
        let receiver = |vm: &Self| Some(CallContext::new(Some(base), vm.super_of(origin)));

        self.push_or_call(value, call, receiver, discard)
    }

    pub(super) fn load_member<I: Implementation>(
        &mut self,
        key: &Key,
        call: Option<u32>,
        discard: bool,
    ) -> Result<(), RuntimeError> {
        let base = self.take_below(call.unwrap_or(0) as usize);
        let (value, origin) = self.own_property::<I>(&base, key)?;
        let receiver = |vm: &Self| Some(CallContext::new(Some(base), vm.super_of(origin)));

        self.push_or_call(value, call, receiver, discard)
    }

    /// Current `self`, or null when the function was called without a receiver.
    fn local_self(&self) -> Value {
        let frame = self.frame();
        let Some(base) = frame.slot_base else {
            return self
                .locals_map()
                .get_key(&self.heap, &self.keys.self_key)
                .cloned()
                .unwrap_or(Value::Null);
        };
        let slots = self.program.code(frame.code).slots.as_ref();
        let slot = slots.and_then(|slots| slots.self_slot);

        slot.and_then(|slot| self.calls.slots[base + slot as usize].clone())
            .unwrap_or(Value::Null)
    }

    /// Reads `self.key`, or the stack-named member when `key` is absent. Without a receiver, Grey Hack reads null
    /// because `ValSeqElem.Val` finds no `self`, then discards call arguments. Upstream MiniScript instead reports an
    /// unknown variable.
    #[inline(never)]
    pub(super) fn load_self_property<I: Implementation>(
        &mut self,
        key: Option<&Key>,
        call: Option<u32>,
        discard: bool,
    ) -> Result<(), RuntimeError> {
        let count = call.unwrap_or(0) as usize;
        let context = self.local_self();

        if context.is_null() && !I::READS_MISSING_SELF {
            return Err(RuntimeError::from(RuntimeErrorKind::UndefinedIdentifier(
                SELF.to_owned(),
            )));
        }

        if context.is_null() {
            let below = count + usize::from(key.is_none());

            for _ in 0..below {
                self.pop();
            }

            self.push_unless(discard, Value::Null);

            return Ok(());
        }

        let (value, origin) = match key {
            Some(key) => self.own_property::<I>(&context, key)?,
            None => {
                let key = self.take_below(count);

                self.property::<I>(&context, &key)?
            }
        };
        let receiver = |vm: &Self| Some(CallContext::new(Some(context), vm.super_of(origin)));

        self.push_or_call(value, call, receiver, discard)
    }

    #[inline(never)]
    pub(super) fn load_super_property<I: Implementation>(
        &mut self,
        key: Option<&Key>,
        call: Option<u32>,
        discard: bool,
    ) -> Result<(), RuntimeError> {
        let context = self.local_super::<I>()?;
        let (value, origin) = match key {
            Some(key) => self.own_property::<I>(&context, key)?,
            None => {
                let key = self.take_below(call.unwrap_or(0) as usize);

                self.property::<I>(&context, &key)?
            }
        };
        let receiver = |vm: &Self| {
            let self_value = vm.frame().self_value.clone();

            Some(CallContext::new(self_value, vm.super_of(origin)))
        };

        self.push_or_call(value, call, receiver, discard)
    }

    #[inline(never)]
    pub(super) fn load_scope(&mut self, scope: Scope) {
        let value = match scope {
            Scope::Outer => {
                let outer = self.frame().outer.unwrap_or(self.scopes.globals);

                self.scope_variable(&self.keys.outer, outer)
            }
            Scope::Locals => self.scope_variable(&self.keys.locals, self.locals_map()),
            Scope::Globals => self.scope_variable(&self.keys.globals, self.scopes.globals),
        };

        self.stack.push(value);
    }

    pub(super) fn assign(&mut self) -> Result<(), RuntimeError> {
        let value = self.pop();
        let key = self.pop();
        let base = self.pop();

        self.set_property(&base, key, value)
    }

    pub(super) fn assign_local(&mut self, key: &Key, cache: &EntryCache) {
        let value = self.pop();

        self.store_local(key, value, cache);
    }

    #[inline(always)]
    pub(super) fn local(&self, local: &Local) -> Option<&Value> {
        let locals = self.locals_map();

        if let Some((map, index)) = local.cache.get()
            && map == locals
            && let Some(value) = map.value_at(&self.heap, index, &local.key)
        {
            return Some(value);
        }

        let index = locals.key_index(&self.heap, &local.key)?;

        local.cache.set(locals, index);

        locals.value_at(&self.heap, index, &local.key)
    }

    pub(super) fn store_local(&mut self, key: &Key, value: Value, cache: &EntryCache) {
        let locals = self.locals_map();

        if let Some((map, index)) = cache.get()
            && map == locals
            && map.value_at(&self.heap, index, key).is_some()
        {
            map.set_at(&mut self.heap, index, value);

            return;
        }

        let index = locals.set_key_at(&mut self.heap, key, value);

        cache.set(locals, index);
    }

    pub(super) fn assign_local_binary(
        &mut self,
        key: &Key,
        operator: Operator,
        right: &Value,
        cache: &EntryCache,
    ) {
        let a = self.pop();
        let value = self.operate(operator, &a, right);

        self.store_local(key, value, cache);
    }

    pub(super) fn load_slot(
        &mut self,
        slot: u32,
        key: &Key,
        invoke: bool,
        discard: bool,
    ) -> Result<(), RuntimeError> {
        let value = match self.slot_mut(slot) {
            Some(value) => value.clone(),
            None => self.lookup(key)?,
        };

        self.push_or_invoke(value, invoke, discard)
    }

    pub(super) fn assign_slot(&mut self, slot: u32, value: &Operand) {
        let value = match value {
            Operand::Stack => self.pop(),
            Operand::Value(value) => value.clone(),
            Operand::Slot(from) => match self.slot_value(*from) {
                Some(value) => value.clone(),
                None => return self.fall_back(),
            },
        };

        *self.slot_mut(slot) = Some(value);
    }

    pub(super) fn copy_variable(&mut self, from: &Variable, to: &Local) {
        match self.variable_value(from) {
            Some(value) => {
                let value = value.clone();

                self.store_local(&to.key, value, &to.cache);
            }
            None => self.fall_back(),
        }
    }

    pub(super) fn assign_member(&mut self, key: &Key) -> Result<(), RuntimeError> {
        let value = self.pop();

        match self.pop() {
            Value::Map(map) if !map.is_sealed(&self.heap) => {
                map.set_key(&mut self.heap, key, value);

                Ok(())
            }
            base => self.set_property(&base, key.value.clone(), value),
        }
    }
}
