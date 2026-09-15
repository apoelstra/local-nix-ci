// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::{DateTime, Utc};
use core::fmt;
use std::collections::HashSet;
use postgres_types::{FromSql, ToSql};

use super::{AckStatus, DbCommitId, DbPullRequestId};
use crate::db::{DbQueryError, EntityType, Transaction, util::log_action};
use crate::repo::Upstream;

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

pub(crate) fn domain_for_upstream(upstream: &Upstream) -> &'static str {
    match *upstream {
        Upstream::Github => "github.com",
        Upstream::Forgejo(_) => "git.rust-bitcoin.org",
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
    pub reviewer_score: f32,
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

        tx.inner
            .query_one(&query, &params)
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

        let rows_affected =
            tx.inner
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
            reviewer_score: row.get("reviewer_score"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }

    pub(crate) fn from_row_no_score(row: &tokio_postgres::Row) -> Self {
        Self {
            id: row.get("id"),
            pull_request_id: row.get("pull_request_id"),
            commit_id: row.get("commit_id"),
            reviewer_name: row.get("reviewer_name"),
            message: row.get("message"),
            status: row.get("status"),
            reviewer_score: 0.0,
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }

    /// Retrieves the list of all ACKs which are 'pending' or 'failed' and
    /// which apply to approved PRs.
    ///
    /// Note: `reviewer_score` in the returned ACKs is always 0.0 since this
    /// method does not know which upstream to use for the maintainers lookup.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn find_all_pending_on_approved_prs(
        tx: &Transaction<'_>,
    ) -> Result<Vec<Self>, DbQueryError> {
        let rows = tx
            .inner
            .query(
                r#"
                SELECT
                    a.id,
                    a.pull_request_id,
                    a.commit_id,
                    a.reviewer_name,
                    a.message,
                    a.status,
                    a.created_at,
                    a.updated_at
                FROM acks a
                JOIN pull_requests pr ON a.pull_request_id = pr.id
                WHERE a.status IN ('pending', 'failed')
                  AND pr.review_status = 'approved'
                  AND pr.merge_status IN ('pending', 'draft')
                ORDER BY a.created_at ASC
                "#,
                &[],
            )
            .await
            .map_err(|error| DbQueryError {
                action: "find_all_pending_on_approved_prs",
                entity_type: EntityType::Ack,
                raw_id: None,
                clauses: vec![],
                error,
            })?;

        Ok(rows.iter().map(Self::from_row_no_score).collect())
    }

    /// Find ACKs for pull request on its tip commit.
    ///
    /// Populates `reviewer_score` from the `maintainers` table using the domain
    /// derived from `upstream` (github.com or git.rust-bitcoin.org). Reviewers
    /// not in the table get a score of 0.0.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn find_by_pull_request(
        tx: &Transaction<'_>,
        pull_request_id: DbPullRequestId,
        upstream: &Upstream,
    ) -> Result<Vec<Self>, DbQueryError> {
        let domain = domain_for_upstream(upstream);
        let rows = tx
            .inner
            .query(
                r#"
                SELECT
                    a.id,
                    a.pull_request_id,
                    a.commit_id,
                    a.reviewer_name,
                    a.message,
                    a.status,
                    a.created_at,
                    a.updated_at,
                    COALESCE(m.review_score, 0.0::real) AS reviewer_score
                FROM acks a
                JOIN pull_requests pr ON a.pull_request_id = pr.id
                LEFT JOIN maintainers m
                    ON m.username = a.reviewer_name
                   AND m.domain = $2
                WHERE a.pull_request_id = $1
                  AND a.commit_id = pr.tip_commit_id
                ORDER BY a.created_at ASC
                "#,
                &[&pull_request_id, &domain],
            )
            .await
            .map_err(|error| DbQueryError {
                action: "find_ack_by_pull_request",
                entity_type: EntityType::PullRequest,
                raw_id: Some(pull_request_id.bare_i32()),
                clauses: vec![format!("pull_request_id = {pull_request_id}")],
                error,
            })?;

        Ok(rows.iter().map(Self::from_row).collect())
    }

    /// Delete external ACKs for a pull request that match the given criteria
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn delete_external_acks_not_in_set(
        tx: &Transaction<'_>,
        pull_request_id: DbPullRequestId,
        keep_keys: &HashSet<String>,
    ) -> Result<(), DbQueryError> {
        let rows = tx
            .inner
            .query(
                r#"
                SELECT a.id, a.pull_request_id, a.commit_id, a.reviewer_name, a.message, a.status, a.created_at, a.updated_at
                FROM acks a
                JOIN pull_requests pr ON a.pull_request_id = pr.id
                WHERE a.pull_request_id = $1
                  AND a.commit_id = pr.tip_commit_id
                ORDER BY a.created_at ASC
                "#,
                &[&pull_request_id],
            )
            .await
            .map_err(|error| DbQueryError {
                action: "find_ack_by_pull_request",
                entity_type: EntityType::PullRequest,
                raw_id: Some(pull_request_id.bare_i32()),
                clauses: vec![format!("pull_request_id = {pull_request_id}")],
                error,
            })?;

        for row in &rows {
            let ack = Self::from_row_no_score(row);
            if ack.status == AckStatus::External {
                let key = format!("{}:{}", ack.reviewer_name, ack.message);
                if !keep_keys.contains(&key) {
                    ack.id.delete(tx).await?;
                }
            }
        }

        Ok(())
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
            Some(row) => {
                let mut updated = Self::from_row_no_score(&row);
                updated.reviewer_score = self.reviewer_score;
                Ok(updated)
            }
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
