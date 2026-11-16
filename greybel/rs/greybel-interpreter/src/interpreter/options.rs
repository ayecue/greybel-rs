use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use greybel_core::{Dialect, PositionEncoding, SyntaxFeatures};
use greybel_documents::NativeImportMode;
use greybel_io::{Stdin, Stdout};
use greybel_symbols::names::UNNAMED_SOURCE;
use scheduler::Clock;

use crate::vm::VmOptions;

#[derive(Clone)]
pub struct InterpreterOptions {
    /// Source dialect. `GreyScript` enables `import_code` and `classID` map labels.
    pub dialect: Dialect,
    /// Entry filename passed to the document provider.
    pub target: String,
    /// Initial values for the program's `params` global.
    pub params: Vec<String>,
    /// Values available through `#envar` lookups.
    pub environment: HashMap<String, String>,
    /// Restricts parsing to syntax compiled by the dialect reference; see `ParserOptions::strict_mode`.
    pub strict_mode: bool,
    /// The syntax features to accept, of those the dialect has.
    pub syntax_features: SyntaxFeatures,
    /// File-reading mode for `import_code`.
    pub native_imports: NativeImportMode,
    /// Inserts a breakpoint before each statement to support stepping.
    pub debug: bool,
    /// Units for string length, indexing, and slice positions.
    pub string_encoding: PositionEncoding,
    /// Minimum bytes allocated between garbage collections; the threshold grows when the live set exceeds it.
    pub collection_threshold: usize,
    /// Maximum heap-object and call-frame bytes before interruption, matching Grey Hack's out-of-memory behavior.
    /// Call nesting itself is unlimited.
    pub memory_limit: usize,
    /// Output-intrinsic destination: terminal on native targets and null on `wasm32-unknown-unknown`.
    pub stdout: Rc<dyn Stdout>,
    /// Input-intrinsic source: terminal on native targets and null on `wasm32-unknown-unknown`.
    pub stdin: Rc<dyn Stdin>,
    /// Host clock backing `time` and `wait`.
    pub clock: Rc<dyn Clock>,
}

impl InterpreterOptions {
    /// Creates default options whose `time` and `wait` operations use `clock`.
    #[must_use]
    pub fn new(clock: Rc<dyn Clock>) -> Self {
        let vm = VmOptions::new(clock);

        Self {
            dialect: Dialect::Greybel,
            target: UNNAMED_SOURCE.to_owned(),
            params: Vec::new(),
            environment: HashMap::new(),
            strict_mode: false,
            syntax_features: SyntaxFeatures::ALL,
            native_imports: NativeImportMode::Evaluation,
            debug: false,
            string_encoding: vm.string_encoding,
            collection_threshold: vm.collection_threshold,
            memory_limit: vm.memory_limit,
            stdout: vm.stdout,
            stdin: vm.stdin,
            clock: vm.clock,
        }
    }
}

impl fmt::Debug for InterpreterOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InterpreterOptions")
            .field("dialect", &self.dialect)
            .field("target", &self.target)
            .field("params", &self.params)
            .field("environment", &self.environment)
            .field("strict_mode", &self.strict_mode)
            .field("syntax_features", &self.syntax_features)
            .field("native_imports", &self.native_imports)
            .field("debug", &self.debug)
            .field("string_encoding", &self.string_encoding)
            .field("collection_threshold", &self.collection_threshold)
            .field("memory_limit", &self.memory_limit)
            .finish_non_exhaustive()
    }
}
