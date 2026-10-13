use greybel_core::Ast;
use greybel_documents::{DocumentError, Uri};
use greybel_symbols::names::UNNAMED_SOURCE;

use crate::bundle::emit_multiline_string_function;
use crate::emit::{BeautifyEmitter, DefaultEmitter, EmitFlags, EmitState, Emitter, UglifyEmitter};
use crate::error::Result;
use crate::minify::Minification;
use crate::options::{BuildMode, TranspileOptions};

pub fn parse(target: &Uri, source: &str, options: &TranspileOptions) -> Result<Ast> {
    parse_target(target, &target.label(), source, options)
}

pub(crate) fn parse_anonymous(source: &str, options: &TranspileOptions) -> Result<Ast> {
    parse_target(
        &Uri::memory(UNNAMED_SOURCE),
        UNNAMED_SOURCE,
        source,
        options,
    )
}

pub fn transpile(source: &str, options: &TranspileOptions) -> Result<String> {
    emit(&parse_anonymous(source, options)?, options)
}

pub fn emit(ast: &Ast, options: &TranspileOptions) -> Result<String> {
    match &options.mode {
        BuildMode::Default => emit_standalone::<_>(&mut DefaultEmitter, ast, options),
        BuildMode::Uglify(uglify) => emit_standalone::<_>(
            &mut UglifyEmitter::new(*uglify, Minification::analyze(&[ast], options)),
            ast,
            options,
        ),
        BuildMode::Beautify(beautify) => {
            emit_standalone::<_>(&mut BeautifyEmitter::new(*beautify), ast, options)
        }
    }
}

fn parse_target(
    target: &Uri,
    filename: &str,
    source: &str,
    options: &TranspileOptions,
) -> Result<Ast> {
    greybel_core::parse(source, options.load_options().parser_options(filename)).map_err(|error| {
        DocumentError::Parse {
            target: target.clone(),
            error,
        }
        .into()
    })
}

fn emit_standalone<E: Emitter>(
    emitter: &mut E,
    ast: &Ast,
    options: &TranspileOptions,
) -> Result<String> {
    let multiline_string = emit_multiline_string_function(emitter, std::iter::once(ast), options)?;
    let mut state = EmitState::new(ast, options);

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
