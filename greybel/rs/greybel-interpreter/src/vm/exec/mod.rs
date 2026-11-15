mod calls;
mod control;
mod operands;
mod operators;
mod variables;

use std::rc::Rc;

use super::implementation::{GreyHack, MiniScript};
use super::native::Completion;
use super::scheduler::Scheduler;
use super::{ContextExit, RunResult, State, Vm};
use crate::bytecode::Op;
use crate::error::RuntimeError;

impl Vm {
    pub(crate) async fn run(
        &mut self,
        scheduler: &mut impl Scheduler,
    ) -> Result<RunResult, RuntimeError> {
        let outcome = self.execute(scheduler).await;

        self.host.stdout.flush();

        outcome
    }

    async fn execute(&mut self, scheduler: &mut impl Scheduler) -> Result<RunResult, RuntimeError> {
        if self.state == State::Pause {
            self.state = State::Active;
        }

        loop {
            if let Err(error) = self.await_pending_native().await {
                self.exit_or_fail(error)?;

                continue;
            }

            match self.state {
                State::Complete if self.has_parent() => {
                    self.pop_context(ContextExit::Completion);

                    continue;
                }
                State::Complete => return Ok(RunResult::Complete),
                State::Failure => {
                    let error = self.take_failure();

                    self.exit_or_fail(error)?;

                    continue;
                }
                State::Pause => {
                    return Ok(RunResult::Breakpoint(self.frame_location(self.frame())));
                }
                State::Active | State::Pending { .. } | State::Yield => {}
            }

            if scheduler.should_collect() {
                self.collect_garbage();
            }

            let op_count = match self.run_batch(scheduler.batch_size().max(1)) {
                Ok(op_count) => op_count,
                Err(error) => {
                    self.exit_or_fail(error)?;

                    continue;
                }
            };

            if self.state == State::Yield {
                self.state = State::Active;
                self.yield_to(scheduler).await;
            } else if self.state == State::Active && scheduler.should_yield(op_count) {
                self.yield_to(scheduler).await;
            }
        }
    }

    pub(super) async fn yield_to(&self, scheduler: &mut impl Scheduler) {
        self.host.stdout.flush_if_due();
        scheduler.yield_now().await;
    }

    pub(super) async fn await_pending_native(&mut self) -> Result<(), RuntimeError> {
        let State::Pending { discard } = self.state else {
            return Ok(());
        };

        self.host.stdout.flush();

        let future = self
            .pending_call
            .as_mut()
            .expect("a waiting call has a future");
        let output = future.await;

        self.pending_call = None;

        let result = output.finish(self);

        match result {
            Ok(Completion::Value(_)) if self.state == State::Complete => Ok(()),
            Ok(Completion::Value(value)) => {
                self.push_unless(discard, value);
                self.state = State::Active;

                Ok(())
            }
            Ok(Completion::Context(entry)) => {
                self.push_context(entry, discard);

                Ok(())
            }
            Err(mut error) => {
                error.stack = self.stack_trace();
                self.state = State::Complete;

                Err(error)
            }
        }
    }

    pub(super) fn run_batch(&mut self, budget: usize) -> Result<usize, RuntimeError> {
        if self.heap.last_allocation_size() >= self.collector.next {
            self.collect_garbage();

            if let Err(mut error) = self.check_memory() {
                error.stack = self.stack_trace();
                self.state = State::Complete;

                return Err(error);
            }
        }

        let program = Rc::clone(&self.program);

        for op_count in 0..budget {
            if !matches!(self.state, State::Active) {
                return Ok(op_count);
            }

            let frame = self.frame();
            let code = program.code(frame.code);
            let ip = frame.ip;
            let Some(op) = code.ops.get(ip) else {
                if self.calls.frames.len() > 1 {
                    self.pop_frame();
                } else {
                    self.state = State::Complete;
                }

                continue;
            };
            let frame = self.frame_mut();

            frame.ip = ip + 1;
            frame.current = ip;
            if let Err(mut error) = self.step(op) {
                error.stack = self.stack_trace();
                self.state = State::Complete;

                return Err(error);
            }
        }

        Ok(budget)
    }

    #[cold]
    #[inline(never)]
    fn exit_or_fail(&mut self, error: RuntimeError) -> Result<(), RuntimeError> {
        if !self.has_parent() {
            return Err(error);
        }

        self.pop_context(ContextExit::Failure(error));

        Ok(())
    }

    #[cold]
    #[inline(never)]
    pub(super) fn raise(&mut self, error: RuntimeError) {
        self.failure = Some(error);
        self.state = State::Failure;
    }

    #[cold]
    #[inline(never)]
    pub(super) fn take_failure(&mut self) -> RuntimeError {
        let mut error = self.failure.take().expect("a failed run keeps its error");

        error.stack = self.stack_trace();
        self.state = State::Complete;

        error
    }

