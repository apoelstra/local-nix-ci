// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::{DateTime, Utc};
use core::fmt;
use postgres_types::{FromSql, ToSql};

use super::DbRepositoryId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromSql, ToSql)]
#[postgres(transparent)]
pub struct DbAllowedApproverId(i32);

impl DbAllowedApproverId {
    /// An i32 representation of the allowed approver ID.
    pub fn bare_i32(self) -> i32 {
        self.0
    }
}

impl fmt::Display for DbAllowedApproverId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[allowed_approver {}]", self.0)
    }
}

/// Allowed approver model
#[derive(Debug, Clone)]
pub struct AllowedApprover {
    pub id: DbAllowedApproverId,
    pub repository_id: DbRepositoryId,
    pub approver_name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewAllowedApprover {
    pub repository_id: DbRepositoryId,
    pub approver_name: String,
}

impl AllowedApprover {
    pub(crate) fn from_row(row: &tokio_postgres::Row) -> Self {
        Self {
            id: row.get("id"),
            repository_id: row.get("repository_id"),
            approver_name: row.get("approver_name"),
            created_at: row.get("created_at"),
        }
    }
}
