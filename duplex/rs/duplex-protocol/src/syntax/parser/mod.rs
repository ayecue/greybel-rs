mod literals;
mod protocol;
mod types;

use super::lexer::{Lexer, Token, TokenKind};
use crate::error::DefinitionError;
use crate::model::{
    Constant, CoreUse, Definition, DocumentationRef, Span, Status, Tag, TagDeclaration, TagList,
    TagPlace, TypeDeclaration, TypeKind, Version,
};

pub(super) const USE: &str = "use";
pub(super) const CORE: &str = "core";
pub(super) const AS: &str = "as";
pub(super) const DEPRECATED_KEYWORD: &str = "deprecated";
pub(super) const UNSUPPORTED_KEYWORD: &str = "unsupported";
pub(super) const OPEN: &str = "open";
pub(super) const ACK: &str = "ack";
pub(super) const REFUSE: &str = "refuse";
pub(super) const PROTOCOL: &str = "protocol";
pub(super) const STRUCT: &str = "struct";
pub(super) const UNION: &str = "union";
pub(super) const ENUM: &str = "enum";
pub(super) const MESSAGE: &str = "message";
pub(super) const RENAMES: &str = "renames";
pub(crate) const RESULT: &str = "Result";
pub(crate) const STREAM: &str = "Stream";
pub(super) const LEFT_OUT: &str = "_";
pub(super) const FLATTEN: &str = "flatten";
pub(super) const CONST: &str = "const";
pub(super) const TAG: &str = "tag";
pub(super) const ON: &str = "on";
pub(super) const REPEATABLE: &str = "repeated";
pub(super) const SECURITY: &str = "security";
pub(super) const SIGNED_KEYWORD: &str = "signed";
pub(super) const FLAVOUR: &str = "flavour";
pub(super) const HEALTH_CHECK: &str = "health_check";
pub(super) const STREAM_WINDOW: &str = "stream_window";
pub(super) const TIMEOUT: &str = "timeout";
pub(super) const MAX_MESSAGE: &str = "max_message";
pub(super) const LIMIT: &str = "limit";
pub(super) const MATCH: &str = "match";
pub(super) const OFF: &str = "off";
pub(super) const INTERVAL: &str = "interval";
pub(super) const UNHEALTHY_THRESHOLD: &str = "unhealthy_threshold";
pub(super) const CLOSE_THRESHOLD: &str = "close_threshold";
pub(super) const TRUE: &str = "true";
pub(super) const FALSE: &str = "false";
pub(super) const NULL: &str = "null";
pub(super) const BOOL: &str = "bool";
pub(super) const CHAR: &str = "char";
pub(super) const STRING: &str = "string";
pub(super) const BYTES: &str = "bytes";
pub(super) const LIST: &str = "list";
pub(super) const RECORD: &str = "record";

pub(crate) struct Parser {
    tokens: Vec<Token>,
    position: usize,
    fields: u32,
    variants: u32,
    members: u32,
}

impl Parser {
    pub(crate) fn new(source: &str) -> Result<Self, DefinitionError> {
        Ok(Self {
            tokens: Lexer::new(source).tokens()?,
            position: 0,
            fields: 0,
            variants: 0,
            members: 0,
        })
    }

