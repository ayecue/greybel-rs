use turso::Value;

/// Converts a SQLite [`Value`] to a Rust type.
pub trait FromSql: Sized {
    /// Converts `value`.
    ///
    /// # Errors
    ///
    /// Returns a conversion error if `value` is not a valid `Self`.
    fn from_sql(value: Value) -> turso::Result<Self>;
}

fn conversion_failure(value: &Value, target: &str) -> turso::Error {
    turso::Error::ConversionFailure(format!("cannot read {value:?} as {target}"))
}

impl FromSql for i64 {
    fn from_sql(value: Value) -> turso::Result<Self> {
        match value {
            Value::Integer(i) => Ok(i),
            other => Err(conversion_failure(&other, "integer")),
        }
    }
}

macro_rules! from_sql_integer {
    ($($ty:ty),*) => {$(
        impl FromSql for $ty {
            fn from_sql(value: Value) -> turso::Result<Self> {
                let integer = i64::from_sql(value)?;
                Self::try_from(integer)
                    .map_err(|_| conversion_failure(&Value::Integer(integer), stringify!($ty)))
            }
        }
    )*};
}

from_sql_integer!(u8, u16, u32, u64);

impl FromSql for f64 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "SQLite INTEGER values convert to the nearest f64 when read as REAL"
    )]
    fn from_sql(value: Value) -> turso::Result<Self> {
        match value {
            Value::Real(f) => Ok(f),
            Value::Integer(i) => Ok(i as Self),
            other => Err(conversion_failure(&other, "real")),
        }
    }
}

impl FromSql for bool {
    fn from_sql(value: Value) -> turso::Result<Self> {
        match value {
            Value::Integer(i) => Ok(i != 0),
            other => Err(conversion_failure(&other, "boolean")),
        }
    }
}

impl FromSql for Vec<u8> {
    fn from_sql(value: Value) -> turso::Result<Self> {
        match value {
            Value::Blob(bytes) => Ok(bytes),
            other => Err(conversion_failure(&other, "blob")),
        }
    }
}

impl FromSql for String {
    fn from_sql(value: Value) -> turso::Result<Self> {
        match value {
            Value::Text(text) => Ok(text),
            other => Err(conversion_failure(&other, "text")),
        }
    }
}

impl<T: FromSql> FromSql for Option<T> {
    fn from_sql(value: Value) -> turso::Result<Self> {
        match value {
            Value::Null => Ok(None),
            other => T::from_sql(other).map(Some),
        }
    }
}

impl FromSql for f32 {
    fn from_sql(value: Value) -> turso::Result<Self> {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "columns stored from f32 values"
        )]
        f64::from_sql(value).map(|number| number as Self)
    }
}

/// A value the database stores as the integer its type converts from with [`TryFrom<i64>`], such as
/// an enum's code.
///
/// A type from another crate cannot implement [`FromSql`] itself, but it can be read as
/// `Code<T>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Code<T>(pub T);

impl<T> FromSql for Code<T>
where
    T: TryFrom<i64>,
    T::Error: std::fmt::Display,
{
    fn from_sql(value: Value) -> turso::Result<Self> {
        let code = i64::from_sql(value)?;

        T::try_from(code)
            .map(Self)
            .map_err(|error| turso::Error::ConversionFailure(error.to_string()))
    }
}
