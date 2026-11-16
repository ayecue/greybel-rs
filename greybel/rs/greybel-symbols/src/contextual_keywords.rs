pub const FROM: &str = "from";
pub const LET: &str = "let";
pub const CONST: &str = "const";
pub const ENUM: &str = "enum";
pub const ASSERT: &str = "assert";
pub const AS: &str = "as";
pub const SWITCH: &str = "switch";
pub const CASE: &str = "case";
pub const IS: &str = "is";
pub const EXPORT: &str = "export";
pub const CLASS: &str = "class";
pub const EXTENDS: &str = "extends";
pub const IMPLEMENTS: &str = "implements";
pub const PRIVATE: &str = "private";
pub const STATIC: &str = "static";
pub const READONLY: &str = "readonly";
pub const CONSTRUCTOR: &str = "constructor";
pub const CREATE: &str = "create";
pub const TYPE: &str = "type";
pub const NO_EVAL: &str = "no-eval";
pub const NO_EMIT: &str = "no-emit";
pub const OVERRIDE: &str = "override";

/// Contextual keywords that act as statement keywords; excludes member names (`constructor`, `create`) and
/// `import_code` flags (`no-eval`, `no-emit`, `override`).
pub const STATEMENT_KEYWORDS: &[&str] = &[
    FROM, LET, CONST, ENUM, ASSERT, AS, SWITCH, CASE, IS, EXPORT, CLASS, EXTENDS, IMPLEMENTS,
    PRIVATE, STATIC, READONLY, TYPE,
];
