use greybel_core::{Ast, LiteralValue, NodeKind};
use greybel_symbols::names::{GLOBALS, MODULE as MODULE_NAME};

use super::output::Part;
use crate::emit::{EmitFlags, EmitState, Emitter, LineBreakStyle, LiteralText};
use crate::error::Result;
use crate::options::{MultilineStringStyle, TranspileOptions};
use crate::transpiler::Transpiler;

pub(crate) const REQUIRE: &str = "__REQUIRE";
pub(crate) const MULTILINE_STRING: &str = "__MSTR";
pub(crate) const LINE_BREAK: &str = "__NL";

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

/// The function or variable that builds write strings with line breaks with.
pub(crate) fn multiline_string_helper(options: &TranspileOptions) -> &'static str {
    match options.multiline_strings {
        MultilineStringStyle::Concatenation => LINE_BREAK,
        MultilineStringStyle::Join => MULTILINE_STRING,
    }
}

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
    let helper = multiline_string_helper(options);
    let rewrites = LineBreakStyle::of(options, helper).rewrites()
        && asts.any(|ast| {
            ast.literals.iter().any(|&id| {
                matches!(ast.kind(id), NodeKind::Literal(literal) if matches!(literal.value, LiteralValue::String(_)))
                    && LiteralText::is_multiline(ast.text(id).unwrap_or_default())
            })
        });

    if !rewrites {
        return Ok(None);
    }

    match options.multiline_strings {
        MultilineStringStyle::Join => {
            emit_boilerplate(emitter, MULTILINE_STRING_FUNCTION, options).map(Some)
        }
        // Written as text, so the uglify emitter cannot put a hoisted literal defined after it in place of `10`.
        MultilineStringStyle::Concatenation => Ok(Some(format!(
            "{}{padding}={padding}char(10)",
            emitter.runtime_name(LINE_BREAK),
            padding = E::INFIX_PADDING
        ))),
    }
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
    let ast = Transpiler::new(&parse_options).parse_anonymous(source)?;
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
