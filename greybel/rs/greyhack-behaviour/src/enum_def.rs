//! The `enum_def!` macro: enums with game numeric codes.

/// Defines an enum with game numeric codes. `code` and `i64::from` convert variants to numbers;
/// `from_code` and `TryFrom<i64>` convert them back, and `ALL` lists every variant.
/// `#[serde(into = "i64", try_from = "i64")]` serializes the numeric code.
#[macro_export]
macro_rules! enum_def {
    ($(#[$meta:meta])* $vis:vis enum $name:ident {
        $($(#[$variant_meta:meta])* $variant:ident = $code:literal,)*
    }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        $vis enum $name {
            $($(#[$variant_meta])* $variant = $code,)*
        }

        impl $name {
            /// All variants ordered by numeric code.
            pub const ALL: &[Self] = &[$(Self::$variant),*];

            /// Numeric code stored by the game.
            #[must_use]
            pub const fn code(self) -> i64 {
                self as i64
            }

            /// Looks up the variant with numeric value `code`.
            #[must_use]
            pub const fn from_code(code: i64) -> Option<Self> {
                match code {
                    $($code => Some(Self::$variant),)*
                    _ => None,
                }
            }
        }

        impl From<$name> for i64 {
            fn from(value: $name) -> Self {
                value.code()
            }
        }

        impl TryFrom<i64> for $name {
            type Error = $crate::UnknownCode;

            fn try_from(code: i64) -> Result<Self, Self::Error> {
                Self::from_code(code).ok_or($crate::UnknownCode {
                    code,
                    kind: stringify!($name),
                })
            }
        }
    };
}

/// Numeric value that does not identify a variant of an [`enum_def!`] enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("{code} is no {kind}")]
pub struct UnknownCode {
    pub code: i64,
    /// Name of the enum being converted.
    pub kind: &'static str,
}
