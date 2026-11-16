use greybel_core::Ast;
use greybel_documents::{DocumentError, Uri};
use greybel_symbols::names::UNNAMED_SOURCE;

use crate::bundle::emit_multiline_string_function;
use crate::emit::{
    BeautifyEmitter, ConstantTable, DefaultEmitter, EmitFlags, EmitState, Emitter, UglifyEmitter,
};
use crate::error::Result;
use crate::minify::Minification;
use crate::options::{BuildMode, TranspileOptions};

#[derive(Debug, Clone)]
pub struct Transpiler<'o> {
    options: &'o TranspileOptions,
}

impl<'o> Transpiler<'o> {
    #[must_use]
    pub fn new(options: &'o TranspileOptions) -> Self {
        Self { options }
    }

    pub(crate) fn options(&self) -> &'o TranspileOptions {
        self.options
    }

    pub fn parse(&self, target: &Uri, source: &str) -> Result<Ast> {
        self.parse_target(target, &target.label(), source)
    }

    pub(crate) fn parse_anonymous(&self, source: &str) -> Result<Ast> {
        self.parse_target(&Uri::memory(UNNAMED_SOURCE), UNNAMED_SOURCE, source)
    }

    pub fn transpile(&self, source: &str) -> Result<String> {
        self.emit(&self.parse_anonymous(source)?)
    }

    pub fn emit(&self, ast: &Ast) -> Result<String> {
        match &self.options.mode {
            BuildMode::Default => self.emit_standalone::<_>(&mut DefaultEmitter, ast),
            BuildMode::Uglify(uglify) => self.emit_standalone::<_>(
                &mut UglifyEmitter::new(*uglify, Minification::analyze(&[ast], self.options)),
                ast,
            ),
            BuildMode::Beautify(beautify) => {
                self.emit_standalone::<_>(&mut BeautifyEmitter::new(*beautify), ast)
            }
        }
    }

    fn parse_target(&self, target: &Uri, filename: &str, source: &str) -> Result<Ast> {
        greybel_core::parse(source, self.options.load_options().parser_options(filename)).map_err(
            |error| {
                DocumentError::Parse {
                    target: target.clone(),
                    error,
                }
                .into()
            },
        )
    }

    fn emit_standalone<E: Emitter>(&self, emitter: &mut E, ast: &Ast) -> Result<String> {
        let options = self.options;
        let multiline_string =
            emit_multiline_string_function(emitter, std::iter::once(ast), options)?;
        let constants = ConstantTable::of([ast]);
        let mut state = EmitState::new(ast, options).with_constants(&constants);

        emitter.emit(&mut state, ast.root(), EmitFlags::default())?;

        let lines: Vec<String> = multiline_string
            .into_iter()
            .chain(emitter.preamble(&state))
            .collect();
        let code = emitter.finish(state.into_output());

        if lines.is_empty() {
            return Ok(code);
        }

        let mut output = lines.join("\n");

        output.push('\n');
        output.push_str(&code);

        Ok(output)
    }
}
