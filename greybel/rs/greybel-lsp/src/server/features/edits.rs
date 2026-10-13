use gen_lsp_types as lsp;
use greybel_core::ColumnSettings;
use greybel_transpiler::{BuildMode, TranspileOptions};

use super::Server;
use crate::error::Result;
use crate::{from_lsp, to_lsp};

impl Server {
    pub(in crate::server) fn code_action(
        &mut self,
        params: lsp::CodeActionParams,
    ) -> Result<Option<Vec<lsp::CodeActionResponse>>> {
        let Some(document) = self.document_analysis(&params.text_document.uri)? else {
            return Ok(None);
        };

        Ok(Some(
            document
                .analyzer
                .code_actions(&document.uri, from_lsp::range(params.range))
                .iter()
                .map(|action| lsp::CodeActionResponse::CodeAction(to_lsp::code_action(action)))
                .collect(),
        ))
    }

    pub(in crate::server) fn format_document(
        &mut self,
        params: lsp::DocumentFormattingParams,
    ) -> Result<Option<Vec<lsp::TextEdit>>> {
        let uri = Self::requested_uri(&params.text_document.uri)?;

        if !self.settings.formatter {
            return Ok(None);
        }

        let Some(file) = self.workspace.file(&uri) else {
            return Ok(None);
        };
        let parse = file.parse(self.workspace.db());
        let (Some(ast), []) = (parse.ast(), parse.errors()) else {
            return Ok(None);
        };
        let options = TranspileOptions {
            dialect: self.dialect,
            mode: BuildMode::Beautify(self.settings.beautify_options()),
            strict_mode: self.settings.strict_mode,
            retain_multiline_strings: true,
            ..TranspileOptions::default()
        };
        let Ok(beautified_text) = greybel_transpiler::emit(ast, &options) else {
            return Ok(None);
        };

        if beautified_text == ast.source() {
            return Ok(Some(Vec::new()));
        }

        Ok(Some(vec![lsp::TextEdit {
            range: lsp::Range {
                start: lsp::Position::default(),
                end: to_lsp::position(ast.lines().position(
                    u32::try_from(ast.source().len()).unwrap_or(u32::MAX),
                    ColumnSettings::default(),
                )),
            },
            new_text: beautified_text,
        }]))
    }
}
