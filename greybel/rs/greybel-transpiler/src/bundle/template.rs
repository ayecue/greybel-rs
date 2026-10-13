use greybel_core::{Ast, LiteralValue, NodeKind};
use greybel_symbols::names::{GLOBALS, MODULE as MODULE_NAME};

use super::output::Part;
use crate::emit::{EmitFlags, EmitState, Emitter, LineBreakStyle, is_multiline};
use crate::error::Result;
use crate::options::TranspileOptions;
use crate::transpile::parse_anonymous;

pub(crate) const REQUIRE: &str = "__REQUIRE";
pub(crate) const MULTILINE_STRING: &str = "__MSTR";

pub(crate) const RUNTIME_NAMES: &[&str] = &[
    "BACKSLASH_CODE",
    "NEW_LINE_OPERATOR",
    "MODULES",
    "EXPORTED",
    REQUIRE,
    "__REQUIRE_EVAL",
    "MAIN",
    MODULE_NAME,
    GLOBALS,
];

pub(super) const HEADER: &str = "MODULES={}
\tEXPORTED={}
  __REQUIRE_EVAL=function(cb, ns)
    if EXPORTED.hasIndex(ns) then return EXPORTED[ns]
    result=cb(ns)
    if result == null then result = { \"exports\": null }
    if not result.hasIndex(\"exports\") then result.exports = null
    EXPORTED[ns]=result
    return result
  end function
\t__REQUIRE=function(ns)
\tif not MODULES.hasIndex(ns) then
\tprint(\"Module \"+ns+\" cannot be found...\")
\treturn null
\tend if
\treturn @__REQUIRE_EVAL(@MODULES[ns],ns).exports
\tend function";

/// Definition of the multiline-string function, emitted first when any string contains a line break.
const MULTILINE_STRING_FUNCTION: &str = "__MSTR=function(lines)
\treturn lines.join(char(10))
\tend function";

pub(super) const MODULE: &str = "MODULES[\"__MODULE_NAME__\"]=function(r)
\tmodule={}
\t\"__MODULE_CODE__\"
\treturn module
\tend function";

const NAME_PLACEHOLDER: &str = "\"__MODULE_NAME__\"";
const CODE_PLACEHOLDER: &str = "\"__MODULE_CODE__\"";

pub(crate) fn emit_multiline_string_function<'a, E: Emitter>(
    emitter: &mut E,
    mut asts: impl Iterator<Item = &'a Ast>,
    options: &TranspileOptions,
) -> Result<Option<String>> {
    let joins = LineBreakStyle::of(options, MULTILINE_STRING).joins()
        && asts.any(|ast| {
            ast.literals.iter().any(|&id| {
                matches!(ast.kind(id), NodeKind::Literal(literal) if matches!(literal.value, LiteralValue::String(_)))
                    && is_multiline(ast.text(id).unwrap_or_default())
            })
        });

    if !joins {
        return Ok(None);
    }

    emit_boilerplate(emitter, MULTILINE_STRING_FUNCTION, options).map(Some)
}

pub(super) fn emit_boilerplate<E: Emitter>(
    emitter: &mut E,
    source: &str,
    options: &TranspileOptions,
) -> Result<String> {
    let parse_options = TranspileOptions {
        strict_mode: false,
        ..options.clone()
    };
    let ast = parse_anonymous(source, &parse_options)?;
    let mut state = EmitState::new(&ast, options);

    emitter.emit(&mut state, ast.root(), EmitFlags::default())?;

    Ok(emitter.finish(state.into_output()))
}

pub(super) struct Template {
    text: String,
    name_range: std::ops::Range<usize>,
    code_range: std::ops::Range<usize>,
}

impl Template {
    pub(super) fn new(text: String) -> Self {
        let find = |placeholder: &str| {
            let start = text.find(placeholder).unwrap_or(text.len());

            start..(start + placeholder.len()).min(text.len())
        };
        let (name_range, code_range) = (find(NAME_PLACEHOLDER), find(CODE_PLACEHOLDER));

        Self {
            text,
            name_range,
            code_range,
        }
    }

    pub(super) fn fill(&self, namespace: &str, code: &Part) -> Part {
        let text = &self.text;
        let mut out = String::with_capacity(text.len() + namespace.len() + code.text.len());

        out.push_str(&text[..self.name_range.start]);
        out.push('"');
        out.push_str(namespace);
        out.push('"');
        out.push_str(&text[self.name_range.end..self.code_range.start]);

        let before = out.matches('\n').count();

        out.push_str(&code.text);
        out.push_str(&text[self.code_range.end..]);

        let mut origins = vec![None; before];

        origins.extend_from_slice(&code.origins);

        Part::new(out, &origins)
    }
}