    #[inline(always)]
    #[expect(
        clippy::too_many_lines,
        reason = "The hot dispatch match stays in one forced-inline function; extracting arm groups adds call boundaries"
    )]
    fn step(&mut self, op: &Op) -> Result<(), RuntimeError> {
        match op {
            Op::Push(value) => self.stack.push(value.clone()),
            Op::Pop => {
                self.pop();
            }
            Op::Duplicate(count) => self.duplicate(usize::from(*count)),
            Op::GetVariable {
                key,
                invoke,
                discard,
                cache,
            } => return self.load_variable(key, *invoke, *discard, cache),
            Op::GetProperty { call, discard } => {
                return self.load_property::<GreyHack>(*call, *discard);
            }
            Op::GetMember { key, call, discard } => {
                return self.load_member::<GreyHack>(key, *call, *discard);
            }
            Op::GetSelfProperty { key, call, discard } => {
                return self.load_self_property::<GreyHack>(key.as_ref(), *call, *discard);
            }
            Op::GetSuperProperty { key, call, discard } => {
                return self.load_super_property::<GreyHack>(key.as_ref(), *call, *discard);
            }
            Op::MiniScriptGetProperty { call, discard } => {
                return self.load_property::<MiniScript>(*call, *discard);
            }
            Op::MiniScriptGetMember { key, call, discard } => {
                return self.load_member::<MiniScript>(key, *call, *discard);
            }
            Op::MiniScriptGetSelfProperty { key, call, discard } => {
                return self.load_self_property::<MiniScript>(key.as_ref(), *call, *discard);
            }
            Op::MiniScriptGetSuperProperty { key, call, discard } => {
                return self.load_super_property::<MiniScript>(key.as_ref(), *call, *discard);
            }
            Op::GetScope(scope) => self.load_scope(*scope),
            Op::Assign => self.assign()?,
            Op::AssignLocal { key, cache } => self.assign_local(key, cache),
            Op::AssignMember(key) => self.assign_member(key)?,
            Op::Unary { operator, discard } => self.unary(*operator, *discard),
            Op::Slice => self.slice(),
            Op::Binary { operator, right } => self.binary(*operator, right.as_ref()),
            Op::SlotBinary {
                operator,
                left,
                right,
                to,
            } => self.slot_binary(*operator, left, right, *to),
            Op::VariableBinary {
                operator,
                left,
                right,
                to,
            } => self.variable_binary(*operator, left, right, to),
            Op::GetSlot {
                slot,
                key,
                invoke,
                discard,
            } => return self.load_slot(*slot, key, *invoke, *discard),
            Op::AssignSlot { slot, value } => self.assign_slot(*slot, value),
            Op::CopyVariable { from, to } => self.copy_variable(from, to),
            Op::NextSlot {
                variable,
                index,
                body,
            } => self.next_slot_item(*variable, *index, *body),
            Op::AssignLocalBinary {
                key,
                operator,
                right,
                cache,
            } => self.assign_local_binary(key, *operator, right, cache),
            Op::BinaryJumpIf {
                operator,
                right,
                target,
                when,
            } => self.binary_jump_if(*operator, right.as_ref(), *target, *when),
            Op::SlotBinaryJumpIf {
                operator,
                left,
                right,
                target,
                when,
            } => self.slot_binary_jump_if(*operator, left, right, *target, *when),
            Op::VariableBinaryJumpIf {
                operator,
                left,
                right,
                target,
                when,
            } => self.variable_binary_jump_if(*operator, left, right, *target, *when),
            Op::Comparison(operators) => self.compare_chain(operators),
            Op::MakeList { items, discard } => self.make_list(*items, *discard),
            Op::MakeMap { entries, discard } => self.make_map(*entries, *discard),
            Op::Function {
                params,
                code,
                capture_outer,
            } => self.make_function(params, *code, *capture_outer),
            Op::Call { arguments, discard } => return self.call(*arguments, *discard),
            Op::Return(value) => self.return_from_frame(value.as_ref()),
            Op::Jump(target) => self.jump(*target),
            Op::JumpIf {
                target,
                when,
                push_condition,
            } => self.jump_if(*target, *when, *push_condition),
            Op::PushIterator => self.push_iterator(),
            Op::Next {
                variable,
                index,
                body,
            } => self.next_item(variable, index, *body),
            Op::PopIterator => {
                self.frame_mut().iterations.pop();
            }
            Op::Import { slot, code } => self.import(*slot, *code),
            Op::Export(slot) => self.export(*slot),
            Op::Breakpoint => {
                if self.breakpoints || self.debug_stop() {
                    self.state = State::Pause;
                }
            }
            Op::EnableBreakpoint => self.breakpoints = true,
        }

        Ok(())
    }
}
