// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::{DateTime, Utc};
use core::fmt;
use postgres_types::{FromSql, ToSql};

use super::{CommitCounts, CommitToTest, DbCommitId, DbRepositoryId, MergeStatus, ReviewStatus};
use crate::db::{DbQueryError, EntityType, util::log_action};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromSql, ToSql)]
#[postgres(transparent)]
pub struct DbPullRequestId(i32);

impl DbPullRequestId {
    /// An i32 representation of the pull request ID.
    pub fn bare_i32(self) -> i32 {
        self.0
    }
}

impl fmt::Display for DbPullRequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[pull_request {}]", self.0)
    }
}

/// Pull request model
#[derive(Debug, Clone)]
pub struct PullRequest {
    pub id: DbPullRequestId,
    pub repository_id: DbRepositoryId,
    pub pr_number: i32,
    pub title: String,
    pub body: String,
    pub author_login: String,
    pub target_branch: String,
    pub tip_commit_id: DbCommitId,
    pub merge_status: MergeStatus,
    pub review_status: ReviewStatus,
    pub priority: i32,
    pub ok_to_merge: bool,
    pub required_reviewers: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub synced_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewPullRequest {
    pub repository_id: DbRepositoryId,
    pub pr_number: i32,
    pub title: String,
    pub body: String,
    pub author_login: String,
    pub target_branch: String,
    pub tip_commit_id: DbCommitId,
    pub merge_status: MergeStatus,
    pub review_status: ReviewStatus,
    pub priority: i32,
    pub ok_to_merge: bool,
    pub required_reviewers: i32,
}

#[derive(Debug, Clone, Default)]
pub struct UpdatePullRequest {
    pub title: Option<String>,
    pub body: Option<String>,
    pub author_login: Option<String>,
    pub target_branch: Option<String>,
    pub tip_commit_id: Option<DbCommitId>,
    pub merge_status: Option<MergeStatus>,
    pub review_status: Option<ReviewStatus>,
    pub priority: Option<i32>,
    pub ok_to_merge: Option<bool>,
    pub required_reviewers: Option<i32>,
}

impl UpdatePullRequest {
    fn to_params_and_clauses(&self) -> (Vec<&(dyn ToSql + Sync)>, Vec<String>) {
        let mut set_clauses = Vec::new();
        let mut params = Vec::<&(dyn ToSql + Sync)>::new();
        let mut param_count = 1;

        if let Some(title) = &self.title {
            set_clauses.push(format!("title = ${}", param_count));
            params.push(title);
            param_count += 1;
        }

        if let Some(body) = &self.body {
            set_clauses.push(format!("body = ${}", param_count));
            params.push(body);
            param_count += 1;
        }

        if let Some(author_login) = &self.author_login {
            set_clauses.push(format!("author_login = ${}", param_count));
            params.push(author_login);
            param_count += 1;
        }

        if let Some(target_branch) = &self.target_branch {
            set_clauses.push(format!("target_branch = ${}", param_count));
            params.push(target_branch);
            param_count += 1;
        }

        if let Some(tip_commit_id) = &self.tip_commit_id {
            set_clauses.push(format!("tip_commit_id = ${}", param_count));
            params.push(tip_commit_id);
            param_count += 1;
        }

        if let Some(merge_status) = &self.merge_status {
            set_clauses.push(format!("merge_status = ${}", param_count));
            params.push(merge_status);
            param_count += 1;
        }

        if let Some(review_status) = &self.review_status {
            set_clauses.push(format!("review_status = ${}", param_count));
            params.push(review_status);
            param_count += 1;
        }

        if let Some(priority) = &self.priority {
            set_clauses.push(format!("priority = ${}", param_count));
            params.push(priority);
            param_count += 1;
        }

        if let Some(ok_to_merge) = &self.ok_to_merge {
            set_clauses.push(format!("ok_to_merge = ${}", param_count));
            params.push(ok_to_merge);
            param_count += 1;
        }

        if let Some(required_reviewers) = &self.required_reviewers {
            set_clauses.push(format!("required_reviewers = ${}", param_count));
            params.push(required_reviewers);
        }

        (params, set_clauses)
    }

