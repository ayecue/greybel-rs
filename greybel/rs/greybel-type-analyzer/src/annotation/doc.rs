use greybel_symbols::annotations::{TAG_PREFIX, Tag};
use greybel_symbols::names::SELF;

use super::TypeExpr;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DocComment {
    pub description: Option<String>,
    pub examples: Vec<String>,
    pub params: Vec<ParamDoc>,
    pub returns: Option<TypeExpr>,
    pub returns_description: Option<String>,
    pub deprecation: Option<String>,
    pub defined_type: Option<TypeExpr>,
    pub mutated_parameter: Option<String>,
    pub templates: Vec<TemplateDoc>,
    pub types: Vec<TypeDoc>,
    pub aliases: Vec<AliasDoc>,
    pub invalid_tags: Vec<InvalidTag>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AliasDoc {
    pub name: String,
    pub type_parameters: Vec<String>,
    pub ty: TypeExpr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamDoc {
    pub name: String,
    pub name_position: TextPosition,
    pub ty: Option<TypeExpr>,
    pub description: Option<String>,
    pub optional: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TextPosition {
    pub line: usize,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateDoc {
    pub name: String,
    pub bound: Option<TypeExpr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeDoc {
    pub name: String,
    pub line: usize,
    pub virtual_type: bool,
    pub type_parameters: Vec<String>,
    pub extends: Option<TypeExpr>,
    pub properties: Vec<PropertyDoc>,
    pub functions: Vec<FunctionDoc>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyDoc {
    pub line: usize,
    pub path: Vec<String>,
    pub ty: TypeExpr,
    pub description: Option<String>,
    pub optional: bool,
    pub readonly: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionDoc {
    pub line: usize,
    pub name: String,
    pub params: Vec<ParamDoc>,
    pub returns: Option<TypeExpr>,
    pub mutated_parameter: Option<String>,
    pub description: Option<String>,
    pub examples: Vec<String>,
}

enum TagType {
    Present(TypeExpr),
    Missing,
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidTag {
    pub line: usize,
    pub message: String,
}

impl DocComment {
    #[must_use]
    pub fn param(&self, name: &str) -> Option<&ParamDoc> {
        self.params.iter().find(|param| param.name == name)
    }

    #[must_use]
    pub fn parse(lines: &[&str]) -> Self {
        let mut reader = Reader::default();

        for (index, line) in lines.iter().enumerate() {
            reader.line_start = line.len() - line.trim_start().len();
            reader.read_line(index, line.trim());
        }

        reader.finish()
    }
}

#[derive(Clone, Copy)]
enum Section {
    Description,
    Example,
    Param(usize),
    Returns,
    Deprecation,
    Type(usize),
    Function(usize, usize),
    FunctionExample(usize, usize),
    Other,
}

struct Reader {
    doc: DocComment,
    line_start: usize,
    description: Vec<String>,
    section: Section,
    mutations: Vec<(usize, Option<(usize, usize)>)>,
}

impl Default for Reader {
    fn default() -> Self {
        Self {
            doc: DocComment::default(),
            line_start: 0,
            description: Vec::new(),
            section: Section::Description,
            mutations: Vec::new(),
        }
    }
}

impl Reader {
    fn read_line(&mut self, index: usize, line: &str) {
        let Some(tag_text) = line.strip_prefix(TAG_PREFIX) else {
            self.read_continuation(line);

            return;
        };
        let (tag, remainder) = tag_text
            .split_once(char::is_whitespace)
            .unwrap_or((tag_text, ""));
        let remainder = remainder.trim();
        let section = match Tag::parse(tag) {
            Some(Tag::Param | Tag::Params) => self.read_param(index, line, remainder),
            Some(Tag::Return | Tag::Returns) => self.read_returns(index, remainder),
            Some(Tag::Mutates) => self.read_mutates(index, remainder),
            Some(tag @ Tag::Define) => {
                self.doc.defined_type = self.required_type(index, tag, remainder);

                Some(Section::Other)
            }
            Some(tag @ Tag::Type) if remainder.starts_with('{') => {
                self.doc.defined_type = self.required_type(index, tag, remainder);

                Some(Section::Other)
            }
            Some(tag @ (Tag::Type | Tag::VirtualType)) => {
                self.read_type_declaration(index, tag, remainder)
            }
            Some(Tag::Template) => self.read_template(index, remainder),
            Some(Tag::Typedef) => self.read_typedef(index, remainder),
            Some(Tag::Extends) => self.read_extends(index, remainder),
            Some(Tag::Property | Tag::Prop) => self.read_property(index, remainder),
            Some(Tag::Function) => self.read_function(index, remainder),
            Some(Tag::Description) => Some(self.read_description(remainder)),
            Some(Tag::Example) => Some(self.read_example(remainder)),
            Some(Tag::Deprecated) => {
                self.doc.deprecation = Some(remainder.to_owned());

                Some(Section::Deprecation)
            }
            Some(Tag::Ignore | Tag::IgnoreFile) => Some(Section::Other),
            None => {
                self.description.push(line.to_owned());

                Some(Section::Description)
            }
        };

        self.section = section.unwrap_or(Section::Other);
    }

    fn read_continuation(&mut self, line: &str) {
        match self.section {
            Section::Description => self.description.push(line.to_owned()),
            Section::Example => self.doc.examples.push(line.to_owned()),
            Section::Param(index) => {
                Self::append_line(&mut self.doc.params[index].description, line)
            }
            Section::Returns => Self::append_line(&mut self.doc.returns_description, line),
            Section::Deprecation => {
                let text = self.doc.deprecation.get_or_insert_default();

                if !text.is_empty() {
                    text.push('\n');
                }

                text.push_str(line);
            }
            Section::Type(index) => Self::append_line(&mut self.doc.types[index].description, line),
            Section::Function(ty, function) => {
                Self::append_line(
                    &mut self.doc.types[ty].functions[function].description,
                    line,
                );
            }
            Section::FunctionExample(ty, function) => {
                self.doc.types[ty].functions[function]
                    .examples
                    .push(line.to_owned());
            }
            Section::Other => {}
        }
    }

    fn read_param(&mut self, index: usize, line: &str, remainder: &str) -> Option<Section> {
        let (ty, remainder) = TypeExpr::parse_braced_prefix(remainder);
        let ty = match self.type_or_invalid(index, Tag::Param, ty) {
            TagType::Present(ty) => Some(ty),
            TagType::Missing => None,
            TagType::Invalid => return None,
        };
        let (name, text) = remainder
            .split_once(char::is_whitespace)
            .unwrap_or((remainder, ""));
        let optional = name.starts_with('[');
        let name = name.trim_matches(['[', ']']);

        if name.is_empty() {
            return self.report_invalid(index, &Self::tag_message(Tag::Param, "requires a name"));
        }

        let start = self.line_start + (name.as_ptr().addr() - line.as_ptr().addr());
        let param = ParamDoc {
            name: name.to_owned(),
            name_position: TextPosition {
                line: index,
                start,
                end: start + name.len(),
            },
            ty,
            description: Self::trimmed_text(text),
            optional,
        };

        if let Some((ty, function)) = self.function_section() {
            self.doc.types[ty].functions[function].params.push(param);

            return Some(Section::Function(ty, function));
        }

        self.doc.params.push(param);

        Some(Section::Param(self.doc.params.len() - 1))
    }

    fn read_returns(&mut self, index: usize, remainder: &str) -> Option<Section> {
        let (ty, text) = TypeExpr::parse_braced_prefix(remainder);
        let ty = match self.type_or_invalid(index, Tag::Returns, ty) {
            TagType::Present(ty) => Some(ty),
            TagType::Missing => None,
            TagType::Invalid => return None,
        };

        if let Some((owner, function)) = self.function_section() {
            let function_doc = &mut self.doc.types[owner].functions[function];

            function_doc.returns = ty.or(function_doc.returns.take());
            Self::append_line(
                &mut function_doc.description,
                text.trim_start_matches('-').trim(),
            );

            return Some(Section::Function(owner, function));
        }

        self.doc.returns = ty.or(self.doc.returns.take());
        self.doc.returns_description = Self::trimmed_text(text);

        Some(Section::Returns)
    }

    fn read_mutates(&mut self, index: usize, remainder: &str) -> Option<Section> {
        let parameter = remainder.split_whitespace().next().unwrap_or(SELF);

        if !parameter.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return self.report_invalid(
                index,
                &Self::tag_message(Tag::Mutates, "takes a parameter name"),
            );
        }

        if let Some((owner, function)) = self.function_section() {
            self.doc.types[owner].functions[function].mutated_parameter =
                Some(parameter.to_owned());
            self.mutations.push((index, Some((owner, function))));

            return Some(Section::Function(owner, function));
        }

        self.doc.mutated_parameter = Some(parameter.to_owned());
        self.mutations.push((index, None));

        Some(Section::Other)
    }

    fn read_type_declaration(
        &mut self,
        index: usize,
        tag: Tag,
        remainder: &str,
    ) -> Option<Section> {
        let name = remainder.split_whitespace().next().unwrap_or_default();
        let Some(TypeExpr::Name { name, arguments }) = TypeExpr::parse(name) else {
            return self.report_invalid(index, &Self::tag_message(tag, "requires a type name"));
        };
        let mut type_parameters = Vec::new();

        for argument in arguments {
            match argument {
                TypeExpr::Name { name, arguments } if arguments.is_empty() => {
                    type_parameters.push(name);
                }
                _ => {
                    return self.report_invalid(
                        index,
                        &Self::tag_message(tag, "parameters must be names"),
                    );
                }
            }
        }

        if type_parameters.is_empty() {
            type_parameters = self
                .doc
                .templates
                .iter()
                .map(|template| template.name.clone())
                .collect();
        }

        self.doc.types.push(TypeDoc {
            name,
            line: index,
            virtual_type: tag == Tag::VirtualType,
            type_parameters,
            extends: None,
            properties: Vec::new(),
            functions: Vec::new(),
            description: None,
        });

        Some(Section::Type(self.doc.types.len() - 1))
    }

    fn read_template(&mut self, index: usize, remainder: &str) -> Option<Section> {
        let (names, bound) = match remainder.split_once(" extends ") {
            Some((names, bound)) => (names, Some(bound.trim())),
            None => (remainder, None),
        };
        let bound = match bound {
            Some(bound) => match TypeExpr::parse(bound.trim_matches(['{', '}'])) {
                Some(bound) => Some(bound),
                None => {
                    return self.report_invalid(
                        index,
                        &Self::tag_message(Tag::Template, "has an invalid bound"),
                    );
                }
            },
            None => None,
        };

        if names.contains('=') {
            return self.report_invalid(index, "Generic defaults are not supported.");
        }

        for name in names.split(',') {
            let name = name.trim();

            if name.is_empty()
                || !name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
            {
                return self.report_invalid(
                    index,
                    &Self::tag_message(Tag::Template, "requires parameter names"),
                );
            }

            self.doc.templates.push(TemplateDoc {
                name: name.to_owned(),
                bound: bound.clone(),
            });
        }

        Some(Section::Other)
    }

    fn read_typedef(&mut self, index: usize, remainder: &str) -> Option<Section> {
        let (expr, remainder) = TypeExpr::parse_braced_prefix(remainder);
        let Ok(Some(ty)) = expr else {
            return self.report_invalid(index, &Self::tag_message(Tag::Typedef, "requires a type"));
        };
        let declaration = remainder.split_whitespace().next().unwrap_or_default();
        let (name, parameters) = match declaration.split_once('<') {
            Some((name, parameters)) => (name, Some(parameters.trim_end_matches('>'))),
            None => (declaration, None),
        };

        if name.is_empty() {
            return self.report_invalid(index, &Self::tag_message(Tag::Typedef, "requires a name"));
        }

        let type_parameters = match parameters {
            Some(parameters) => parameters
                .split(',')
                .map(|parameter| parameter.trim().to_owned())
                .filter(|parameter| !parameter.is_empty())
                .collect(),
            None => self
                .doc
                .templates
                .iter()
                .map(|template| template.name.clone())
                .collect(),
        };

        self.doc.aliases.push(AliasDoc {
            name: name.to_owned(),
            type_parameters,
            ty,
        });

        Some(Section::Other)
    }

    fn read_extends(&mut self, index: usize, remainder: &str) -> Option<Section> {
        let ty = self.type_section(Tag::Extends, index)?;
        let expr = if remainder.starts_with('{') {
            TypeExpr::parse_braced_prefix(remainder).0.ok().flatten()
        } else {
            TypeExpr::parse(remainder)
        };
        let Some(expr) = expr else {
            return self.report_invalid(index, &Self::tag_message(Tag::Extends, "requires a type"));
        };

        self.doc.types[ty].extends = Some(expr);

        Some(Section::Type(ty))
    }

    fn read_property(&mut self, index: usize, remainder: &str) -> Option<Section> {
        let ty = self.type_section(Tag::Property, index)?;
        let braced_remainder = remainder
            .strip_prefix("{readonly ")
            .map(|remainder| format!("{{{remainder}"));
        let (expr, remainder) =
            TypeExpr::parse_braced_prefix(braced_remainder.as_deref().unwrap_or(remainder));
        let Ok(Some(expr)) = expr else {
            return self
                .report_invalid(index, &Self::tag_message(Tag::Property, "requires a type"));
        };
        let (path, text) = remainder
            .split_once(char::is_whitespace)
            .unwrap_or((remainder, ""));
        let optional = path.starts_with('[');
        let path: Vec<String> = path
            .trim_matches(['[', ']'])
            .split('.')
            .map(str::to_owned)
            .collect();

        if path.iter().any(String::is_empty) {
            return self
                .report_invalid(index, &Self::tag_message(Tag::Property, "requires a name"));
        }

        self.doc.types[ty].properties.push(PropertyDoc {
            line: index,
            path,
            ty: expr,
            description: Self::trimmed_text(text),
            optional,
            readonly: braced_remainder.is_some(),
        });

        Some(Section::Type(ty))
    }

    fn read_function(&mut self, index: usize, remainder: &str) -> Option<Section> {
        let ty = self.type_section(Tag::Function, index)?;
        let name = remainder.split_whitespace().next().unwrap_or_default();

        if name.is_empty() {
            return self
                .report_invalid(index, &Self::tag_message(Tag::Function, "requires a name"));
        }

        let functions = &mut self.doc.types[ty].functions;

        functions.push(FunctionDoc {
            line: index,
            name: name.to_owned(),
            params: Vec::new(),
            returns: None,
            mutated_parameter: None,
            description: None,
            examples: Vec::new(),
        });

        Some(Section::Function(ty, functions.len() - 1))
    }

    fn read_description(&mut self, remainder: &str) -> Section {
        match self.section {
            Section::Function(ty, function) | Section::FunctionExample(ty, function) => {
                Self::append_line(
                    &mut self.doc.types[ty].functions[function].description,
                    remainder,
                );

                Section::Function(ty, function)
            }
            Section::Type(ty) => {
                Self::append_line(&mut self.doc.types[ty].description, remainder);

                Section::Type(ty)
            }
            _ => {
                self.description.push(remainder.to_owned());

                Section::Description
            }
        }
    }

    fn read_example(&mut self, remainder: &str) -> Section {
        if let Some((ty, function)) = self.function_section() {
            if !remainder.is_empty() {
                self.doc.types[ty].functions[function]
                    .examples
                    .push(remainder.to_owned());
            }

            return Section::FunctionExample(ty, function);
        }

        if !remainder.is_empty() {
            self.doc.examples.push(remainder.to_owned());
        }

        Section::Example
    }

    fn type_section(&mut self, tag: Tag, index: usize) -> Option<usize> {
        match self.section {
            Section::Type(ty) | Section::Function(ty, _) | Section::FunctionExample(ty, _) => {
                Some(ty)
            }
            _ => self.report_invalid(
                index,
                &Self::tag_message(
                    tag,
                    &format!(
                        "needs a {TAG_PREFIX}{} or {TAG_PREFIX}{} before it",
                        Tag::Type,
                        Tag::VirtualType
                    ),
                ),
            ),
        }
    }

    fn function_section(&self) -> Option<(usize, usize)> {
        match self.section {
            Section::Function(ty, function) | Section::FunctionExample(ty, function) => {
                Some((ty, function))
            }
            _ => None,
        }
    }

    fn type_or_invalid(
        &mut self,
        index: usize,
        tag: Tag,
        ty: Result<Option<TypeExpr>, ()>,
    ) -> TagType {
        match ty {
            Ok(Some(ty)) => TagType::Present(ty),
            Ok(None) => TagType::Missing,
            Err(()) => {
                self.report_invalid::<()>(index, &Self::tag_message(tag, "has an invalid type"));

                TagType::Invalid
            }
        }
    }

    fn required_type(&mut self, index: usize, tag: Tag, remainder: &str) -> Option<TypeExpr> {
        match TypeExpr::parse_braced_prefix(remainder).0 {
            Ok(Some(ty)) => Some(ty),
            _ => self.report_invalid(index, &Self::tag_message(tag, "requires a type")),
        }
    }

    fn tag_message(tag: Tag, message: &str) -> String {
        format!("{TAG_PREFIX}{tag} {message}.")
    }

    fn report_invalid<T>(&mut self, line: usize, message: &str) -> Option<T> {
        self.doc.invalid_tags.push(InvalidTag {
            line,
            message: message.to_owned(),
        });

        None
    }

    fn finish(mut self) -> DocComment {
        for (line, function) in std::mem::take(&mut self.mutations) {
            let (parameter, params) = match function {
                Some((owner, function)) => {
                    let function = &self.doc.types[owner].functions[function];

                    (function.mutated_parameter.as_deref(), &function.params)
                }
                None => (self.doc.mutated_parameter.as_deref(), &self.doc.params),
            };
            let Some(parameter) = parameter.filter(|&parameter| parameter != SELF) else {
                continue;
            };

            if !params.iter().any(|param| param.name == parameter) {
                let message = format!(
                    "{TAG_PREFIX}{} \"{parameter}\" names no {TAG_PREFIX}{}.",
                    Tag::Mutates,
                    Tag::Param
                );

                self.report_invalid::<()>(line, &message);
            }
        }

        let description = self.description.join("\n");
        let description = description.trim();

        if !description.is_empty() {
            self.doc.description = Some(description.to_owned());
        }

        self.doc
    }

    fn trimmed_text(text: &str) -> Option<String> {
        let text = text.trim().trim_start_matches('-').trim();

        (!text.is_empty()).then(|| text.to_owned())
    }

    fn append_line(text: &mut Option<String>, line: &str) {
        if line.is_empty() {
            return;
        }

        match text {
            Some(text) => {
                text.push('\n');
                text.push_str(line);
            }
            None => *text = Some(line.to_owned()),
        }
    }
}
