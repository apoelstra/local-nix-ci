// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::{DateTime, Utc};
use core::fmt;
use postgres_types::{FromSql, ToSql};

use super::{DbCommitId, DbRepositoryId};
use crate::db::{DbQueryError, EntityType, util::log_action};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromSql, ToSql)]
#[postgres(transparent)]
pub struct DbStackId(i32);

impl DbStackId {
    /// An i32 representation of the stack ID.
    pub fn bare_i32(self) -> i32 {
        self.0
    }
}

impl fmt::Display for DbStackId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[stack {}]", self.0)
    }
}

/// Stack model
#[derive(Debug, Clone)]
pub struct Stack {
    pub id: DbStackId,
    pub repository_id: DbRepositoryId,
    pub target_branch: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Stack-Commit relationship model
#[derive(Debug, Clone)]
pub struct StackCommit {
    pub id: i32,
    pub stack_id: DbStackId,
    pub commit_id: DbCommitId,
    pub sequence_order: i32,
}

#[derive(Debug, Clone)]
pub struct NewStack {
    pub repository_id: DbRepositoryId,
    pub target_branch: String,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateStack {
    pub target_branch: Option<String>,
}

impl UpdateStack {
    fn to_params_and_clauses(&self) -> (Vec<&(dyn ToSql + Sync)>, Vec<String>) {
        let mut set_clauses = Vec::new();
        let mut params = Vec::<&(dyn ToSql + Sync)>::new();
        let param_count = 1;

        if let Some(target_branch) = &self.target_branch {
            set_clauses.push(format!("target_branch = ${}", param_count));
            params.push(target_branch);
        }

        (params, set_clauses)
    }

    fn to_log_string(&self) -> String {
        use core::fmt::Write as _;

        let mut ret = String::new();
        if let Some(target_branch) = &self.target_branch {
            let _ = writeln!(ret, "    set target_branch to {}", target_branch);
        }

        ret
    }
}

impl DbStackId {
    /// Updates a stack by its database ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails (the update or the log).
    pub async fn apply_update(
        self,
        tx: &tokio_postgres::Transaction<'_>,
        updates: &UpdateStack,
    ) -> Result<Option<tokio_postgres::Row>, DbQueryError> {
        let ret = self.apply_update_no_log(tx, updates).await?;
        log_action(
            tx,
            EntityType::Stack,
            self.bare_i32(),
            "stack_updated",
            Some(&updates.to_log_string()),
            None,
        )
        .await?;
        Ok(ret)
    }

    /// Updates a stack by its database ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails (the update or the log).
    pub async fn apply_update_no_log(
        self,
        tx: &tokio_postgres::Transaction<'_>,
        updates: &UpdateStack,
    ) -> Result<Option<tokio_postgres::Row>, DbQueryError> {
        let (mut params, clauses) = updates.to_params_and_clauses();
        if clauses.is_empty() {
            return Ok(None);
        }

        params.push(&self);
        let query = format!(
            r#"
            UPDATE stacks SET {}
            WHERE id = ${}
            RETURNING id, repository_id, target_branch, created_at, updated_at
            "#,
            clauses.join(", "),
            clauses.len() + 1,
        );

        tx.query_one(&query, &params)
            .await
            .map(Some)
            .map_err(|error| DbQueryError {
                action: "update",
                entity_type: EntityType::Stack,
                raw_id: Some(self.bare_i32()),
                clauses,
                error,
            })
    }
}

impl Stack {
    pub(crate) fn from_row(row: &tokio_postgres::Row) -> Self {
        Self {
            id: row.get("id"),
            repository_id: row.get("repository_id"),
            target_branch: row.get("target_branch"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }

    /// Updates a stack.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails (the update or the log).
    pub async fn update(
        &self,
        tx: &tokio_postgres::Transaction<'_>,
        updates: &UpdateStack,
    ) -> Result<Self, DbQueryError> {
        let ret = match self.id.apply_update_no_log(tx, updates).await? {
            Some(row) => Ok(Self::from_row(&row)),
            None => Ok(self.clone()),
        };
        log_action(
            tx,
            EntityType::Stack,
            self.id.bare_i32(),
            "stack_updated",
            Some(&format!(
                "updated stack {}\n{}",
                self.target_branch,
                updates.to_log_string()
            )),
            None,
        )
        .await?;
        ret
    }
}