    fn to_log_string(&self) -> String {
        use core::fmt::Write as _;

        let mut ret = String::new();
        if let Some(title) = &self.title {
            let _ = writeln!(ret, "    set title to {}", title);
        }

        if let Some(body) = &self.body {
            let _ = writeln!(ret, "    set body to {}", body);
        }

        if let Some(author_login) = &self.author_login {
            let _ = writeln!(ret, "    set author_login to {}", author_login);
        }

        if let Some(target_branch) = &self.target_branch {
            let _ = writeln!(ret, "    set target_branch to {}", target_branch);
        }

        if let Some(tip_commit_id) = &self.tip_commit_id {
            let _ = writeln!(ret, "    set tip_commit_id to {}", tip_commit_id);
        }

        if let Some(merge_status) = &self.merge_status {
            let _ = writeln!(ret, "    set merge_status to {}", merge_status);
        }

        if let Some(review_status) = &self.review_status {
            let _ = writeln!(ret, "    set review_status to {}", review_status);
        }

        if let Some(priority) = &self.priority {
            let _ = writeln!(ret, "    set priority to {}", priority);
        }

        if let Some(ok_to_merge) = &self.ok_to_merge {
            let _ = writeln!(ret, "    set ok_to_merge to {}", ok_to_merge);
        }

        if let Some(required_reviewers) = &self.required_reviewers {
            let _ = writeln!(ret, "    set required_reviewers to {}", required_reviewers);
        }

        ret
    }
}

impl DbPullRequestId {
    /// Updates a pull request by its database ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails (the update or the log).
    pub async fn apply_update(
        self,
        tx: &tokio_postgres::Transaction<'_>,
        updates: &UpdatePullRequest,
    ) -> Result<Option<tokio_postgres::Row>, DbQueryError> {
        let ret = self.apply_update_no_log(tx, updates).await?;
        log_action(
            tx,
            EntityType::PullRequest,
            self.bare_i32(),
            "pull_request_updated",
            Some(&updates.to_log_string()),
            None,
        )
        .await?;
        Ok(ret)
    }

    /// Updates a pull request by its database ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails (the update or the log).
    pub async fn apply_update_no_log(
        self,
        tx: &tokio_postgres::Transaction<'_>,
        updates: &UpdatePullRequest,
    ) -> Result<Option<tokio_postgres::Row>, DbQueryError> {
        let (mut params, clauses) = updates.to_params_and_clauses();
        if clauses.is_empty() {
            return Ok(None);
        }

        params.push(&self);
        let query = format!(
            r#"
            UPDATE pull_requests SET {}
            WHERE id = ${}
            RETURNING id, repository_id, pr_number, title, body, author_login, target_branch, tip_commit_id, merge_status, review_status,
                     priority, ok_to_merge, required_reviewers, created_at, updated_at, synced_at
            "#,
            clauses.join(", "),
            clauses.len() + 1,
        );

        tx.query_one(&query, &params)
            .await
            .map(Some)
            .map_err(|error| DbQueryError {
                action: "update",
                entity_type: EntityType::PullRequest,
                raw_id: Some(self.bare_i32()),
                clauses,
                error,
            })
    }

    /// Count commits in various states for this PR
    ///
    /// Returns `(total_commits, approved_commits, untested_commits)`
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn get_commit_counts(
        self,
        tx: &tokio_postgres::Transaction<'_>,
    ) -> Result<CommitCounts, DbQueryError> {
        let row = tx
            .query_one(
                r#"
                SELECT
                    COUNT(*) as total,
                    COUNT(CASE WHEN c.review_status = 'approved' THEN 1 END) as approved,
                    COUNT(CASE WHEN c.review_status = 'approved' AND c.ci_status = 'unstarted' AND c.should_run_ci = true THEN 1 END) as untested
                FROM commits c
                JOIN pr_commits pc ON c.id = pc.commit_id
                WHERE pc.pull_request_id = $1 AND pc.is_current = true
                "#,
                &[&self],
            )
            .await
            .map_err(|error| {
                DbQueryError {
                    action: "get_commit_counts",
                    entity_type: EntityType::PullRequest,
                    raw_id: Some(self.bare_i32()),
                    clauses: vec![],
                    error,
                }
            })?;
        Ok(CommitCounts::from_row(&row))
    }

