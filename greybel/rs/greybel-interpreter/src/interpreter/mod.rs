mod error;
mod options;

use std::rc::Rc;

use greybel_core::{Dialect, ParserOptions};
use greybel_documents::{AsyncResourceProvider, DocumentError, DocumentOptions, Workspace};

pub use self::error::InterpreterError;
pub use self::options::InterpreterOptions;
use crate::api::Register;
use crate::bytecode::Program;
use crate::compiler::CompileOptions;
use crate::value::{MapLabel, Value};
use crate::vm::{InstructionBudget, RunResult, Scheduler, Vm, VmOptions};

pub const INJECTED_TARGET: &str = "injected";

pub struct Interpreter<P, S = InstructionBudget> {
    options: InterpreterOptions,
    provider: P,
    scheduler: S,
    apis: Vec<Box<dyn Register>>,
    program: Option<Rc<Program>>,
    vm: Option<Vm>,
}

impl<P: AsyncResourceProvider> Interpreter<P> {
    #[must_use]
    pub fn new(options: InterpreterOptions, provider: P) -> Self {
        Self {
            options,
            provider,
            scheduler: InstructionBudget::default(),
            apis: Vec::new(),
            program: None,
            vm: None,
        }
    }
}

impl<P: AsyncResourceProvider, S: Scheduler> Interpreter<P, S> {
    #[must_use]
    pub fn with_scheduler<T: Scheduler>(self, scheduler: T) -> Interpreter<P, T> {
        Interpreter {
            options: self.options,
            provider: self.provider,
            scheduler,
            apis: self.apis,
            program: self.program,
            vm: self.vm,
        }
    }

    #[must_use]
    pub fn with_api(mut self, api: impl Register + 'static) -> Self {
        self.add_api(api);

        self
    }

    pub fn add_api(&mut self, api: impl Register + 'static) {
        self.apis.push(Box::new(api));
    }

    #[must_use]
    pub fn with_host_data<T: Clone + 'static>(self, data: T) -> Self {
        self.with_api(move |vm: &mut Vm| vm.set_host_data(data.clone()))
    }

    #[must_use]
    pub fn options(&self) -> &InterpreterOptions {
        &self.options
    }

    pub fn set_target(&mut self, target: impl Into<String>) {
        self.options.target = target.into();
        self.program = None;
        self.vm = None;
    }

    #[must_use]
    pub fn provider(&self) -> &P {
        &self.provider
    }

    #[must_use]
    pub fn scheduler(&self) -> &S {
        &self.scheduler
    }

    pub fn scheduler_mut(&mut self) -> &mut S {
        &mut self.scheduler
    }

    #[must_use]
    pub fn program(&self) -> Option<&Rc<Program>> {
        self.program.as_ref()
    }

    #[must_use]
    pub fn vm(&self) -> Option<&Vm> {
        self.vm.as_ref()
    }

    pub fn vm_mut(&mut self) -> Option<&mut Vm> {
        self.vm.as_mut()
    }

    pub async fn compile(&mut self) -> Result<Rc<Program>, InterpreterError> {
        let target = self.options.target.clone();
        let program = Rc::new(self.load(&target, None, self.options.debug).await?);

        self.program = Some(Rc::clone(&program));

        Ok(program)
    }

    pub async fn compile_code(&mut self, code: &str) -> Result<Rc<Program>, InterpreterError> {
        let target = self.options.target.clone();
        let program = Rc::new(self.load(&target, Some(code), self.options.debug).await?);

        self.program = Some(Rc::clone(&program));

        Ok(program)
    }

    pub async fn start(&mut self) -> Result<(), InterpreterError> {
        let program = match &self.program {
            Some(program) => Rc::clone(program),
            None => self.compile().await?,
        };
        let options = VmOptions {
            params: self.options.params.clone(),
            map_label: self.map_label(),
            collection_threshold: self.options.collection_threshold,
            memory_limit: self.options.memory_limit,
            string_encoding: self.options.string_encoding,
            stdout: Rc::clone(&self.options.stdout),
            stdin: Rc::clone(&self.options.stdin),
            clock: Rc::clone(&self.options.clock),
        };
        let mut vm = Vm::new(program, options);

        for api in &self.apis {
            api.register(&mut vm);
        }

        self.vm = Some(vm);

        Ok(())
    }

    pub async fn resume(&mut self) -> Result<RunResult, InterpreterError> {
        let Self { vm, scheduler, .. } = self;
        let vm = vm.as_mut().ok_or(InterpreterError::NoProgram)?;

        Ok(vm.run(scheduler).await?)
    }

    pub async fn run(&mut self) -> Result<RunResult, InterpreterError> {
        self.start().await?;

        self.resume().await
    }

    pub async fn run_code(&mut self, code: &str) -> Result<RunResult, InterpreterError> {
        self.compile_code(code).await?;

        self.run().await
    }

    pub async fn inject(&mut self, code: &str) -> Result<(), InterpreterError> {
        if self.vm.is_none() {
            return Err(InterpreterError::NoProgram);
        }

        let program = self.load(INJECTED_TARGET, Some(code), false).await?;
        let Self { vm, scheduler, .. } = self;
        let vm = vm.as_mut().ok_or(InterpreterError::NoProgram)?;

        Ok(vm.inject(program, scheduler).await?)
    }

    pub fn exit(&mut self) {
        if let Some(vm) = &mut self.vm {
            vm.exit();
        }
    }

    #[must_use]
    pub fn global(&self, name: &str) -> Option<&Value> {
        let vm = self.vm.as_ref()?;

        vm.globals().get(vm.heap(), &Value::from(name))
    }

    pub fn set_global(&mut self, name: &str, value: Value) -> Result<(), InterpreterError> {
        let vm = self.vm.as_mut().ok_or(InterpreterError::NoProgram)?;
        let globals = vm.globals();

        globals.set(vm.heap_mut(), name.into(), value);

        Ok(())
    }

    fn map_label(&self) -> Option<MapLabel> {
        match self.options.dialect {
            Dialect::GreyScript => Some(|heap, map| map.class_id(heap)),
            Dialect::MiniScript | Dialect::Greybel => None,
        }
    }

    async fn load(
        &self,
        target: &str,
        code: Option<&str>,
        debug: bool,
    ) -> Result<Program, InterpreterError> {
        let documents = DocumentOptions {
            parser: ParserOptions {
                environment: self.options.environment.clone(),
                strict_mode: self.options.strict_mode,
                syntax_features: self.options.syntax_features,
                mono_number_parsing: true,
                ..ParserOptions::new(self.options.dialect)
            },
            native_imports: self.options.native_imports,
            ..DocumentOptions::default()
        };
        let mut workspace = Workspace::new(documents);
        let uri = self
            .provider
            .resolve(target)
            .await
            .map_err(|source| DocumentError::Resolve {
                target: target.to_owned(),
                source,
            })?;

        if let Some(code) = code {
            workspace.set_file(&uri, code);
        }

        workspace.load_async(&uri, &self.provider).await?;

        let graph = workspace.module_graph(&uri)?;
        let options = CompileOptions {
            environment: self.options.environment.clone(),
            debug,
            dialect: self.options.dialect,
            native_imports: self.options.native_imports,
        };

        Ok(Program::compile(&graph, &options)?)
    }
}
