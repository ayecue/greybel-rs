//! TMP reference captures for every test case (`crate::cases`) and crate output differences
//! from it.

use std::fmt::{self, Display};
use std::path::Path;

use serde::Deserialize;
use serde::de::DeserializeOwned;

/// Width, in pixels, the reference project lays the cases out in.
pub const WIDTH: u32 = 1000;

/// Contains TMP output for all cases at layout width `width`.
#[derive(Deserialize)]
pub struct Captures<C> {
    pub width: f32,
    pub cases: Vec<C>,
}

impl<C: DeserializeOwned> Captures<C> {
    /// Captures written at `path`.
    ///
    /// # Panics
    /// When the file isn't the captures.
    #[must_use]
    pub fn read(path: &Path) -> Self {
        let json = std::fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));

        serde_json::from_str(&json).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
    }
}

impl<C> Captures<C> {
    /// Case-specific output differences: `compare` receives the recorded differences, case, and
    /// the width.
    pub fn differences(
        &self,
        mut compare: impl FnMut(&mut Differences, &C, f32),
    ) -> Vec<Difference> {
        self.cases
            .iter()
            .enumerate()
            .flat_map(|(index, case)| {
                let mut differences = Differences::new(index);

                compare(&mut differences, case, self.width);

                differences.found
            })
            .collect()
    }
}

/// Field of a case where the crate knowingly differs from TMP, and why.
pub struct KnownDifference {
    pub case: usize,
    pub field: &'static str,
    pub reason: &'static str,
}

impl KnownDifference {
    #[must_use]
    pub fn matches(&self, difference: &Difference) -> bool {
        self.case == difference.case && self.field == difference.field
    }
}

/// Field of a case the crate gives otherwise than TMP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Difference {
    pub case: usize,
    pub field: String,
    pub expected: String,
    pub actual: String,
}

impl Display for Difference {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "case {} {}: TMP {} vs {}",
            self.case, self.field, self.expected, self.actual
        )
    }
}

/// Differences of one case.
pub struct Differences {
    case: usize,
    found: Vec<Difference>,
}

impl Differences {
    #[must_use]
    pub const fn new(case: usize) -> Self {
        Self {
            case,
            found: Vec::new(),
        }
    }

    pub fn count(&mut self, field: &str, expected: usize, actual: usize) {
        if expected != actual {
            self.push(field, expected, actual);
        }
    }

    pub fn flag(&mut self, field: &str, expected: bool, actual: bool) {
        if expected != actual {
            self.push(field, expected, actual);
        }
    }

    /// Number further than `tolerance` from TMP's, or not a number.
    pub fn float(&mut self, field: &str, expected: f32, actual: f32, tolerance: f32) {
        if (expected - actual).abs() > tolerance || actual.is_nan() {
            self.push(field, expected, actual);
        }
    }

    pub fn push(&mut self, field: &str, expected: impl Display, actual: impl Display) {
        self.found.push(Difference {
            case: self.case,
            field: field.to_owned(),
            expected: expected.to_string(),
            actual: actual.to_string(),
        });
    }
}

/// Checks that crate-versus-TMP differences match `known` exactly.
///
/// # Panics
/// Panics on an undocumented difference or when a documented difference no longer exists.
pub fn assert_known(differences: &[Difference], known: &[KnownDifference]) {
    let unexpected: Vec<String> = differences
        .iter()
        .filter(|difference| !known.iter().any(|known| known.matches(difference)))
        .map(ToString::to_string)
        .collect();
    let resolved: Vec<String> = known
        .iter()
        .filter(|known| {
            !differences
                .iter()
                .any(|difference| known.matches(difference))
        })
        .map(|known| format!("case {} {}: {}", known.case, known.field, known.reason))
        .collect();

    assert!(
        unexpected.is_empty(),
        "{} differences:\n{}",
        unexpected.len(),
        unexpected.join("\n")
    );
    assert!(
        resolved.is_empty(),
        "listed differences no longer differ:\n{}",
        resolved.join("\n")
    );
}
