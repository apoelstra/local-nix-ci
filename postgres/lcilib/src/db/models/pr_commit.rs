// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::{DateTime, Utc};
use core::fmt;
use postgres_types::{FromSql, ToSql};

use crate::db::{DbQueryError, EntityType, Transaction};
use super::{CommitType, DbCommitId, DbPullRequestId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromSql, ToSql)]
#[postgres(transparent)]
pub struct DbPrCommitId(i32);

impl DbPrCommitId {
    /// An i32 representation of the pr commit ID.
    pub fn bare_i32(self) -> i32 {
        self.0
    }
}

impl fmt::Display for DbPrCommitId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[pr_commit {}]", self.0)
    }
}

/// PR-Commit relationship model
#[derive(Debug, Clone)]
pub struct PrCommit {
    pub id: DbPrCommitId,
    pub pull_request_id: DbPullRequestId,
    pub commit_id: DbCommitId,
    pub sequence_order: i32,
    pub commit_type: CommitType,
    pub is_current: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PrCommit {
    pub(crate) fn from_row(row: &tokio_postgres::Row) -> Self {
        Self {
            id: row.get("id"),
            pull_request_id: row.get("pull_request_id"),
            commit_id: row.get("commit_id"),
            sequence_order: row.get("sequence_order"),
            commit_type: row.get("commit_type"),
            is_current: row.get("is_current"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }

    /// Find all PR-commit relationships for a specific commit
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn find_by_commit(
        tx: &Transaction<'_>,
        commit_id: DbCommitId,
    ) -> Result<Vec<Self>, DbQueryError> {
        let rows = tx
            .inner
            .query(
                r#"
                SELECT id, pull_request_id, commit_id, sequence_order, commit_type, is_current, created_at, updated_at
                FROM pr_commits
                WHERE commit_id = $1
                ORDER BY updated_at DESC
                "#,
                &[&commit_id],
            )
            .await
            .map_err(|error| DbQueryError {
                action: "find_by_commit",
                entity_type: EntityType::Commit,
                raw_id: Some(commit_id.bare_i32()),
                clauses: vec![format!("commit_id = {commit_id}")],
                error,

            })?;

        Ok(rows.iter().map(Self::from_row).collect())
    }
}
