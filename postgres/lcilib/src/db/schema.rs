// SPDX-License-Identifier: GPL-3.0-or-later

use tokio_postgres::Client;
use super::util::table_exists;

const EXPECTED_SCHEMA_VERSION: u32 = 1;

/// Ensure the database contains exactly the schema version this binary expects.
pub async fn ensure_schema(client: &mut Client) -> Result<(), SchemaError> {
    let tx = client.transaction().await?;

    // Treat "the global metadata table exists" as indicating that the database has been initialized.
    let meta_exists = table_exists(&tx, "local_ci", "global").await?;

    if meta_exists {
        let row = tx
            .query_opt("SELECT schema_version FROM local_ci.global LIMIT 1", &[])
            .await?;

        let Some(row) = row else {
            return Err(SchemaError::MissingOrInvalidMetaRow);
        };

        let version = row.get::<_, u32>(0);
        if version != EXPECTED_SCHEMA_VERSION {
            return Err(SchemaError::IncompatibleVersion {
                found: version,
                expected: EXPECTED_SCHEMA_VERSION,
            });
        }

        tx.commit().await?;
        return Ok(());
    }

    tx.batch_execute(include_str!("../../sql/schema_v1.sql")).await?;
    tx.commit().await?;
    Ok(())
}

#[derive(Debug)]
pub enum SchemaError {
    Db(tokio_postgres::Error),
    IncompatibleVersion { found: u32, expected: u32 },
    MissingOrInvalidMetaRow,
}

impl std::fmt::Display for SchemaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaError::Db(e) => write!(f, "database error: {e}"),
            SchemaError::IncompatibleVersion { found, expected } => {
                write!(
                    f,
                    "database schema version {found} is incompatible with expected version {expected}"
                )
            }
            SchemaError::MissingOrInvalidMetaRow => {
                write!(f, "app_meta table is missing its schema version row or it is invalid")
            }
        }
    }
}

impl std::error::Error for SchemaError {}

impl From<tokio_postgres::Error> for SchemaError {
    fn from(e: tokio_postgres::Error) -> Self {
        SchemaError::Db(e)
    }
}
