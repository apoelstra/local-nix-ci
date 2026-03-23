// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::{DateTime, Utc};
use core::fmt;
use postgres_types::{FromSql, ToSql};

use super::{PullRequest, Stack};
use crate::db::{DbQueryError, EntityType};

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

impl DbRepositoryId {
    /// Returns the list of current (not pushed) pull requests associated with
    /// this repository.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn get_current_pull_requests(
        self,
        tx: &tokio_postgres::Transaction<'_>,
    ) -> Result<Vec<PullRequest>, DbQueryError> {
        let rows = tx
            .query(
                r#"
                SELECT id, repository_id, pr_number, title, body, author_login, target_branch, tip_commit_id, merge_status, review_status,
                       priority, ok_to_merge, required_reviewers, created_at, updated_at, synced_at
                FROM pull_requests
                WHERE repository_id = $1 AND merge_status != 'pushed' AND merge_status != 'conflicted'
                ORDER BY pr_number DESC
                "#,
                &[&self],
            )
            .await
            .map_err(|error| {
                DbQueryError {
                    action: "get_current_pull_requests",
                    entity_type: EntityType::Repository,
                    raw_id: Some(self.bare_i32()),
                    clauses: vec![],
                    error,
                }
            })?;
        Ok(rows.iter().map(PullRequest::from_row).collect())
    }

    /// Returns the list of merge stacks associated with
    /// this repository.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn get_stacks(
        self,
        tx: &tokio_postgres::Transaction<'_>,
    ) -> Result<Vec<Stack>, DbQueryError> {
        let rows = tx
            .query(
                r#"
                SELECT id, repository_id, target_branch, created_at, updated_at
                FROM stacks
                WHERE repository_id = $1
                ORDER BY created_at ASC
                "#,
                &[&self],
            )
            .await
            .map_err(|error| DbQueryError {
                action: "get_stacks",
                entity_type: EntityType::Repository,
                raw_id: Some(self.bare_i32()),
                clauses: vec![],
                error,
            })?;
        Ok(rows.iter().map(Stack::from_row).collect())
    }
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

    /// Find repository by ID
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    ///
    /// # Panics
    ///
    /// Panics if no repository with the given ID is in the database.
    pub async fn get_by_id(
        tx: &tokio_postgres::Transaction<'_>,
        id: DbRepositoryId,
    ) -> Result<Self, DbQueryError> {
        match Self::find_by_id(tx, id).await {
            Ok(Some(x)) => Ok(x),
            Ok(None) => panic!("no repository with id {id} in database"),
            Err(e) => Err(e),
        }
    }

    /// Find repository by ID
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn find_by_id(
        tx: &tokio_postgres::Transaction<'_>,
        id: DbRepositoryId,
    ) -> Result<Option<Self>, DbQueryError> {
        let rows = tx
            .query("SELECT id, name, path, nixfile_path, created_at, last_synced_at FROM repositories WHERE id = $1", &[&id])
            .await
            .map_err(|error| {
                DbQueryError {
                    action: "find_by_id",
                    entity_type: EntityType::Repository,
                    raw_id: Some(id.bare_i32()),
                    clauses: vec![],
                    error,
                }
            })?;

        Ok(rows.first().map(Self::from_row))
    }
}