    pub(crate) fn parse(mut self) -> Result<Definition, DefinitionError> {
        let core = self.uses()?;
        let status = self.status()?;
        let mut protocol = None;
        let mut types = Vec::new();
        let mut constants = Vec::new();
        let mut tag_declarations = Vec::new();

        loop {
            let docs = self.docs();
            let tags = self.tags()?;
            let token = self.peek().clone();

            match &token.kind {
                TokenKind::End if docs.is_empty() && tags.is_empty() => {
                    return Ok(Definition {
                        core,
                        status,
                        protocol,
                        types,
                        constants,
                        tags: tag_declarations,
                    });
                }
                TokenKind::Ident(keyword)
                    if [CONST, TAG].contains(&keyword.as_str()) && !tags.is_empty() =>
                {
                    return Err(DefinitionError::new(
                        token.span,
                        format!("a `{keyword}` takes no tags"),
                    ));
                }
                TokenKind::Ident(keyword) if keyword == TAG => {
                    self.bump();
                    tag_declarations.push(self.tag_declaration(docs, token.span)?);
                }
                TokenKind::Ident(keyword) if keyword == PROTOCOL => {
                    self.bump();
                    if protocol.is_some() {
                        return Err(DefinitionError::new(
                            token.span,
                            "a definition declares one protocol at most",
                        ));
                    }

                    protocol = Some(self.protocol(docs, tags, token.span)?);
                }
                TokenKind::Ident(keyword) if keyword == CONST => {
                    self.bump();

                    let name = self.name()?;

                    self.expect(':')?;

                    let ty = self.type_ref()?;

                    self.expect('=')?;

                    let value = self.literal()?;

                    self.expect(';')?;
                    constants.push(Constant {
                        docs,
                        name,
                        ty,
                        value,
                        span: token.span,
                    });
                }
                TokenKind::Ident(keyword) if keyword == STRUCT => {
                    self.bump();

                    let name = self.name()?;
                    let kind = TypeKind::Struct(self.fields()?);

                    types.push(TypeDeclaration {
                        docs,
                        tags,
                        name,
                        kind,
                        span: token.span,
                    });
                }
                TokenKind::Ident(keyword) if keyword == UNION => {
                    self.bump();

                    let name = self.name()?;
                    let kind = TypeKind::Union(self.variants()?);

                    types.push(TypeDeclaration {
                        docs,
                        tags,
                        name,
                        kind,
                        span: token.span,
                    });
                }
                TokenKind::Ident(keyword) if keyword == ENUM => {
                    self.bump();

                    let name = self.name()?;
                    let kind = TypeKind::Enumeration(self.members()?);

                    types.push(TypeDeclaration {
                        docs,
                        tags,
                        name,
                        kind,
                        span: token.span,
                    });
                }
                _ => {
                    return Err(
                        self.token_error("`protocol`, `const`, `tag`, `struct`, `union` or `enum`")
                    );
                }
            }
        }
    }

    /// Parses `tag name: Type on place, place repeated;` after the `tag` keyword.
    fn tag_declaration(
        &mut self,
        docs: DocumentationRef,
        span: Span,
    ) -> Result<TagDeclaration, DefinitionError> {
        let name = self.name()?;
        let ty = if self.consume(':') {
            Some(self.type_ref()?)
        } else {
            None
        };

        if !self.consume_keyword(ON) {
            return Err(self.token_error("`:` or `on`"));
        }

        let mut places = Vec::new();

        loop {
            let place_span = self.peek().span;
            let keyword = self.name()?;
            let Some(place) = TagPlace::from_keyword(&keyword) else {
                let keywords: Vec<String> = TagPlace::ALL
                    .iter()
                    .map(|place| format!("`{}`", place.keyword()))
                    .collect();

                return Err(DefinitionError::new(
                    place_span,
                    format!("a tag is put on {}, not `{keyword}`", keywords.join(", ")),
                ));
            };

            if places.contains(&place) {
                return Err(DefinitionError::new(
                    place_span,
                    format!("the tag `{name}` names `{keyword}` twice"),
                ));
            }

            places.push(place);
            if !self.consume(',') {
                break;
            }
        }

        let repeatable = self.consume_keyword(REPEATABLE);

        self.expect(';')?;

        Ok(TagDeclaration {
            docs,
            name,
            ty,
            places,
            repeatable,
            span,
        })
    }

    /// Parses every `@name` and `@name(value)` annotation before an item.
    fn tags(&mut self) -> Result<TagList, DefinitionError> {
        let mut tags = TagList::default();

        while self.is('@') {
            let span = self.peek().span;

            self.bump();

            let name = self.name()?;
            let value = if self.consume('(') {
                let value = self.literal()?;

                self.expect(')')?;

                Some(value)
            } else {
                None
            };

            tags.push(Tag { name, value, span });
        }

        Ok(tags)
    }

