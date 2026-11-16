use greybel_core::NodeId;
use greybel_core::syntax::greybel::{Envar, File, Import};
use greybel_core::syntax::greyscript::ImportCode;
use greybel_symbols::contextual_keywords::FROM;
use greybel_symbols::delimiters::{ESCAPED_QUOTE, QUOTE, STATEMENT_SEPARATOR};
use greybel_symbols::literals::NULL;
use greybel_symbols::{Keyword, Punct};

use super::{EmitStatements, Emitter};
use crate::emit::{EmitFlags, EmitState};
use crate::error::Result;

pub(crate) trait EmitDirectives: Emitter {
    fn emit_import_code(&mut self, s: &mut EmitState<'_>, id: NodeId, node: &ImportCode) {
        if s.options().dev_mode {
            s.push_import_code_source(node, !Self::RETAINS_COMMENTS);

            return;
        }

        if !node.emit {
            return;
        }

        if !node.eval || s.graph().is_none() {
            {
                let text = s.read(node.directory);

                s.keyword(Keyword::ImportCode);
                s.punct(Punct::LParen);
                s.write_quoted_text(&text);
                s.punct(Punct::RParen);
            };

            return;
        }

        for (index, path) in s.bundled_imports(id).iter().enumerate() {
            if index > 0 {
                s.eol();
            }

            s.keyword(Keyword::ImportCode);
            s.punct(Punct::LParen);
            s.write_quoted_text(path);
            s.punct(Punct::RParen);
        }
    }

    fn emit_envar(&mut self, s: &mut EmitState<'_>, node: &Envar) -> Result<()> {
        if s.options().dev_mode {
            {
                let text = s.read(node.name);

                s.keyword(Keyword::Envar);
                s.push(" ");
                s.push(&text);
            };

            return Ok(());
        }

        match s.options().environment.get(s.read(node.name).as_ref()) {
            Some(value) if !value.is_empty() => {
                s.write_quoted_text(&value.replace(QUOTE, ESCAPED_QUOTE));
            }
            _ => s.push(NULL),
        }

        Ok(())
    }

    fn emit_import(&mut self, s: &mut EmitState<'_>, id: NodeId, node: &Import) -> Result<()> {
        let Some(dependency) = s.dependency_module(id) else {
            s.keyword(Keyword::Import);
            s.push(" ");

            match node.target {
                Some(target) => {
                    let names = s.ast().slice(target).to_owned();

                    s.push(&names);
                }
                None => self.emit(s, node.name, EmitFlags::default())?,
            }

            {
                let text = s.read(node.path);

                s.push(" ");
                s.push(FROM);
                s.push(" ");
                s.write_quoted_text(&text);
                s.push(STATEMENT_SEPARATOR);
            };

            return Ok(());
        };

        self.emit(s, node.name, EmitFlags::default())?;

        let call = self.require_assignment(s, s.namespace(dependency));

        s.push(&call);

        let bindings = s.ast().list(node.bindings).to_vec();

        if bindings.is_empty() {
            return Ok(());
        }

        s.eol();

        self.emit_statements(s, &bindings)
    }

    fn emit_file(&mut self, s: &mut EmitState<'_>, node: &File) -> Result<()> {
        if s.options().dev_mode {
            s.keyword(Keyword::Filename);

            return Ok(());
        }

        let name = node.filename.rsplit(['/', '\\']).next().unwrap_or_default();

        s.write_quoted_text(name);

        Ok(())
    }
}

impl<E: Emitter> EmitDirectives for E {}
