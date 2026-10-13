mod registry;

use std::cmp::Ordering;

pub use registry::VariationRegistry;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged, expecting = "a string or a number")]
pub enum Variation {
    Text(String),
    Number(f64),
}

impl Variation {
    const GROUP_PREFIX: char = '!';

    fn group(&self) -> Option<&str> {
        match self {
            Self::Text(text) => text.strip_prefix(Self::GROUP_PREFIX),
            Self::Number(_) => None,
        }
    }
}

impl PartialEq for Variation {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Text(left), Self::Text(right)) => left == right,
            (Self::Number(left), Self::Number(right)) => {
                left == right || (left.is_nan() && right.is_nan())
            }
            _ => false,
        }
    }
}

impl Eq for Variation {}

/// Variation ordering: ascending numbers first with NaN last, then case-insensitive alphabetical text; text
/// differing only by case uses byte order.
impl Ord for Variation {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Number(left), Self::Number(right)) => match (left.is_nan(), right.is_nan()) {
                (true, true) => Ordering::Equal,
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                (false, false) => left.partial_cmp(right).unwrap_or(Ordering::Equal),
            },
            (Self::Number(_), Self::Text(_)) => Ordering::Less,
            (Self::Text(_), Self::Number(_)) => Ordering::Greater,
            (Self::Text(left), Self::Text(right)) => left
                .to_lowercase()
                .cmp(&right.to_lowercase())
                .then_with(|| left.cmp(right)),
        }
    }
}

impl PartialOrd for Variation {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl From<&str> for Variation {
    fn from(text: &str) -> Self {
        Self::Text(text.to_owned())
    }
}

impl From<String> for Variation {
    fn from(text: String) -> Self {
        Self::Text(text)
    }
}

impl From<f64> for Variation {
    fn from(number: f64) -> Self {
        Self::Number(number)
    }
}