    fn uses(&mut self) -> Result<Option<CoreUse>, DefinitionError> {
        let mut core = None;

        while self.is_keyword(USE) {
            let span = self.peek().span;

            self.bump();

            let name = self.name()?;
            let version = self.version()?;

            if self.consume_keyword(AS) {
                self.name()?;
            }

            self.expect(';')?;
            if name != CORE {
                return Err(DefinitionError::new(
                    span,
                    format!("`use` names `{CORE}` only; `{name}` can't be used yet"),
                ));
            }

            if core.is_some() {
                return Err(DefinitionError::new(span, "the core is used once"));
            }

            core = Some(CoreUse { version, span });
        }

        Ok(core)
    }

    fn status(&mut self) -> Result<Status, DefinitionError> {
        if self.consume_keyword(DEPRECATED_KEYWORD) {
            let note = self.text()?;

            self.expect(';')?;

            return Ok(Status::Deprecation(note));
        }

        if self.consume_keyword(UNSUPPORTED_KEYWORD) {
            self.expect(';')?;

            return Ok(Status::Unavailable);
        }

        Ok(Status::Current)
    }

    fn version(&mut self) -> Result<Version, DefinitionError> {
        let span = self.peek().span;
        let major = self.number()?;

        self.expect('.')?;

        let minor = self.number()?;
        let part = |number: u32| {
            u16::try_from(number).map_err(|_| {
                DefinitionError::new(span, "a version's major and minor are below 2^16")
            })
        };

        Ok(Version::new(part(major)?, part(minor)?))
    }

    fn docs(&mut self) -> DocumentationRef {
        let mut docs = DocumentationRef::default();

        while let TokenKind::Doc(line) = &self.peek().kind {
            docs.push(line.clone());
            self.bump();
        }

        docs
    }

    fn name(&mut self) -> Result<String, DefinitionError> {
        match &self.peek().kind {
            TokenKind::Ident(name) => {
                let name = name.clone();

                self.bump();

                Ok(name)
            }
            _ => Err(self.token_error("a name")),
        }
    }

    fn text(&mut self) -> Result<String, DefinitionError> {
        match &self.peek().kind {
            TokenKind::Text(text) => {
                let text = text.clone();

                self.bump();

                Ok(text)
            }
            _ => Err(self.token_error("a text in quotes")),
        }
    }

    fn is_keyword(&self, keyword: &str) -> bool {
        matches!(&self.peek().kind, TokenKind::Ident(ident) if ident == keyword)
    }

    fn consume_keyword(&mut self, keyword: &str) -> bool {
        let found = self.is_keyword(keyword);

        if found {
            self.bump();
        }

        found
    }

    fn expect(&mut self, punct: char) -> Result<(), DefinitionError> {
        if self.consume(punct) {
            Ok(())
        } else {
            Err(self.token_error(&format!("`{punct}`")))
        }
    }

    fn is(&self, punct: char) -> bool {
        self.peek().kind == TokenKind::Punct(punct)
    }

    fn consume(&mut self, punct: char) -> bool {
        let present = self.is(punct);

        if present {
            self.bump();
        }

        present
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.position.min(self.tokens.len() - 1)]
    }

    fn bump(&mut self) {
        self.position += 1;
    }

    fn token_error(&self, expectation: &str) -> DefinitionError {
        let token = self.peek();
        let actual = match &token.kind {
            TokenKind::Ident(name) => format!("`{name}`"),
            TokenKind::Text(text) => format!("\"{text}\""),
            TokenKind::Number(digits) => format!("`{digits}`"),
            TokenKind::Doc(_) => "a doc comment".to_owned(),
            TokenKind::Punct(punct) => format!("`{punct}`"),
            TokenKind::Arrow => "`->`".to_owned(),
            TokenKind::End => "the end of the file".to_owned(),
        };

        DefinitionError::new(
            token.span,
            format!("expected {expectation}, found {actual}"),
        )
    }
}
