/// What a highlighted token is. The discriminants are the LSP semantic token type indexes of [`HighlightKind::ALL`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum HighlightKind {
    Keyword,
    String,
    Number,
    Variable,
    Property,
    Function,
    Parameter,
    Operator,
    Comment,
    Constant,
    Punctuator,
    Type,
}

impl HighlightKind {
    pub const ALL: [HighlightKind; 12] = [
        HighlightKind::Keyword,
        HighlightKind::String,
        HighlightKind::Number,
        HighlightKind::Variable,
        HighlightKind::Property,
        HighlightKind::Function,
        HighlightKind::Parameter,
        HighlightKind::Operator,
        HighlightKind::Comment,
        HighlightKind::Constant,
        HighlightKind::Punctuator,
        HighlightKind::Type,
    ];

    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            HighlightKind::Keyword => "keyword",
            HighlightKind::String => "string",
            HighlightKind::Number => "number",
            HighlightKind::Variable => "variable",
            HighlightKind::Property => "property",
            HighlightKind::Function => "function",
            HighlightKind::Parameter => "parameter",
            HighlightKind::Operator => "operator",
            HighlightKind::Comment => "comment",
            HighlightKind::Constant => "constant",
            HighlightKind::Punctuator => "punctuator",
            HighlightKind::Type => "type",
        }
    }
}
