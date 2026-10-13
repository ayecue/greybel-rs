use turso::Value;

/// Converts a SQLite [`Value`] to a Rust type.
pub(crate) trait FromSql: Sized {
    fn from_sql(value: Value) -> turso::Result<Self>;
}

fn conversion(value: &Value, target: &str) -> turso::Error {
    turso::Error::ConversionFailure(format!("cannot read {value:?} as {target}"))
}

impl FromSql for i64 {
    fn from_sql(value: Value) -> turso::Result<Self> {
        match value {
            Value::Integer(i) => Ok(i),
            other => Err(conversion(&other, "integer")),
        }
    }
}

macro_rules! from_sql_integer {
    ($($ty:ty),*) => {$(
        impl FromSql for $ty {
            fn from_sql(value: Value) -> turso::Result<Self> {
                let integer = i64::from_sql(value)?;
                Self::try_from(integer)
                    .map_err(|_| conversion(&Value::Integer(integer), stringify!($ty)))
            }
        }
    )*};
}

from_sql_integer!(u8, u16, u32, u64);

impl FromSql for f64 {
    fn from_sql(value: Value) -> turso::Result<Self> {
        match value {
            Value::Real(f) => Ok(f),
            Value::Integer(i) => Ok(i as Self),
            other => Err(conversion(&other, "real")),
        }
    }
}

impl FromSql for bool {
    fn from_sql(value: Value) -> turso::Result<Self> {
        match value {
            Value::Integer(i) => Ok(i != 0),
            other => Err(conversion(&other, "boolean")),
        }
    }
}

impl FromSql for Vec<u8> {
    fn from_sql(value: Value) -> turso::Result<Self> {
        match value {
            Value::Blob(bytes) => Ok(bytes),
            other => Err(conversion(&other, "blob")),
        }
    }
}

impl FromSql for String {
    fn from_sql(value: Value) -> turso::Result<Self> {
        match value {
            Value::Text(text) => Ok(text),
            other => Err(conversion(&other, "text")),
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
