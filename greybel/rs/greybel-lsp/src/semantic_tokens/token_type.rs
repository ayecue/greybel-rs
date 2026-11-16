#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub(super) enum TokenType {
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

impl TokenType {
    pub(super) const ALL: [TokenType; 12] = [
        TokenType::Keyword,
        TokenType::String,
        TokenType::Number,
        TokenType::Variable,
        TokenType::Property,
        TokenType::Function,
        TokenType::Parameter,
        TokenType::Operator,
        TokenType::Comment,
        TokenType::Constant,
        TokenType::Punctuator,
        TokenType::Type,
    ];

    pub(super) fn name(self) -> &'static str {
        match self {
            TokenType::Keyword => "keyword",
            TokenType::String => "string",
            TokenType::Number => "number",
            TokenType::Variable => "variable",
            TokenType::Property => "property",
            TokenType::Function => "function",
            TokenType::Parameter => "parameter",
            TokenType::Operator => "operator",
            TokenType::Comment => "comment",
            TokenType::Constant => "constant",
            TokenType::Punctuator => "punctuator",
            TokenType::Type => "type",
        }
    }
}
