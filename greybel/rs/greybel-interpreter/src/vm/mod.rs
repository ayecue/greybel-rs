mod call;
mod collector;
mod context;
mod debug;
mod exec;
mod frame;
mod host;
mod implementation;
mod index;
mod inject;
mod native;
mod operators;
mod property;
mod scheduler;
mod stack;
mod types;

use std::any::TypeId;
use std::future::Future;
use std::rc::Rc;

use ::scheduler::Clock;
pub use collector::Pin;
pub use context::{ContextEndFn, ContextEntry, ContextExit};
pub use debug::{DebugControl, FrameView, StepMode, StopReason};
use greybel_core::{Dialect, PositionEncoding};
use greybel_io::{Stdin, Stdout};
use greybel_symbols::names::{
    ENTRY_KEY, ENTRY_VALUE, GLOBALS, IS_GREYBEL, ISA, LOCALS, OUTER, PARAMS, SELF, SUPER,
};
use greybel_symbols::types::{FUNC_REF, LIST, MAP, NUMBER, STRING};
pub use native::{ArgContainer, AsyncNativeFn, NativeFn, NativeFuture, NativeOutput, NativeReturn};
use rustc_hash::FxHashMap;
pub use scheduler::{DEFAULT_BATCH, DEFAULT_SLICE, InstructionBudget, Scheduler, YieldHook};
pub use types::{TypeTable, ValueType};

use self::collector::Collector;
use self::frame::{CallStack, Frame};
use self::host::HostContext;
use self::types::ScopeTable;
use crate::api::Native;
use crate::bytecode::{Location, Program};
use crate::error::RuntimeError;
use crate::value::{Closure, FunctionBody, Heap, Key, Map, MapLabel, Param, Value};

const COLLECTION_THRESHOLD: usize = 1 << 20;
const MEMORY_LIMIT: usize = 1 << 30;

#[derive(Clone)]
pub(crate) struct VmOptions {
    pub params: Vec<String>,
    pub map_label: Option<MapLabel>,
    pub collection_threshold: usize,
    pub memory_limit: usize,
    pub string_encoding: PositionEncoding,
    pub stdout: Rc<dyn Stdout>,
    pub stdin: Rc<dyn Stdin>,
    pub clock: Rc<dyn Clock>,
}

impl VmOptions {
    #[cfg(not(target_family = "wasm"))]
    fn process_stdio(clock: &Rc<dyn Clock>) -> (Rc<dyn Stdout>, Rc<dyn Stdin>) {
        use greybel_io_terminal::{TerminalScreen, TerminalStdin, TerminalStdout};

        let stdout: Rc<dyn Stdout> = Rc::new(TerminalStdout::new(
            TerminalScreen::default(),
            Rc::clone(clock),
        ));
        let stdin = Rc::new(TerminalStdin::new(Rc::clone(&stdout)));

        (stdout, stdin)
    }

    #[cfg(target_family = "wasm")]
    fn process_stdio(_clock: &Rc<dyn Clock>) -> (Rc<dyn Stdout>, Rc<dyn Stdin>) {
        (Rc::new(greybel_io::NullIo), Rc::new(greybel_io::NullIo))
    }
}

