// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::{DateTime, Utc};
use core::fmt;
use postgres_types::{FromSql, ToSql};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromSql, ToSql)]
#[postgres(transparent)]
pub struct DbRepositoryId(i32);

impl DbRepositoryId {
    /// An i32 representation of the repository ID.
    pub fn bare_i32(self) -> i32 {
        self.0
    }
}

impl fmt::Display for DbRepositoryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[repository {}]", self.0)
    }
}

/// Repository model
#[derive(Debug, Clone)]
pub struct Repository {
    pub id: DbRepositoryId,
    pub name: String,
    pub path: String,
    pub nixfile_path: String,
    pub created_at: DateTime<Utc>,
    pub last_synced_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewRepository {
    pub name: String,
    pub path: String,
    pub nixfile_path: String,
}

impl Repository {
    pub(crate) fn from_row(row: &tokio_postgres::Row) -> Self {
        Self {
            id: row.get("id"),
            name: row.get("name"),
            path: row.get("path"),
            nixfile_path: row.get("nixfile_path"),
            created_at: row.get("created_at"),
            last_synced_at: row.get("last_synced_at"),
        }
    }
}
