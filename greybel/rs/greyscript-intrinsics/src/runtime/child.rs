//! Compiles `launch` child contexts and runs SSH service scripts in separate interpreters.
//! Grey Hack launches pre-built binaries, so children compile as self-contained code; [`SelfContained`] serves only their source.

use std::cell::RefCell;
use std::io;
use std::rc::Rc;

use greybel_core::Dialect;
use greybel_documents::{AsyncResourceProvider, NativeImportMode, Uri};
use greybel_interpreter::{
    Interpreter, InterpreterError, InterpreterOptions, PrintOptions, Program, Stdout,
};

use super::register_game;
use crate::Game;

/// Captured background output and optional failure from an SSH script.
pub struct CapturedScript {
    pub output: String,
    pub error: Option<InterpreterError>,
}

/// Compiles and runs child programs using the parent's game server, I/O, and clock.
#[derive(Clone)]
pub struct ChildRunner {
    /// Parent interpreter settings for I/O, clock, and encoding.
    /// Each child clones these and overrides `target` and `params`.
    options: InterpreterOptions,
}

impl ChildRunner {
    /// Captures inherited interpreter options; shared I/O sends child output to the parent's destination.
    #[must_use]
    pub fn new(options: &InterpreterOptions) -> Self {
        Self {
            options: options.clone(),
        }
    }

    /// Compiles binary `source` at `target` for a child context in the launching VM.
    ///
    /// # Errors
    ///
    /// Returns compilation errors.
    pub(crate) async fn compile(
        &self,
        target: &str,
        source: String,
    ) -> Result<Rc<Program>, InterpreterError> {
        child(self.options.clone(), target, source).compile().await
    }

    /// Compiles `source` for `build`.
    ///
    /// # Errors
    ///
    /// Returns compilation diagnostics formatted like Grey Hack's compiler.
    pub async fn compile_check(
        &self,
        target: &str,
        source: String,
    ) -> Result<(), InterpreterError> {
        child(self.options.clone(), target, source)
            .compile()
            .await
            .map(|_| ())
    }

    /// Runs an SSH script with captured output, preserving output emitted before an error alongside that error.
    pub async fn run_and_capture(
        &self,
        target: &str,
        source: String,
        params: Vec<String>,
        game: Game,
    ) -> CapturedScript {
        let capture = Rc::new(CaptureStdout::default());
        let options = InterpreterOptions {
            params,
            stdout: Rc::clone(&capture) as Rc<dyn Stdout>,
            ..self.options.clone()
        };
        let error = self.run_with(options, target, source, game).await.err();

        CapturedScript {
            output: capture.take(),
            error,
        }
    }

    /// Runs `source` as `game` with `options` and the game API registered.
    async fn run_with(
        &self,
        options: InterpreterOptions,
        target: &str,
        source: String,
        game: Game,
    ) -> Result<(), InterpreterError> {
        let mut interpreter = register_game(child(options, target, source), self.clone(), game);

        interpreter.run().await?;

        Ok(())
    }
}

/// Builds a self-contained interpreter for `source` as `target` using `options`.
/// Imports are already inlined by the game server; compilation uses Grey Hack's strict mode.
fn child(
    options: InterpreterOptions,
    target: &str,
    source: String,
) -> Interpreter<SingleSourceProvider> {
    let options = InterpreterOptions {
        dialect: Dialect::GreyScript,
        strict_mode: true,
        target: target.to_owned(),
        native_imports: NativeImportMode::Call,
        ..options
    };

    Interpreter::new(
        options,
        SingleSourceProvider {
            entry: Uri::memory(target),
            code: source,
        },
    )
}

/// [`Stdout`] implementation that records lines without displaying them.
#[derive(Default)]
struct CaptureStdout(RefCell<Vec<String>>);

impl CaptureStdout {
    fn take(&self) -> String {
        self.0.borrow().join("\n")
    }
}

impl Stdout for CaptureStdout {
    fn print(&self, text: &str, _options: PrintOptions) {
        self.0.borrow_mut().push(text.to_owned());
    }

    fn clear(&self) {
        self.0.borrow_mut().clear();
    }
}

/// Serves one source and reports all other paths as missing, disabling import resolution for launched programs.
pub(crate) struct SingleSourceProvider {
    entry: Uri,
    code: String,
}

impl AsyncResourceProvider for SingleSourceProvider {
    async fn resolve(&self, target: &str) -> io::Result<Uri> {
        Ok(Uri::memory(target))
    }

    async fn relative_to(&self, _source: &Uri, target: &str) -> io::Result<Uri> {
        Ok(Uri::memory(target))
    }

    async fn read(&self, uri: &Uri) -> io::Result<Option<String>> {
        Ok((*uri == self.entry).then(|| self.code.clone()))
    }
}
