//! Sources for the tests of types where variables are read.

/// Gives `x` the fact `number` while its declared type is `number or string`.
pub const NUMBER_FACT: &str = "x = 1\nif rnd then x = \"s\"\nx = 1\n";
