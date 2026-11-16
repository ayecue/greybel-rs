use crate::Keyword;

pub const KEYWORDS: &[Keyword] = &[
    Keyword::Envar,
    Keyword::Import,
    Keyword::Include,
    Keyword::ImportWithComment,
    Keyword::IncludeWithComment,
    Keyword::Debugger,
    Keyword::Line,
    Keyword::Filename,
    Keyword::Inject,
    Keyword::IfEnv,
    Keyword::ElseEnv,
    Keyword::EndIfEnv,
    Keyword::EndEnum,
    Keyword::EndSwitch,
    Keyword::EndClass,
    Keyword::EndType,
];
