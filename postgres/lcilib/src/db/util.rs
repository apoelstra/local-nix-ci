// SPDX-License-Identifier: GPL-3.0-or-later

use tokio_postgres::{Error, Transaction};

/// Determines whether a given table exists in the given schema.
///
/// Takes a transaction rather than a client on the assumption that
/// this information will be needed for subsequent operations.
///
/// # Errors
///
/// Errors if the `SELECT` query fails.
pub async fn table_exists(
    tx: &Transaction<'_>,
    schema: &str,
    table: &str,
) -> Result<bool, Error> {
    let row = tx
        .query_one(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM information_schema.tables
                WHERE table_schema = $1
                  AND table_name = $2
            )
            "#,
            &[&schema, &table],
        )
        .await?;

    Ok(row.get::<_, bool>(0))
}
