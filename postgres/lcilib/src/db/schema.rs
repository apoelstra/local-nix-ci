// SPDX-License-Identifier: GPL-3.0-or-later

use super::util::table_exists;
use tokio_postgres::Client;

// WARNING: you must decode integers from Postgres as i32s. If you try to use u32, it will
// fail (expecting an OID, will give an opaque "failed to decode" error).
// See https://docs.rs/tokio-postgres/latest/tokio_postgres/types/trait.FromSql.html
const EXPECTED_SCHEMA_VERSION: i32 = 1;

/// Ensure the database contains exactly the schema version this binary expects.
pub(super) async fn ensure_schema(client: &mut Client) -> Result<(), SchemaError> {
    let tx = client.transaction().await.map_err(|e| SchemaError::Db {
        e,
        action: "create transaction",
    })?;

    // Treat "the global metadata table exists" as indicating that the database has been initialized.
    let meta_exists = table_exists(&tx, "global")
        .await
        .map_err(|e| SchemaError::Db {
            e,
            action: "check for 'global' table",
        })?;

    if meta_exists {
        let row = tx
            .query_opt("SELECT schema_version FROM global LIMIT 1", &[])
            .await
            .map_err(|e| SchemaError::Db {
                e,
                action: "select schema_version from 'global' table",
            })?;

        let Some(row) = row else {
            return Err(SchemaError::MissingOrInvalidMetaRow);
        };

        let version = row.get::<_, i32>(0);
        if version != EXPECTED_SCHEMA_VERSION {
            return Err(SchemaError::IncompatibleVersion {
                found: version,
                expected: EXPECTED_SCHEMA_VERSION,
            });
        }

        tx.commit().await.map_err(|e| SchemaError::Db {
            e,
            action: "commit transaction (select version)",
        })?;
        return Ok(());
    }

    tx.batch_execute(include_str!("../../sql/schema_v1.sql"))
        .await
        .map_err(|e| SchemaError::Db {
            e,
            action: "execute schema_v1.sql",
        })?;
    tx.commit().await.map_err(|e| SchemaError::Db {
        e,
        action: "commit transaction (execute schema_v1.sql)",
    })?;
    Ok(())
}

#[derive(Debug)]
pub enum SchemaError {
    Db {
        action: &'static str,
        e: tokio_postgres::Error,
    },
    IncompatibleVersion {
        found: i32,
        expected: i32,
    },
    MissingOrInvalidMetaRow,
}

impl std::fmt::Display for SchemaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Db { action, .. } => write!(f, "database error ({action})"),
            Self::IncompatibleVersion { found, expected } => {
                write!(
                    f,
                    "database schema version {found} is incompatible with expected version {expected}"
                )
            }
            Self::MissingOrInvalidMetaRow => {
                write!(
                    f,
                    "app_meta table is missing its schema version row or it is invalid"
                )
            }
        }
    }
}

impl std::error::Error for SchemaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            Self::Db { ref e, .. } => Some(e),
            Self::IncompatibleVersion { .. } => None,
            Self::MissingOrInvalidMetaRow => None,
        }
    }
}
