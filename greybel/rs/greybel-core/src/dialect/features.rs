use std::ops::BitOr;

/// A set of Greybel syntax features: syntax Grey Hack doesn't have, which builds write as plain GreyScript. Strict
/// mode has none.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct SyntaxFeatures(u32);

impl SyntaxFeatures {
    pub const ALL: Self = Self((1 << 17) - 1);
    /// `assert condition, message`.
    pub const ASSERT: Self = Self(1 << 6);
    /// `class Name extends Base ... end class`.
    pub const CLASSES: Self = Self(1 << 15);
    /// `a ?? b`.
    pub const COALESCE: Self = Self(1 << 13);
    /// `let` and `const` before an assignment.
    pub const DECLARATIONS: Self = Self(1 << 2);
    /// `[a, b] = list`, `{name, port as p} = map`, `for [x, y] in list`, `for {key, value} in map` and
    /// `#import {a, b as c} from "path"`.
    pub const DESTRUCTURING_ASSIGNMENTS: Self = Self(1 << 8);
    /// `enum Name ... end enum`.
    pub const ENUM: Self = Self(1 << 3);
    /// `export name = value`.
    pub const EXPORT: Self = Self(1 << 11);
    /// `for i, item in list`.
    pub const FOR_INDEX: Self = Self(1 << 9);
    /// `{name}` for `{"name": name}`.
    pub const MAP_SHORTHAND: Self = Self(1 << 5);
    /// Every feature with its name in `greybel.json` (`"syntax": {"ternary": false}`).
    pub const NAMES: [(&'static str, Self); 17] = [
        ("number-literals", Self::NUMBER_LITERALS),
        ("templates", Self::TEMPLATES),
        ("declarations", Self::DECLARATIONS),
        ("enum", Self::ENUM),
        ("spread", Self::SPREAD),
        ("map-shorthand", Self::MAP_SHORTHAND),
        ("assert", Self::ASSERT),
        ("short-functions", Self::SHORT_FUNCTIONS),
        ("destructuring-assignments", Self::DESTRUCTURING_ASSIGNMENTS),
        ("for-index", Self::FOR_INDEX),
        ("switch", Self::SWITCH),
        ("export", Self::EXPORT),
        ("ternary", Self::TERNARY),
        ("coalesce", Self::COALESCE),
        ("optional-chains", Self::OPTIONAL_CHAINS),
        ("classes", Self::CLASSES),
        ("types", Self::TYPES),
    ];
    pub const NONE: Self = Self(0);
    /// `0xFF`, `0o17`, `0b1010` and `_` between digits (`1_000_000`).
    pub const NUMBER_LITERALS: Self = Self(1);
    /// `a?.b`, `a?.[i]` and `f?.(x)`.
    pub const OPTIONAL_CHAINS: Self = Self(1 << 14);
    /// `(x) => x * 2`.
    pub const SHORT_FUNCTIONS: Self = Self(1 << 7);
    /// `...list` in list literals and `...map` in map literals.
    pub const SPREAD: Self = Self(1 << 4);
    /// `switch value ... case ... end switch`.
    pub const SWITCH: Self = Self(1 << 10);
    /// `#"text {expression}"`.
    pub const TEMPLATES: Self = Self(1 << 1);
    /// `condition ? a : b`.
    pub const TERNARY: Self = Self(1 << 12);
    /// Types in code: `name: T` parameters and `let` variables, `): T` returns, `function<T>`, `expr as T`, `x!`
    /// and `type` statements.
    pub const TYPES: Self = Self(1 << 16);

    #[must_use]
    pub const fn contains(self, features: Self) -> bool {
        self.0 & features.0 == features.0
    }

    /// The feature named `name` in `greybel.json`.
    #[must_use]
    pub fn by_name(name: &str) -> Option<Self> {
        Self::NAMES
            .iter()
            .find(|(feature, _)| *feature == name)
            .map(|&(_, feature)| feature)
    }

    /// Every feature but those named in `names`; names that aren't features are skipped.
    #[must_use]
    pub fn except<'a>(names: impl IntoIterator<Item = &'a str>) -> Self {
        names
            .into_iter()
            .filter_map(Self::by_name)
            .fold(Self::ALL, Self::without)
    }

    #[must_use]
    pub const fn without(self, features: Self) -> Self {
        Self(self.0 & !features.0)
    }

    #[must_use]
    pub const fn intersection(self, features: Self) -> Self {
        Self(self.0 & features.0)
    }
}

impl BitOr for SyntaxFeatures {
    type Output = Self;

    fn bitor(self, features: Self) -> Self {
        Self(self.0 | features.0)
    }
}
