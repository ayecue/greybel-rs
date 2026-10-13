use meta_utils::{DEFAULT_LANGUAGE, LocalizedDefinition, Signature};

use crate::anchors::AnchorTable;
use crate::api;
use crate::example::ExampleHighlighter;
use crate::markdown::MarkdownReader;
use crate::protocol::{Argument, Block, Literal, Member, MemberKind, Section, TypeText};

/// Builds type sections with resolved links and highlighted examples.
pub(crate) struct SectionWriter {
    anchors: AnchorTable,
    highlighter: ExampleHighlighter,
}

impl SectionWriter {
    pub(crate) fn new() -> Self {
        Self {
            anchors: AnchorTable::of(api::container()),
            highlighter: ExampleHighlighter::new(),
        }
    }

    /// Parses Markdown `text` into blocks and resolves internal and external links.
    pub(crate) fn blocks(&self, text: &str) -> Vec<Block> {
        MarkdownReader::new(&self.anchors, &self.highlighter).read(text)
    }

    /// Builds every type section in [`api::types`] order.
    pub(crate) fn sections(&self) -> Vec<Section> {
        api::types()
            .into_iter()
            .map(|signature| self.section(signature))
            .collect()
    }

    fn section(&self, signature: &Signature) -> Section {
        let type_description = signature
            .descriptions(DEFAULT_LANGUAGE)
            .and_then(|table| table.type_description());
        let mut names: Vec<&String> = signature.definitions().keys().collect();

        names.sort();

        Section {
            name: signature.type_name().to_owned(),
            extends: signature.extends().map(str::to_owned),
            description: type_description
                .map(|description| self.blocks(description.text()))
                .unwrap_or_default(),
            example: type_description
                .and_then(|description| description.example())
                .map(|lines| self.highlighter.tokens(lines))
                .unwrap_or_default(),
            members: names
                .into_iter()
                .filter_map(|name| signature.definition(name, DEFAULT_LANGUAGE))
                .map(|member| self.member(&member))
                .collect(),
        }
    }

    fn member(&self, member: &LocalizedDefinition<'_>) -> Member {
        let definition = member.definition();
        let kind = match definition.as_function() {
            Some(function) => MemberKind::Function {
                arguments: function
                    .arguments()
                    .iter()
                    .map(|argument| Argument {
                        name: argument.label().to_owned(),
                        optional: argument.is_optional(),
                        types: argument.types().iter().map(TypeText::of).collect(),
                        default: argument.default().map(Literal::from),
                    })
                    .collect(),
                returns: function.returns().iter().map(TypeText::of).collect(),
                return_variations: api::container()
                    .variations()
                    .resolve(function.return_variations())
                    .expect("the bundled data's variation groups exist (its data tests check them)")
                    .iter()
                    .map(Literal::from)
                    .collect(),
            },
            None => MemberKind::Value {
                r#type: TypeText::of(definition.type_meta()),
            },
        };

        Member {
            name: member.name().to_owned(),
            tags: definition.tags().to_vec(),
            kind,
            description: member
                .description()
                .map(|text| self.blocks(text))
                .unwrap_or_default(),
            example: member
                .example()
                .map(|lines| self.highlighter.tokens(lines))
                .unwrap_or_default(),
        }
    }
}
