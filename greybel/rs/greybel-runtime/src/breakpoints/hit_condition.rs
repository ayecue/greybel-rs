use std::num::ParseIntError;

/// Hit-condition validation error.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(super) enum HitConditionError {
    #[error(
        "`{text}` is not a hit count: write a number, optionally after =, ==, >=, >, <, <=, or %."
    )]
    Count {
        text: String,
        #[source]
        source: ParseIntError,
    },
    #[error("`% 0` never stops: write % and a number above 0.")]
    ZeroEvery,
}

/// Predicate evaluated against a breakpoint hit count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HitCondition {
    Equal(u32),
    AtLeast(u32),
    Above(u32),
    Below(u32),
    AtMost(u32),
    Every(u32),
}

impl HitCondition {
    /// Parses an optional comparator and unsigned count. A bare number selects `AtLeast`; malformed counts and `% 0`
    /// return descriptive errors.
    pub(super) fn parse(text: &str) -> Result<Self, HitConditionError> {
        type Make = fn(u32) -> HitCondition;

        let text = text.trim();

        let operators: [(&str, Make); 7] = [
            ("==", Self::Equal),
            (">=", Self::AtLeast),
            ("<=", Self::AtMost),
            ("=", Self::Equal),
            (">", Self::Above),
            ("<", Self::Below),
            ("%", Self::Every),
        ];
        let (make, count) = operators
            .iter()
            .find_map(|(operator, make)| text.strip_prefix(operator).map(|count| (*make, count)))
            .unwrap_or((Self::AtLeast, text));
        let count = count
            .trim()
            .parse::<u32>()
            .map_err(|source| HitConditionError::Count {
                text: text.to_owned(),
                source,
            })?;

        match make(count) {
            Self::Every(0) => Err(HitConditionError::ZeroEvery),
            condition => Ok(condition),
        }
    }

    /// Checks `hits` against this predicate.
    pub(super) fn is_met(self, hits: u32) -> bool {
        match self {
            Self::Equal(count) => hits == count,
            Self::AtLeast(count) => hits >= count,
            Self::Above(count) => hits > count,
            Self::Below(count) => hits < count,
            Self::AtMost(count) => hits <= count,
            Self::Every(count) => hits.is_multiple_of(count),
        }
    }
}

#[cfg(test)]
#[path = "tests/hit_condition.rs"]
mod tests;
