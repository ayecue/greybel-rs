//! Cached statement execution and row mapping by column name.

mod value;
mod view;

use turso::{Connection, IntoParams};
pub(crate) use value::FromSql;
pub(crate) use view::{ColumnIndex, FromRow, RowView, Scalar};

/// Runs cached `sql` without recompiling repeated statements and returns the changed-row count.
pub(crate) async fn execute(
    conn: &Connection,
    sql: &str,
    params: impl IntoParams,
) -> turso::Result<u64> {
    conn.prepare_cached(sql).await?.execute(params).await
}

/// Runs `INSERT` statement `sql` and returns the inserted rowid.
pub(crate) async fn insert(
    conn: &Connection,
    sql: &str,
    params: impl IntoParams,
) -> turso::Result<i64> {
    execute(conn, sql, params).await?;

    Ok(conn.last_insert_rowid())
}

/// Runs `sql` and maps all rows through [`FromRow`].
pub(crate) async fn fetch_all<T: FromRow>(
    conn: &Connection,
    sql: &str,
    params: impl IntoParams,
) -> turso::Result<Vec<T>> {
    let mut statement = conn.prepare_cached(sql).await?;
    let columns = ColumnIndex::of(&statement);
    let mut rows = statement.query(params).await?;
    let mut out = Vec::new();

    while let Some(row) = rows.next().await? {
        out.push(T::from_row(&RowView {
            row: &row,
            columns: &columns,
        })?);
    }

    Ok(out)
}

/// Runs `sql` and maps the first row, if any, through [`FromRow`].
pub(crate) async fn fetch_opt<T: FromRow>(
    conn: &Connection,
    sql: &str,
    params: impl IntoParams,
) -> turso::Result<Option<T>> {
    let mut statement = conn.prepare_cached(sql).await?;
    let columns = ColumnIndex::of(&statement);
    let mut rows = statement.query(params).await?;

    match rows.next().await? {
        Some(row) => Ok(Some(T::from_row(&RowView {
            row: &row,
            columns: &columns,
        })?)),
        None => Ok(None),
    }
}

/// Collects the first column from every query row.
pub(crate) async fn fetch_column<T: FromSql>(
    conn: &Connection,
    sql: &str,
    params: impl IntoParams,
) -> turso::Result<Vec<T>> {
    let rows = fetch_all::<Scalar<T>>(conn, sql, params).await?;

    Ok(rows.into_iter().map(|Scalar(value)| value).collect())
}

/// Returns the first column of the first query row, if any.
pub(crate) async fn fetch_value<T: FromSql>(
    conn: &Connection,
    sql: &str,
    params: impl IntoParams,
) -> turso::Result<Option<T>> {
    let row = fetch_opt::<Scalar<T>>(conn, sql, params).await?;

    Ok(row.map(|Scalar(value)| value))
}
