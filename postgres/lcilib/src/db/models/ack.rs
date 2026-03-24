// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::{DateTime, Utc};
use core::fmt;
use postgres_types::{FromSql, ToSql};

use super::{AckStatus, DbCommitId, DbPullRequestId};
use crate::db::{DbQueryError, EntityType, Transaction, util::log_action};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromSql, ToSql)]
#[postgres(transparent)]
pub struct DbAckId(i32);

impl DbAckId {
    /// An i32 representation of the ack ID.
    pub fn bare_i32(self) -> i32 {
        self.0
    }
}

impl fmt::Display for DbAckId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[ack {}]", self.0)
    }
}

/// ACK model
#[derive(Debug, Clone)]
pub struct Ack {
    pub id: DbAckId,
    pub pull_request_id: DbPullRequestId,
    pub commit_id: DbCommitId,
    pub reviewer_name: String,
    pub message: String,
    pub status: AckStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewAck {
    pub pull_request_id: DbPullRequestId,
    pub commit_id: DbCommitId,
    pub reviewer_name: String,
    pub message: String,
    pub status: AckStatus,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateAck {
    pub commit_id: Option<DbCommitId>,
    pub message: Option<String>,
    pub status: Option<AckStatus>,
}

impl UpdateAck {
    fn to_params_and_clauses(&self) -> (Vec<&(dyn ToSql + Sync)>, Vec<String>) {
        let mut set_clauses = Vec::new();
        let mut params = Vec::<&(dyn ToSql + Sync)>::new();
        let mut param_count = 1;

        if let Some(commit_id) = &self.commit_id {
            set_clauses.push(format!("commit_id = ${}", param_count));
            params.push(commit_id);
            param_count += 1;
        }

        if let Some(message) = &self.message {
            set_clauses.push(format!("message = ${}", param_count));
            params.push(message);
            param_count += 1;
        }

        if let Some(status) = &self.status {
            set_clauses.push(format!("status = ${}", param_count));
            params.push(status);
        }

        (params, set_clauses)
    }

    fn to_log_string(&self) -> String {
        use core::fmt::Write as _;

        let mut ret = String::new();
        if let Some(commit_id) = &self.commit_id {
            let _ = writeln!(ret, "    set commit_id to {}", commit_id);
        }

        if let Some(message) = &self.message {
            let _ = writeln!(ret, "    set message to {}", message);
        }

        if let Some(status) = &self.status {
            let _ = writeln!(ret, "    set status to {}", status);
        }

        ret
    }
}

impl DbAckId {
    /// Updates an ack by its database ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails (the update or the log).
    pub async fn apply_update(
        self,
        tx: &Transaction<'_>,
        updates: &UpdateAck,
    ) -> Result<Option<tokio_postgres::Row>, DbQueryError> {
        let ret = self.apply_update_no_log(tx, updates).await?;
        log_action(
            tx,
            EntityType::Ack,
            self.bare_i32(),
            "ack_updated",
            Some(&updates.to_log_string()),
            None,
        )
        .await?;
        Ok(ret)
    }

    /// Updates an ack by its database ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails (the update or the log).
    pub async fn apply_update_no_log(
        self,
        tx: &Transaction<'_>,
        updates: &UpdateAck,
    ) -> Result<Option<tokio_postgres::Row>, DbQueryError> {
        let (mut params, clauses) = updates.to_params_and_clauses();
        if clauses.is_empty() {
            return Ok(None);
        }

        params.push(&self);
        let query = format!(
            r#"
            UPDATE acks SET {}
            WHERE id = ${}
            RETURNING id, pull_request_id, commit_id, reviewer_name, message, status, created_at, updated_at
            "#,
            clauses.join(", "),
            clauses.len() + 1,
        );

        tx.inner.query_one(&query, &params)
            .await
            .map(Some)
            .map_err(|error| DbQueryError {
                action: "update",
                entity_type: EntityType::Ack,
                raw_id: Some(self.bare_i32()),
                clauses,
                error,
            })
    }

    /// Deletes an ack by its database ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails (the delete or the log).
    pub async fn delete(self, tx: &Transaction<'_>) -> Result<u64, DbQueryError> {
        let query = "DELETE FROM acks WHERE id = $1";
        let params: &[&(dyn ToSql + Sync)] = &[&self];

        let rows_affected = tx
            .inner
            .execute(query, params)
            .await
            .map_err(|error| DbQueryError {
                action: "delete",
                entity_type: EntityType::Ack,
                raw_id: Some(self.bare_i32()),
                clauses: vec![],
                error,
            })?;

        log_action(
            tx,
            EntityType::Ack,
            self.bare_i32(),
            "ack_deleted",
            Some(&format!("deleted ack {}", self)),
            None,
        )
        .await?;

        Ok(rows_affected)
    }
}

impl Ack {
    pub(crate) fn from_row(row: &tokio_postgres::Row) -> Self {
        Self {
            id: row.get("id"),
            pull_request_id: row.get("pull_request_id"),
            commit_id: row.get("commit_id"),
            reviewer_name: row.get("reviewer_name"),
            message: row.get("message"),
            status: row.get("status"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }

    /// Updates an ack.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails (the update or the log).
    pub async fn update(
        &self,
        tx: &Transaction<'_>,
        updates: &UpdateAck,
    ) -> Result<Self, DbQueryError> {
        let ret = match self.id.apply_update_no_log(tx, updates).await? {
            Some(row) => Ok(Self::from_row(&row)),
            None => Ok(self.clone()),
        };
        log_action(
            tx,
            EntityType::Ack,
            self.id.bare_i32(),
            "ack_updated",
            Some(&format!(
                "updated ack from {}\n{}",
                self.reviewer_name,
                updates.to_log_string()
            )),
            None,
        )
        .await?;
        ret
    }
}
