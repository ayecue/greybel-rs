use std::collections::HashMap;

use turso::{Row, Statement};

use super::{Code, FromSql};

/// Maps result column names to positions.
pub(crate) struct ColumnIndex(HashMap<String, usize>);

impl ColumnIndex {
    pub(crate) fn of(statement: &Statement) -> Self {
        Self(
            statement
                .columns()
                .iter()
                .enumerate()
                .map(|(index, column)| (column.name().to_owned(), index))
                .collect(),
        )
    }
}

/// Database row with its column-name index.
pub struct RowView<'a> {
    pub(crate) row: &'a Row,
    pub(crate) columns: &'a ColumnIndex,
}

impl RowView<'_> {
    /// Converts column `name` to `T`.
    ///
    /// # Errors
    ///
    /// Returns a conversion error if the value is not a `T`.
    ///
    /// # Panics
    ///
    /// Panics if the query has no column `name`.
    pub fn get<T: FromSql>(&self, name: &str) -> turso::Result<T> {
        let index = *self
            .columns
            .0
            .get(name)
            .unwrap_or_else(|| panic!("column `{name}` is not in the query"));

        T::from_sql(self.row.get_value(index)?)
    }

    /// Converts the integer in column `name` to `T` with [`TryFrom<i64>`], as [`Code`] does.
    ///
    /// # Errors
    ///
    /// Returns a conversion error if the value is not an integer `T` converts from.
    ///
    /// # Panics
    ///
    /// Panics if the query has no column `name`.
    pub fn get_code<T>(&self, name: &str) -> turso::Result<T>
    where
        T: TryFrom<i64>,
        T::Error: std::fmt::Display,
    {
        self.get::<Code<T>>(name).map(|Code(value)| value)
    }

    /// Like [`RowView::get_code`], with `None` for a null column.
    ///
    /// # Errors
    ///
    /// Returns a conversion error if the value is neither null nor an integer `T` converts from.
    ///
    /// # Panics
    ///
    /// Panics if the query has no column `name`.
    pub fn get_code_opt<T>(&self, name: &str) -> turso::Result<Option<T>>
    where
        T: TryFrom<i64>,
        T::Error: std::fmt::Display,
    {
        Ok(self.get::<Option<Code<T>>>(name)?.map(|Code(value)| value))
    }

    /// Returns the first column of a single-column query.
    ///
    /// # Errors
    ///
    /// Returns a conversion error if the value is not a `T`.
    pub fn first<T: FromSql>(&self) -> turso::Result<T> {
        self.nth(0)
    }

    /// Returns column `index`.
    ///
    /// # Errors
    ///
    /// Returns a conversion error if the value is not a `T`.
    pub fn nth<T: FromSql>(&self, index: usize) -> turso::Result<T> {
        T::from_sql(self.row.get_value(index)?)
    }
}

/// Builds a value from one database row by column name.
pub trait FromRow: Sized {
    /// Reads the value from `row`.
    ///
    /// # Errors
    ///
    /// Returns a conversion error if a column is not the expected type.
    fn from_row(row: &RowView<'_>) -> turso::Result<Self>;
}

/// Single value from a one-column query, such as an ID, name, or address.
pub struct Scalar<T>(pub T);

impl<T: FromSql> FromRow for Scalar<T> {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self(row.first()?))
    }
}

impl<A: FromSql, B: FromSql> FromRow for (A, B) {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok((row.nth(0)?, row.nth(1)?))
    }
}

impl<A: FromSql, B: FromSql, C: FromSql> FromRow for (A, B, C) {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok((row.nth(0)?, row.nth(1)?, row.nth(2)?))
    }
}
