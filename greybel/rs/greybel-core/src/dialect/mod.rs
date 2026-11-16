mod features;
mod grammar;
mod greybel;
mod greyscript;
mod miniscript;
mod rules;
mod version;

pub use features::SyntaxFeatures;
pub use grammar::Grammar;
pub(crate) use rules::{LexerRuleTable, RuleTable};
pub use version::Version;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Dialect {
    #[default]
    MiniScript,
    Greybel,
    GreyScript,
}

impl Dialect {
    pub const ALL: [Self; 3] = [Self::MiniScript, Self::Greybel, Self::GreyScript];

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::MiniScript => "miniscript",
            Self::Greybel => "greybel",
            Self::GreyScript => "greyscript",
        }
    }

    #[must_use]
    pub const fn default_version(self) -> Version {
        match self {
            Self::MiniScript | Self::Greybel => Version::Latest,
            Self::GreyScript => Version::V1_6,
        }
    }

    #[must_use]
    pub const fn has_directives(self) -> bool {
        !matches!(self, Self::MiniScript)
    }

    pub(crate) const fn rules(self, strict: bool) -> &'static RuleTable {
        match (self, strict) {
            (Self::MiniScript, false) => &miniscript::RULES,
            (Self::MiniScript, true) => &miniscript::STRICT_RULES,
            (Self::Greybel, false) => &greybel::RULES,
            (Self::Greybel, true) => &greybel::STRICT_RULES,
            (Self::GreyScript, false) => &greyscript::RULES,
            (Self::GreyScript, true) => &greyscript::STRICT_RULES,
        }
    }
}
