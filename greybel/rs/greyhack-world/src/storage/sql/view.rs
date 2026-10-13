use std::collections::HashMap;

use turso::{Row, Statement};

use super::FromSql;

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
pub(crate) struct RowView<'a> {
    pub(crate) row: &'a Row,
    pub(crate) columns: &'a ColumnIndex,
}

impl RowView<'_> {
    /// Converts column `name` to `T`.
    pub(crate) fn get<T: FromSql>(&self, name: &str) -> turso::Result<T> {
        let index = *self
            .columns
            .0
            .get(name)
            .unwrap_or_else(|| panic!("column `{name}` is not in the query"));

        T::from_sql(self.row.get_value(index)?)
    }

    /// Returns the first column of a single-column query.
    pub(crate) fn first<T: FromSql>(&self) -> turso::Result<T> {
        self.nth(0)
    }

    /// Returns column `index`.
    pub(crate) fn nth<T: FromSql>(&self, index: usize) -> turso::Result<T> {
        T::from_sql(self.row.get_value(index)?)
    }
}

/// Builds a value from one database row by column name.
pub(crate) trait FromRow: Sized {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self>;
}

/// Single value from a one-column query, such as an ID, name, or address.
pub(crate) struct Scalar<T>(pub(crate) T);

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
