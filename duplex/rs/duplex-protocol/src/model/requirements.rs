/// Inclusive byte, item, or entry count bounds.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Length {
    pub min: u32,
    /// Inclusive maximum, or `None` for no upper bound.
    pub max: Option<u32>,
}

impl Length {
    /// Minimum zero with no upper bound.
    pub const ANY: Self = Self { min: 0, max: None };

    #[must_use]
    pub fn is_any(self) -> bool {
        self == Self::ANY
    }

    #[must_use]
    pub fn contains(self, length: u64) -> bool {
        length >= u64::from(self.min) && self.max.is_none_or(|max| length <= u64::from(max))
    }
}

impl std::fmt::Display for Length {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.min, self.max) {
            (0, Some(max)) => write!(formatter, "..={max}"),
            (min, Some(max)) => write!(formatter, "{min}..={max}"),
            (min, None) => write!(formatter, "{min}.."),
        }
    }
}

/// Inclusive numeric bounds as source literals.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Range {
    pub min: Option<String>,
    pub max: Option<String>,
}

impl Range {
    #[must_use]
    pub fn is_any(&self) -> bool {
        self.min.is_none() && self.max.is_none()
    }
}

/// String UTF-8 byte-length and full-string regular-expression requirements.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct StringRequirementContainer {
    pub length: Length,
    /// Regular expression required to match the whole string.
    pub pattern: Option<String>,
}

impl StringRequirementContainer {
    #[must_use]
    pub fn is_any(&self) -> bool {
        self.length.is_any() && self.pattern.is_none()
    }
}