    /// Returns the list of current non-merge commits associated with
    /// this PR, in sequence order.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn get_current_non_merge_commits(
        self,
        tx: &tokio_postgres::Transaction<'_>,
    ) -> Result<Vec<CommitToTest>, DbQueryError> {
        let rows = tx
            .query(
                r#"
                SELECT c.id, c.repository_id, c.git_commit_id, c.jj_change_id, c.review_status,
                       c.should_run_ci, c.ci_status, c.nix_derivation, c.review_text, c.created_at,
                       pc.commit_type
                FROM commits c
                JOIN pr_commits pc ON c.id = pc.commit_id
                WHERE pc.pull_request_id = $1
                AND pc.is_current = true
                ORDER BY pc.sequence_order ASC
                "#,
                &[&self],
            )
            .await
            .map_err(|error| DbQueryError {
                action: "get_current_non_merge_commits",
                entity_type: EntityType::PullRequest,
                raw_id: Some(self.bare_i32()),
                clauses: vec![],
                error,
            })?;
        Ok(rows.iter().map(CommitToTest::from_row).collect())
    }

    /// Returns the list of posted ACKs for the tip commit of this PR.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn get_posted_acks_for_tip(
        self,
        tx: &tokio_postgres::Transaction<'_>,
    ) -> Result<Vec<(String, String)>, DbQueryError> {
        let rows = tx
            .query(
                r#"
                SELECT a.reviewer_name, a.message
                FROM acks a
                JOIN pull_requests pr ON a.pull_request_id = pr.id
                WHERE a.pull_request_id = $1
                AND a.commit_id = pr.tip_commit_id
                AND a.status IN ('posted', 'external')
                ORDER BY a.created_at ASC
                "#,
                &[&self],
            )
            .await
            .map_err(|error| DbQueryError {
                action: "get_posted_acks_for_tip",
                entity_type: EntityType::PullRequest,
                raw_id: Some(self.bare_i32()),
                clauses: vec![],
                error,
            })?;

        Ok(rows
            .iter()
            .map(|row| {
                let reviewer_name: String = row.get("reviewer_name");
                let message: String = row.get("message");
                (reviewer_name, message)
            })
            .collect())
    }
}

impl PullRequest {
    pub(crate) fn from_row(row: &tokio_postgres::Row) -> Self {
        Self {
            id: row.get("id"),
            repository_id: row.get("repository_id"),
            pr_number: row.get("pr_number"),
            title: row.get("title"),
            body: row.get("body"),
            author_login: row.get("author_login"),
            target_branch: row.get("target_branch"),
            tip_commit_id: row.get("tip_commit_id"),
            merge_status: row.get("merge_status"),
            review_status: row.get("review_status"),
            priority: row.get("priority"),
            ok_to_merge: row.get("ok_to_merge"),
            required_reviewers: row.get("required_reviewers"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            synced_at: row.get("synced_at"),
        }
    }

    /// Updates a pull request.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails (the update or the log).
    pub async fn update(
        &self,
        tx: &tokio_postgres::Transaction<'_>,
        updates: &UpdatePullRequest,
    ) -> Result<Self, DbQueryError> {
        let ret = match self.id.apply_update_no_log(tx, updates).await? {
            Some(row) => Ok(Self::from_row(&row)),
            None => Ok(self.clone()),
        };
        log_action(
            tx,
            EntityType::PullRequest,
            self.id.bare_i32(),
            "pull_request_updated",
            Some(&format!(
                "updated pull request #{}\n{}",
                self.pr_number,
                updates.to_log_string()
            )),
            None,
        )
        .await?;
        ret
    }
}
