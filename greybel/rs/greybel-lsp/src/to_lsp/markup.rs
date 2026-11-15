use gen_lsp_types as lsp;
use greybel_type_analyzer::{Documentation, Hover, SignatureHelp};

use super::range;

pub struct Markup {
    pub markdown: bool,
    pub language: &'static str,
}

impl Markup {
    #[must_use]
    pub fn hover(&self, hover: &Hover) -> lsp::Hover {
        let mut sections = vec![self.code(&hover.text)];

        if let Some(documentation) = &hover.documentation {
            sections.extend(self.documentation_sections(documentation));
        }

        lsp::Hover {
            contents: lsp::Contents::MarkupContent(self.content(self.join(&sections))),
            range: Some(range(hover.range)),
        }
    }

    #[must_use]
    pub fn signature_help(&self, help: &SignatureHelp) -> lsp::SignatureHelp {
        let parameters = help
            .parameters
            .iter()
            .map(|parameter| lsp::ParameterInformation {
                label: lsp::ParameterInformationLabel::String(parameter.label.clone()),
                documentation: parameter
                    .documentation
                    .as_ref()
                    .map(|text| lsp::Documentation::MarkupContent(self.content(text.clone()))),
            })
            .collect();
        let active = u32::try_from(help.active_parameter).unwrap_or(u32::MAX);

        lsp::SignatureHelp {
            signatures: vec![lsp::SignatureInformation {
                label: help.label.clone(),
                documentation: help
                    .documentation
                    .as_ref()
                    .map(|documentation| self.documentation(documentation)),
                parameters: Some(parameters),
                active_parameter: None,
            }],
            active_signature: Some(0),
            active_parameter: Some(lsp::ActiveParameter::Int(active)),
        }
    }

    #[must_use]
    pub fn documentation(&self, documentation: &Documentation) -> lsp::Documentation {
        let text = self.join(&self.documentation_sections(documentation));

        lsp::Documentation::MarkupContent(self.content(text))
    }

    fn content(&self, value: String) -> lsp::MarkupContent {
        lsp::MarkupContent {
            kind: if self.markdown {
                lsp::MarkupKind::Markdown
            } else {
                lsp::MarkupKind::PlainText
            },
            value,
        }
    }

    fn code(&self, code: &str) -> String {
        if self.markdown {
            format!("```{}\n{code}\n```", self.language)
        } else {
            code.to_owned()
        }
    }

    fn documentation_sections(&self, documentation: &Documentation) -> Vec<String> {
        let mut sections = Vec::new();

        if let Some(description) = &documentation.description {
            sections.push(description.clone());
        }

        if !documentation.examples.is_empty() {
            let heading = if self.markdown {
                "#### Examples"
            } else {
                "Examples:"
            };

            sections.push(format!(
                "{heading}\n{}",
                self.code(&documentation.examples.join("\n"))
            ));
        }

        sections
    }

    fn join(&self, sections: &[String]) -> String {
        sections.join(if self.markdown { "\n\n---\n\n" } else { "\n\n" })
    }
}
