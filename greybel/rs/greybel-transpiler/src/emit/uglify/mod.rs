mod emitter;
mod shape;

use greybel_symbols::names::{GLOBALS, LOOP_INDEX_PREFIX, LOOP_INDEX_SUFFIX};
use rustc_hash::FxHashMap;

use super::{EmitFlags, EmitState, Emitter};
use crate::bundle::{MULTILINE_STRING, REQUIRE};
use crate::error::Result;
use crate::minify::Minification;
use crate::options::UglifyOptions;

#[derive(Debug, Clone)]
pub struct UglifyEmitter {
    options: UglifyOptions,
    minification: Minification,
    loop_index_names: FxHashMap<String, String>,
    in_parameters: bool,
}

impl UglifyEmitter {
    #[must_use]
    pub fn new(options: UglifyOptions, minification: Minification) -> Self {
        Self {
            options,
            minification,
            loop_index_names: FxHashMap::default(),
            in_parameters: false,
        }
    }

    fn resolved_name<'a>(&'a self, name: &'a str) -> &'a str {
        let index = (!self.loop_index_names.is_empty() && name.starts_with("__"))
            .then(|| self.loop_index_names.get(name))
            .flatten();

        index.map_or_else(|| self.minification.rename(name), String::as_str)
    }

    fn rename_loop_index(&mut self, original: &str, index: String) {
        self.loop_index_names.insert(
            format!("{LOOP_INDEX_PREFIX}{original}{LOOP_INDEX_SUFFIX}"),
            index,
        );
    }

    fn in_parameters(
        &mut self,
        s: &mut EmitState<'_>,
        f: impl FnOnce(&mut Self, &mut EmitState<'_>) -> Result<()>,
    ) -> Result<()> {
        self.in_parameters = true;

        let result = f(self, s);

        self.in_parameters = false;

        result
    }

    fn preamble_lines(&self, s: &EmitState<'_>) -> Vec<String> {
        let globals = self.minification.rename(GLOBALS);
        let line_breaks = s.line_breaks(self.runtime_name(MULTILINE_STRING));
        let literals: Vec<_> = self.minification.hoisted_literals(line_breaks).collect();
        let mut lines = Vec::new();

        if !self.options.disable_namespaces_optimization || !literals.is_empty() {
            lines.push(format!("{GLOBALS}.{globals}={GLOBALS}"));
        }

        lines.extend(
            literals
                .into_iter()
                .map(|(namespace, text)| format!("{globals}.{namespace}={text}")),
        );

        lines
    }

    fn require_name(&self) -> &str {
        self.runtime_name(REQUIRE)
    }
}

fn member_flags(s: &EmitState<'_>, base: greybel_core::NodeId) -> EmitFlags {
    EmitFlags {
        is_member: true,
        base_is_scope: s.is_scope_name(base),
        ..EmitFlags::default()
    }
}