impl VmOptions {
    pub(crate) fn new(clock: Rc<dyn Clock>) -> Self {
        let (stdout, stdin) = Self::process_stdio(&clock);

        Self {
            params: Vec::new(),
            map_label: None,
            collection_threshold: COLLECTION_THRESHOLD,
            memory_limit: MEMORY_LIMIT,
            string_encoding: PositionEncoding::Utf16,
            stdout,
            stdin,
            clock,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunResult {
    Complete,
    Breakpoint(Option<Location>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Active,
    Pending { discard: bool },
    Yield,
    Pause,
    Complete,
    Failure,
}

struct Keys {
    self_key: Key,
    super_key: Key,
    isa: Key,
    entry_key: Key,
    entry_value: Key,
    locals: Key,
    outer: Key,
    globals: Key,
}

pub struct Vm {
    program: Rc<Program>,
    heap: Heap,
    stack: Vec<Value>,
    calls: CallStack,
    scopes: ScopeTable,
    native_functions: FxHashMap<*const Native, Value>,
    state: State,
    breakpoints: bool,
    debug: Option<Rc<DebugControl>>,
    keys: Keys,
    collector: Collector,
    native_depth: u32,
    pending_call: Option<NativeFuture>,
    host: HostContext,
    spare_arguments: Option<Vec<Value>>,
    failure: Option<RuntimeError>,
    named_types: u8,
}

impl Vm {
    #[must_use]
    pub(crate) fn new(program: impl Into<Rc<Program>>, options: VmOptions) -> Self {
        let program = program.into();
        let named_types = match program.dialect {
            Dialect::MiniScript => u8::MAX,
            Dialect::GreyScript | Dialect::Greybel => 0,
        };
        let mut heap = Heap::default().with_string_encoding(options.string_encoding);

        if let Some(label) = options.map_label {
            heap = heap.with_map_label(label);
        }

        let types = TypeTable {
            string: heap.add_map(),
            number: heap.add_map(),
            list: heap.add_map(),
            map: heap.add_map(),
            function: heap.add_map(),
        };
        let api = heap.add_map_from([
            (STRING.into(), Value::Map(types.string)),
            (NUMBER.into(), Value::Map(types.number)),
            (LIST.into(), Value::Map(types.list)),
            (MAP.into(), Value::Map(types.map)),
            (FUNC_REF.into(), Value::Map(types.function)),
        ]);
        let params = options.params.into_iter().map(Value::from).collect();
        let params = Value::List(heap.add_list(params));
        let globals =
            heap.add_map_from([(IS_GREYBEL.into(), Value::TRUE), (PARAMS.into(), params)]);
        let entry = Frame::new(program.entry(), Some(globals), None, false);
        let exports = vec![None; program.import_count as usize];

        Self {
            program,
            heap,
            stack: Vec::new(),
            calls: CallStack::new(entry),
            scopes: ScopeTable {
                globals,
                api,
                types,
                exports,
            },
            native_functions: FxHashMap::default(),
            state: State::Active,
            breakpoints: false,
            debug: None,
            collector: Collector::new(options.collection_threshold, options.memory_limit),
            native_depth: 0,
            pending_call: None,
            host: HostContext::new(options.stdout, options.stdin, options.clock),
            keys: Keys {
                self_key: Key::new(SELF),
                super_key: Key::new(SUPER),
                isa: Key::new(ISA),
                entry_key: Key::new(ENTRY_KEY),
                entry_value: Key::new(ENTRY_VALUE),
                locals: Key::new(LOCALS),
                outer: Key::new(OUTER),
                globals: Key::new(GLOBALS),
            },
            spare_arguments: None,
            failure: None,
            named_types,
        }
    }

    pub fn type_map(&mut self, kind: ValueType) -> Map {
        self.named_types |= 1 << kind as u8;

        self.scopes.types.get(kind)
    }

    pub(super) fn is_named(&self, kind: ValueType) -> bool {
        self.named_types & (1 << kind as u8) != 0
    }

    #[must_use]
    pub fn program(&self) -> &Program {
        &self.program
    }

    #[must_use]
    pub fn heap(&self) -> &Heap {
        &self.heap
    }

    pub fn heap_mut(&mut self) -> &mut Heap {
        &mut self.heap
    }

    #[must_use]
    pub fn types(&self) -> TypeTable {
        self.scopes.types
    }

    #[must_use]
    pub fn globals(&self) -> Map {
        self.scopes.globals
    }

    #[must_use]
    pub fn api(&self) -> Map {
        self.scopes.api
    }

    pub fn add_native(
        &mut self,
        params: impl IntoIterator<Item = Param>,
        function: impl Fn(&mut Self, &ArgContainer) -> Result<Value, RuntimeError> + 'static,
    ) -> Value {
        self.add_native_function(
            params,
            FunctionBody::Native {
                call: Rc::new(move |vm, args| function(vm, args).map(NativeReturn::from)),
                label: None,
            },
        )
    }

    pub fn add_async_native<F>(
        &mut self,
        params: impl IntoIterator<Item = Param>,
        function: impl Fn(&mut Self, &ArgContainer) -> F + 'static,
    ) -> Value
    where
        F: Future<Output = Result<Value, RuntimeError>> + 'static,
    {
        let function: AsyncNativeFn = Rc::new(move |vm, args| {
            let future = function(vm, args);

            Box::pin(async move { NativeOutput::new(future.await) })
        });

        self.add_native_function(
            params,
            FunctionBody::AsyncNative {
                call: function,
                label: None,
            },
        )
    }

    pub fn native_function(&mut self, native: &'static Native) -> Value {
        if let Some(value) = self.native_functions.get(&std::ptr::from_ref(native)) {
            return value.clone();
        }

        let value = native.add_function(self);

        self.native_functions
            .insert(std::ptr::from_ref(native), value.clone());

        value
    }

    #[must_use]
    pub fn stdout(&self) -> &Rc<dyn Stdout> {
        &self.host.stdout
    }

    #[must_use]
    pub fn stdin(&self) -> &Rc<dyn Stdin> {
        &self.host.stdin
    }

    #[must_use]
    pub fn clock(&self) -> &Rc<dyn Clock> {
        &self.host.clock
    }

    #[must_use]
    pub fn run_time(&self) -> f64 {
        self.host
            .clock
            .now()
            .saturating_sub(self.host.start_time)
            .as_secs_f64()
    }

    pub fn request_yield(&mut self) {
        if self.state == State::Active {
            self.state = State::Yield;
        }
    }

    pub fn set_host_data<T: 'static>(&mut self, data: T) {
        self.host_data_table(TypeId::of::<T>())
            .insert(TypeId::of::<T>(), Box::new(data));
    }

    #[must_use]
    pub fn host_data<T: 'static>(&self) -> Option<&T> {
        let id = TypeId::of::<T>();

        self.host
            .context_stack
            .data
            .get(&id)
            .or_else(|| self.host.data.get(&id))?
            .downcast_ref()
    }

    pub fn host_data_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.host_data_table(TypeId::of::<T>())
            .get_mut(&TypeId::of::<T>())?
            .downcast_mut()
    }

    pub fn host_data_or_insert_with<T: 'static>(&mut self, init: impl FnOnce() -> T) -> &mut T {
        self.host_data_table(TypeId::of::<T>())
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(init()))
            .downcast_mut()
            .expect("host data is stored under its own type id")
    }

    fn host_data_table(&mut self, id: TypeId) -> &mut context::HostData {
        if self.host.context_stack.data.contains_key(&id) {
            &mut self.host.context_stack.data
        } else {
            &mut self.host.data
        }
    }

    pub(crate) fn add_native_function(
        &mut self,
        params: impl IntoIterator<Item = Param>,
        body: FunctionBody,
    ) -> Value {
        let function = self.heap.add_function(Closure {
            params: params.into_iter().collect(),
            body,
            outer: None,
        });

        Value::Function(function)
    }

    pub fn define(&mut self, name: &str, value: Value) {
        self.scopes.api.set(&mut self.heap, name.into(), value);
    }

    pub fn add_intrinsic(&mut self, kind: ValueType, name: &str, value: Value) {
        self.scopes
            .types
            .get(kind)
            .set(&mut self.heap, name.into(), value);
    }

    pub fn exit(&mut self) {
        self.state = State::Complete;
    }

    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.state == State::Complete
    }

    pub fn set_breakpoints(&mut self, enabled: bool) {
        self.breakpoints = enabled;
    }

    #[must_use]
    pub fn breakpoints(&self) -> bool {
        self.breakpoints
    }
}
