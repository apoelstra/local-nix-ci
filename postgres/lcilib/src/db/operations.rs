// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashSet;
use tokio_postgres::{Error, Transaction};

use super::models::{
    Ack, AckStatus, AllowedApprover, Commit, CommitToTest, CommitType, DbAckId, DbCommitId,
    DbPrCommitId, DbPullRequestId, DbRepositoryId, DbStackId, LogEntry, NewAck, NewAllowedApprover,
    NewCommit, NewPullRequest, NewStack, PrCommit, PullRequest, Stack,
    UpdateCommit,
};
use super::util::{self, EntityType};
use crate::db::DbQueryError;
use crate::git::CommitId;

/// Error type for database operations with contextual information
#[derive(Debug)]
pub enum OperationError {
    /// Database error with context
    Database {
        /// The underlying database error
        db_error: Error,
        /// The operation that failed (e.g., "`create_repository`", "`find_commit_by_id`")
        operation: String,
        /// The entity type being operated on (e.g., "`Repository`", "`Commit`")
        entity_type: String,
        /// Additional context about the operation
        context: Option<String>,
    },
    AckDeleteQuery(DbQueryError),
    LogQuery(DbQueryError),
    /// Entity not found error
    NotFound {
        /// The operation that failed
        operation: String,
        /// The entity type that was not found
        entity_type: String,
        /// Context about what was being searched for
        context: String,
    },
    /// Wrapped operation error with additional context
    Wrapped {
        /// The underlying operation error
        inner: Box<OperationError>,
        /// The operation that failed
        operation: String,
        /// The entity type being operated on
        entity_type: String,
        /// Additional context about the operation
        context: String,
    },
}

impl OperationError {
    /// Create a new database operation error
    pub fn new(
        db_error: Error,
        operation: &str,
        entity_type: &str,
        context: Option<String>,
    ) -> Self {
        Self::Database {
            db_error,
            operation: operation.to_string(),
            entity_type: entity_type.to_string(),
            context,
        }
    }

    /// Create a database operation error with context
    pub fn with_context(
        db_error: Error,
        operation: &str,
        entity_type: &str,
        context: &str,
    ) -> Self {
        Self::new(db_error, operation, entity_type, Some(context.to_string()))
    }

    /// Create a not found error
    pub fn not_found(operation: &str, entity_type: &str, context: &str) -> Self {
        Self::NotFound {
            operation: operation.to_string(),
            entity_type: entity_type.to_string(),
            context: context.to_string(),
        }
    }

    /// Wrap an existing operation error with additional context
    pub fn wrap(inner: Self, operation: &str, entity_type: &str, context: &str) -> Self {
        Self::Wrapped {
            inner: Box::new(inner),
            operation: operation.to_string(),
            entity_type: entity_type.to_string(),
            context: context.to_string(),
        }
    }
}

impl std::fmt::Display for OperationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database {
                operation,
                entity_type,
                context,
                db_error: _,
            } => match context {
                Some(ctx) => write!(
                    f,
                    "Database operation '{}' failed for {}: {}",
                    operation, entity_type, ctx
                ),
                None => write!(
                    f,
                    "Database operation '{}' failed for {}",
                    operation, entity_type
                ),
            },
            Self::AckDeleteQuery(..) => f.write_str("failed to delete ack"),
            Self::LogQuery(..) => f.write_str("failed to insert log entry"),
            Self::NotFound {
                operation,
                entity_type,
                context,
            } => {
                write!(
                    f,
                    "Operation '{}' failed: {} not found ({})",
                    operation, entity_type, context
                )
            }
            Self::Wrapped {
                inner,
                operation,
                entity_type,
                context,
            } => {
                write!(
                    f,
                    "Operation '{}' failed for {}: {} (caused by: {})",
                    operation, entity_type, context, inner
                )
            }
        }
    }
}

impl std::error::Error for OperationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database { db_error, .. } => Some(db_error),
            Self::AckDeleteQuery(e) => Some(e),
            Self::LogQuery(e) => Some(e),
            Self::NotFound { .. } => None,
            Self::Wrapped { inner, .. } => Some(inner.as_ref()),
        }
    }
}

/// `Commit` operations
impl Commit {
    /// Create a new commit
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn create(
        tx: &Transaction<'_>,
        new_commit: NewCommit,
    ) -> Result<Self, OperationError> {
        let git_commit_str = new_commit.git_commit_id.to_string();
        let row = tx
            .query_one(
                r#"
                INSERT INTO commits (repository_id, git_commit_id, jj_change_id, review_status,
                                   should_run_ci, ci_status, nix_derivation, review_text)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                RETURNING id, repository_id, git_commit_id, jj_change_id, review_status,
                         should_run_ci, ci_status, nix_derivation, review_text, created_at
                "#,
                &[
                    &new_commit.repository_id,
                    &git_commit_str,
                    &new_commit.jj_change_id,
                    &new_commit.review_status,
                    &new_commit.should_run_ci,
                    &new_commit.ci_status,
                    &new_commit.nix_derivation,
                    &new_commit.review_text,
                ],
            )
            .await
            .map_err(|e| {
                OperationError::with_context(
                    e,
                    "create",
                    "Commit",
                    &format!("git_commit_id: {}", git_commit_str),
                )
            })?;

        let commit = Self::from_row(&row);

        util::log_action(
            tx,
            EntityType::Commit,
            commit.id.bare_i32(),
            "commit_created",
            Some(&format!("Created commit: {}", commit.git_commit_id)),
            None,
        )
        .await
        .map_err(OperationError::LogQuery)?;

        Ok(commit)
    }

    /// Find commit by ID
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn find_by_id(
        tx: &Transaction<'_>,
        id: DbCommitId,
    ) -> Result<Option<Self>, OperationError> {
        let rows = tx
            .query(
                r#"
                SELECT id, repository_id, git_commit_id, jj_change_id, review_status,
                       should_run_ci, ci_status, nix_derivation, review_text, created_at
                FROM commits WHERE id = $1
                "#,
                &[&id],
            )
            .await
            .map_err(|e| {
                OperationError::with_context(e, "find_by_id", "Commit", &format!("id: {}", id))
            })?;

        Ok(rows.first().map(Self::from_row))
    }

    /// Find commit by git commit ID
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn find_by_git_id(
        tx: &Transaction<'_>,
        repository_id: DbRepositoryId,
        git_commit_id: &CommitId,
    ) -> Result<Option<Self>, OperationError> {
        let git_commit_str = git_commit_id.to_string();
        let rows = tx
            .query(
                r#"
                SELECT id, repository_id, git_commit_id, jj_change_id, review_status,
                       should_run_ci, ci_status, nix_derivation, review_text, created_at
                FROM commits WHERE repository_id = $1 AND git_commit_id = $2
                "#,
                &[&repository_id, &git_commit_str],
            )
            .await
            .map_err(|e| {
                OperationError::with_context(
                    e,
                    "find_by_git_id",
                    "Commit",
                    &format!(
                        "repository_id: {}, git_commit_id: {}",
                        repository_id, git_commit_str
                    ),
                )
            })?;

        Ok(rows.first().map(Self::from_row))
    }

    /// Find commits by repository
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn find_by_repository(
        tx: &Transaction<'_>,
        repository_id: DbRepositoryId,
    ) -> Result<Vec<Self>, OperationError> {
        let rows = tx
            .query(
                r#"
                SELECT id, repository_id, git_commit_id, jj_change_id, review_status,
                       should_run_ci, ci_status, nix_derivation, review_text, created_at
                FROM commits WHERE repository_id = $1 ORDER BY created_at DESC
                "#,
                &[&repository_id],
            )
            .await
            .map_err(|e| {
                OperationError::with_context(
                    e,
                    "find_by_repository",
                    "Commit",
                    &format!("repository_id: {}", repository_id),
                )
            })?;

        Ok(rows.iter().map(Self::from_row).collect())
    }

    /// Find commits that need CI
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn find_needing_ci(tx: &Transaction<'_>) -> Result<Vec<Self>, OperationError> {
        let rows = tx
            .query(
                r#"
                SELECT id, repository_id, git_commit_id, jj_change_id, review_status,
                       should_run_ci, ci_status, nix_derivation, review_text, created_at
                FROM commits
                WHERE should_run_ci = true AND ci_status = 'unstarted'
                ORDER BY created_at ASC
                "#,
                &[],
            )
            .await
            .map_err(|e| OperationError::new(e, "find_needing_ci", "Commit", None))?;

        Ok(rows.iter().map(Self::from_row).collect())
    }

    /// Update commit with custom log message
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn update_with_custom_log(
        &self,
        tx: &Transaction<'_>,
        updates: UpdateCommit,
        log_action: &str,
        log_description: Option<&str>,
        log_reason: Option<&str>,
    ) -> Result<Self, OperationError> {
        let mut set_clauses = Vec::new();
        let mut params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = Vec::new();
        let mut param_count = 1;

        if let Some(review_status) = &updates.review_status {
            set_clauses.push(format!("review_status = ${}", param_count));
            params.push(review_status);
            param_count += 1;
        }

        if let Some(should_run_ci) = &updates.should_run_ci {
            set_clauses.push(format!("should_run_ci = ${}", param_count));
            params.push(should_run_ci);
            param_count += 1;
        }

        if let Some(ci_status) = &updates.ci_status {
            set_clauses.push(format!("ci_status = ${}", param_count));
            params.push(ci_status);
            param_count += 1;
        }

        if let Some(nix_derivation) = &updates.nix_derivation {
            set_clauses.push(format!("nix_derivation = ${}", param_count));
            params.push(nix_derivation);
            param_count += 1;
        }

        if let Some(review_text) = &updates.review_text {
            set_clauses.push(format!("review_text = ${}", param_count));
            params.push(review_text);
            param_count += 1;
        }

        if set_clauses.is_empty() {
            return Ok(self.clone());
        }

        params.push(&self.id);
        let query = format!(
            r#"
            UPDATE commits SET {}
            WHERE id = ${}
            RETURNING id, repository_id, git_commit_id, jj_change_id, review_status,
                     should_run_ci, ci_status, nix_derivation, review_text, created_at
            "#,
            set_clauses.join(", "),
            param_count
        );

        let row = tx.query_one(&query, &params).await.map_err(|e| {
            OperationError::with_context(
                e,
                "update_with_custom_log",
                "Commit",
                &format!("id: {}, git_commit_id: {}", self.id, self.git_commit_id),
            )
        })?;
        let updated_commit = Self::from_row(&row);

        // Log with custom message
        util::log_action(
            tx,
            EntityType::Commit,
            self.id.bare_i32(),
            log_action,
            log_description,
            log_reason,
        )
        .await
        .map_err(OperationError::LogQuery)?;

        Ok(updated_commit)
    }
}

/// `PullRequest` operations
impl PullRequest {
    /// Get the number of valid ACKs for this pull request
    /// Only counts 'posted' and 'external' ACKs for the tip commit
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn get_ack_count(&self, tx: &Transaction<'_>) -> Result<i64, OperationError> {
        let row = tx
            .query_one(
                r#"
                SELECT COUNT(*)
                FROM acks
                WHERE pull_request_id = $1
                AND commit_id = $2
                AND status IN ('posted', 'external')
                "#,
                &[&self.id, &self.tip_commit_id],
            )
            .await
            .map_err(|e| {
                OperationError::with_context(
                    e,
                    "get_ack_count",
                    "PullRequest",
                    &format!("pr_id: {}, pr_number: {}", self.id, self.pr_number),
                )
            })?;

        Ok(row.get::<_, i64>(0))
    }

    /// Find all PRs that need testing, ordered by priority
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn find_needing_testing_prioritized(
        tx: &Transaction<'_>,
    ) -> Result<Vec<Self>, OperationError> {
        let rows = tx
            .query(
                r#"
                SELECT DISTINCT pr.id, pr.repository_id, pr.pr_number, pr.title, pr.body, pr.author_login, pr.target_branch,
                       pr.tip_commit_id, pr.merge_status, pr.review_status, pr.priority, pr.ok_to_merge,
                       pr.required_reviewers, pr.created_at, pr.updated_at, pr.synced_at
                FROM pull_requests pr
                JOIN pr_commits pc ON pr.id = pc.pull_request_id AND pc.is_current = true
                JOIN commits c ON pc.commit_id = c.id
                WHERE pr.merge_status = 'pending'
                AND pc.commit_type != 'merge'
                AND c.review_status = 'approved'
                AND c.ci_status = 'unstarted'
                AND c.should_run_ci = true
                ORDER BY pr.priority DESC, pr.created_at ASC
                "#,
                &[],
            )
            .await
            .map_err(|e| OperationError::new(e, "find_needing_testing_prioritized", "PullRequest", None))?;

        Ok(rows.iter().map(Self::from_row).collect())
    }

    /// Get the next untested approved commit for this PR
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn get_next_untested_commit(
        &self,
        tx: &Transaction<'_>,
    ) -> Result<Option<CommitToTest>, OperationError> {
        let rows = tx
            .query(
                r#"
                SELECT c.id, c.repository_id, c.git_commit_id, c.jj_change_id, c.review_status,
                       c.should_run_ci, c.ci_status, c.nix_derivation, c.review_text, c.created_at,
                       pc.commit_type,
                       pr.id as pr_id, pr.repository_id as pr_repository_id, pr.pr_number, pr.title, pr.body, 
                       pr.author_login, pr.target_branch, pr.tip_commit_id, pr.merge_status, pr.review_status as pr_review_status,
                       pr.priority, pr.ok_to_merge, pr.required_reviewers, pr.created_at as pr_created_at, 
                       pr.updated_at as pr_updated_at, pr.synced_at as pr_synced_at
                FROM commits c
                JOIN pr_commits pc ON c.id = pc.commit_id
                JOIN pull_requests pr ON pc.pull_request_id = pr.id
                WHERE pc.pull_request_id = $1
                AND pc.is_current = true
                AND c.review_status = 'approved'
                AND c.ci_status = 'unstarted'
                AND c.should_run_ci = true
                ORDER BY pc.sequence_order ASC
                LIMIT 1
                "#,
                &[&self.id],
            )
            .await
            .map_err(|e| {
                OperationError::with_context(
                    e,
                    "get_next_untested_commit",
                    "PullRequest",
                    &format!("pr_id: {}, pr_number: {}", self.id, self.pr_number),
                )
            })?;

        let Some(row) = rows.first() else {
            return Ok(None);
        };

        let commit_type = row.get("commit_type");
        let pr = Self {
            id: row.get("pr_id"),
            repository_id: row.get("pr_repository_id"),
            pr_number: row.get("pr_number"),
            title: row.get("title"),
            body: row.get("body"),
            author_login: row.get("author_login"),
            target_branch: row.get("target_branch"),
            tip_commit_id: row.get("tip_commit_id"),
            merge_status: row.get("merge_status"),
            review_status: row.get("pr_review_status"),
            priority: row.get("priority"),
            ok_to_merge: row.get("ok_to_merge"),
            required_reviewers: row.get("required_reviewers"),
            created_at: row.get("pr_created_at"),
            updated_at: row.get("pr_updated_at"),
            synced_at: row.get("pr_synced_at"),
        };

        let mut commit = CommitToTest::from_row(row);
        commit.prs.push((pr, commit_type));
        
        Ok(Some(commit))
    }

    /// Create a new pull request
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn create(
        tx: &Transaction<'_>,
        new_pr: NewPullRequest,
    ) -> Result<Self, OperationError> {
        let row = tx
            .query_one(
                r#"
                INSERT INTO pull_requests (repository_id, pr_number, title, body, author_login, target_branch, tip_commit_id,
                                         merge_status, review_status, priority, ok_to_merge, required_reviewers)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                RETURNING id, repository_id, pr_number, title, body, author_login, target_branch, tip_commit_id, merge_status, review_status,
                         priority, ok_to_merge, required_reviewers, created_at, updated_at, synced_at
                "#,
                &[
                    &new_pr.repository_id,
                    &new_pr.pr_number,
                    &new_pr.title,
                    &new_pr.body,
                    &new_pr.author_login,
                    &new_pr.target_branch,
                    &new_pr.tip_commit_id,
                    &new_pr.merge_status,
                    &new_pr.review_status,
                    &new_pr.priority,
                    &new_pr.ok_to_merge,
                    &new_pr.required_reviewers,
                ],
            )
            .await
            .map_err(|e| OperationError::with_context(e, "create", "PullRequest", &format!("pr_number: {}", new_pr.pr_number)))?;

        let pr = Self::from_row(&row);

        // Create initial pr_commit record for the tip commit
        PrCommit::create(tx, pr.id, new_pr.tip_commit_id, 1, CommitType::Single)
            .await
            .map_err(|e| {
                OperationError::wrap(
                    e,
                    "create",
                    "PullRequest",
                    "creating initial pr_commit record",
                )
            })?;

        util::log_action(
            tx,
            EntityType::PullRequest,
            pr.id.bare_i32(),
            "pr_created",
            Some(&format!("Created PR #{}", pr.pr_number)),
            None,
        )
        .await
        .map_err(OperationError::LogQuery)?;

        Ok(pr)
    }

    /// Find pull request by ID
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn find_by_id(
        tx: &Transaction<'_>,
        id: DbPullRequestId,
    ) -> Result<Option<Self>, OperationError> {
        let rows = tx
            .query(
                r#"
                SELECT id, repository_id, pr_number, title, body, author_login, target_branch, tip_commit_id, merge_status, review_status,
                       priority, ok_to_merge, required_reviewers, created_at, updated_at, synced_at
                FROM pull_requests WHERE id = $1
                "#,
                &[&id],
            )
            .await
            .map_err(|e| OperationError::with_context(e, "find_by_id", "PullRequest", &format!("id: {}", id)))?;

        Ok(rows.first().map(Self::from_row))
    }

    /// Find pull request by PR number
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn find_by_number(
        tx: &Transaction<'_>,
        repository_id: DbRepositoryId,
        pr_number: i32,
    ) -> Result<Option<Self>, OperationError> {
        let rows = tx
            .query(
                r#"
                SELECT id, repository_id, pr_number, title, body, author_login, target_branch, tip_commit_id, merge_status, review_status,
                       priority, ok_to_merge, required_reviewers, created_at, updated_at, synced_at
                FROM pull_requests WHERE repository_id = $1 AND pr_number = $2
                "#,
                &[&repository_id, &pr_number],
            )
            .await
            .map_err(|e| OperationError::with_context(e, "find_by_number", "PullRequest", &format!("repository_id: {}, pr_number: {}", repository_id, pr_number)))?;

        Ok(rows.first().map(Self::from_row))
    }

    /// Find pull requests ready for merge
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn find_ready_for_merge(tx: &Transaction<'_>) -> Result<Vec<Self>, OperationError> {
        let rows = tx
            .query(
                r#"
                SELECT id, repository_id, pr_number, title, body, author_login, target_branch, tip_commit_id, merge_status, review_status,
                       priority, ok_to_merge, required_reviewers, created_at, updated_at, synced_at
                FROM pull_requests
                WHERE review_status = 'approved' AND ok_to_merge = true
                ORDER BY priority DESC, created_at ASC
                "#,
                &[],
            )
            .await
            .map_err(|e| OperationError::new(e, "find_ready_for_merge", "PullRequest", None))?;

        Ok(rows.iter().map(Self::from_row).collect())
    }

    /// Get commits for this pull request in order
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn get_commits(
        &self,
        tx: &Transaction<'_>,
    ) -> Result<Vec<(Commit, CommitType)>, OperationError> {
        let rows = tx
            .query(
                r#"
                SELECT c.id, c.repository_id, c.git_commit_id, c.jj_change_id, c.review_status,
                       c.should_run_ci, c.ci_status, c.nix_derivation, c.review_text, c.created_at,
                       pc.commit_type
                FROM commits c
                JOIN pr_commits pc ON c.id = pc.commit_id
                WHERE pc.pull_request_id = $1 AND pc.is_current = true
                ORDER BY pc.sequence_order ASC
                "#,
                &[&self.id],
            )
            .await
            .map_err(|e| {
                OperationError::with_context(
                    e,
                    "get_commits",
                    "PullRequest",
                    &format!("pr_id: {}, pr_number: {}", self.id, self.pr_number),
                )
            })?;

        Ok(rows
            .iter()
            .map(|row| (Commit::from_row(row), row.get("commit_type")))
            .collect())
    }

    /// Get previous tip commits for this pull request
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn get_previous_tips(
        &self,
        tx: &Transaction<'_>,
    ) -> Result<Vec<Commit>, OperationError> {
        let rows = tx
            .query(
                r#"
                SELECT c.id, c.repository_id, c.git_commit_id, c.jj_change_id, c.review_status,
                       c.should_run_ci, c.ci_status, c.nix_derivation, c.review_text, c.created_at
                FROM commits c
                JOIN pr_commits pc ON c.id = pc.commit_id
                WHERE pc.pull_request_id = $1 AND pc.is_current = false AND pc.commit_type = 'tip'
                ORDER BY pc.updated_at DESC
                "#,
                &[&self.id],
            )
            .await
            .map_err(|e| {
                OperationError::with_context(
                    e,
                    "get_previous_tips",
                    "PullRequest",
                    &format!("pr_id: {}, pr_number: {}", self.id, self.pr_number),
                )
            })?;

        Ok(rows.iter().map(Commit::from_row).collect())
    }

    /// Add commit to pull request
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn add_commit(
        &self,
        tx: &Transaction<'_>,
        commit_id: DbCommitId,
        sequence_order: i32,
        commit_type: CommitType,
    ) -> Result<(), OperationError> {
        tx.execute(
            "INSERT INTO pr_commits (pull_request_id, commit_id, sequence_order, commit_type) VALUES ($1, $2, $3, $4)",
            &[&self.id, &commit_id, &sequence_order, &commit_type],
        ).await
        .map_err(|e| OperationError::with_context(e, "add_commit", "PullRequest", &format!("pr_id: {}, commit_id: {}", self.id, commit_id)))?;

        util::log_action(
            tx,
            EntityType::PullRequest,
            self.id.bare_i32(),
            "commit_added",
            Some(&format!(
                "Added commit {} to PR #{}",
                commit_id, self.pr_number
            )),
            None,
        )
        .await
        .map_err(OperationError::LogQuery)?;

        Ok(())
    }
}

/// `Stack` operations
impl Stack {
    /// Get PRs associated with this stack
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn get_associated_prs(
        &self,
        tx: &Transaction<'_>,
    ) -> Result<Vec<PullRequest>, OperationError> {
        let rows = tx
            .query(
                r#"
                SELECT DISTINCT pr.id, pr.repository_id, pr.pr_number, pr.title, pr.body, pr.author_login, pr.target_branch,
                       pr.tip_commit_id, pr.merge_status, pr.review_status, pr.priority, pr.ok_to_merge,
                       pr.required_reviewers, pr.created_at, pr.updated_at, pr.synced_at
                FROM pull_requests pr
                JOIN pr_commits pc ON pr.id = pc.pull_request_id AND pc.is_current = true
                JOIN stack_commits sc ON pc.commit_id = sc.commit_id
                WHERE sc.stack_id = $1
                ORDER BY pr.pr_number ASC
                "#,
                &[&self.id],
            )
            .await
            .map_err(|e| OperationError::with_context(e, "get_associated_prs", "Stack", &format!("stack_id: {}, target_branch: {}", self.id, self.target_branch)))?;

        Ok(rows.iter().map(PullRequest::from_row).collect())
    }

    /// Count commits in various states for this stack
    ///
    /// Returns `(total_commits, untested_commits)`
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn get_commit_counts(
        &self,
        tx: &Transaction<'_>,
    ) -> Result<(i64, i64), OperationError> {
        let row = tx
            .query_one(
                r#"
                SELECT
                    COUNT(*) as total,
                    COUNT(CASE WHEN c.review_status = 'approved' AND c.ci_status = 'unstarted' AND c.should_run_ci = true THEN 1 END) as untested
                FROM commits c
                JOIN stack_commits sc ON c.id = sc.commit_id
                WHERE sc.stack_id = $1
                "#,
                &[&self.id],
            )
            .await
            .map_err(|e| OperationError::with_context(e, "get_commit_counts", "Stack", &format!("stack_id: {}, target_branch: {}", self.id, self.target_branch)))?;

        Ok((row.get::<_, i64>("total"), row.get::<_, i64>("untested")))
    }
    /// Create a new stack
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn create(tx: &Transaction<'_>, new_stack: NewStack) -> Result<Self, OperationError> {
        let row = tx
            .query_one(
                r#"
                INSERT INTO stacks (repository_id, target_branch)
                VALUES ($1, $2)
                RETURNING id, repository_id, target_branch, created_at, updated_at
                "#,
                &[&new_stack.repository_id, &new_stack.target_branch],
            )
            .await
            .map_err(|e| {
                OperationError::with_context(
                    e,
                    "create",
                    "Stack",
                    &format!("target_branch: {}", new_stack.target_branch),
                )
            })?;

        let stack = Self::from_row(&row);

        util::log_action(
            tx,
            EntityType::Stack,
            stack.id.bare_i32(),
            "stack_created",
            Some(&format!(
                "Created stack for branch: {}",
                stack.target_branch
            )),
            None,
        )
        .await
        .map_err(OperationError::LogQuery)?;

        Ok(stack)
    }

    /// Find stack by ID
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn find_by_id(
        tx: &Transaction<'_>,
        id: DbStackId,
    ) -> Result<Option<Self>, OperationError> {
        let rows = tx
            .query(
                "SELECT id, repository_id, target_branch, created_at, updated_at FROM stacks WHERE id = $1",
                &[&id],
            )
            .await
            .map_err(|e| OperationError::with_context(e, "find_by_id", "Stack", &format!("id: {}", id)))?;

        Ok(rows.first().map(Self::from_row))
    }
}

/// `Ack` operations
impl Ack {
    /// Create a new ACK
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn create(tx: &Transaction<'_>, new_ack: NewAck) -> Result<Self, OperationError> {
        let row = tx
            .query_one(
                r#"
                INSERT INTO acks (pull_request_id, commit_id, reviewer_name, message, status)
                VALUES ($1, $2, $3, $4, $5)
                RETURNING id, pull_request_id, commit_id, reviewer_name, message, status, created_at, updated_at
                "#,
                &[
                    &new_ack.pull_request_id,
                    &new_ack.commit_id,
                    &new_ack.reviewer_name,
                    &new_ack.message,
                    &new_ack.status,
                ],
            )
            .await
            .map_err(|e| OperationError::with_context(e, "create", "Ack", &format!("reviewer: {}", new_ack.reviewer_name)))?;

        let ack = Self::from_row(&row);

        util::log_action(
            tx,
            EntityType::Ack,
            ack.id.bare_i32(),
            "ack_created",
            Some(&format!("Created ACK from {}", ack.reviewer_name)),
            None,
        )
        .await
        .map_err(OperationError::LogQuery)?;

        Ok(ack)
    }

    /// Find ACK by ID
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn find_by_id(
        tx: &Transaction<'_>,
        id: DbAckId,
    ) -> Result<Option<Self>, OperationError> {
        let rows = tx
            .query(
                r#"
                SELECT id, pull_request_id, commit_id, reviewer_name, message, status, created_at, updated_at
                FROM acks WHERE id = $1
                "#,
                &[&id],
            )
            .await
            .map_err(|e| OperationError::with_context(e, "find_by_id", "Ack", &format!("id: {}", id)))?;

        Ok(rows.first().map(Self::from_row))
    }

    /// Find ACKs for pull request
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn find_by_pull_request(
        tx: &Transaction<'_>,
        pull_request_id: DbPullRequestId,
    ) -> Result<Vec<Self>, OperationError> {
        let rows = tx
            .query(
                r#"
                SELECT id, pull_request_id, commit_id, reviewer_name, message, status, created_at, updated_at
                FROM acks WHERE pull_request_id = $1 ORDER BY created_at ASC
                "#,
                &[&pull_request_id],
            )
            .await
            .map_err(|e| OperationError::with_context(e, "find_by_pull_request", "Ack", &format!("pull_request_id: {}", pull_request_id)))?;

        Ok(rows.iter().map(Self::from_row).collect())
    }

    /// Find pending ACKs
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn find_pending(tx: &Transaction<'_>) -> Result<Vec<Self>, OperationError> {
        let rows = tx
            .query(
                r#"
                SELECT id, pull_request_id, commit_id, reviewer_name, message, status, created_at, updated_at
                FROM acks WHERE status = 'pending' ORDER BY created_at ASC
                "#,
                &[],
            )
            .await
            .map_err(|e| OperationError::new(e, "find_pending", "Ack", None))?;

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
    ) -> Result<(), OperationError> {
        // Find external ACKs that should be deleted
        let existing_acks = Self::find_by_pull_request(tx, pull_request_id).await?;

        for ack in existing_acks {
            if ack.status == AckStatus::External {
                let key = format!("{}:{}", ack.reviewer_name, ack.message);
                if !keep_keys.contains(&key) {
                    ack.id
                        .delete(tx)
                        .await
                        .map_err(OperationError::AckDeleteQuery)?;
                }
            }
        }

        Ok(())
    }
}

/// `AllowedApprover` operations
impl AllowedApprover {
    /// Create a new allowed approver
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn create(
        tx: &Transaction<'_>,
        new_approver: NewAllowedApprover,
    ) -> Result<Self, OperationError> {
        let row = tx
            .query_one(
                r#"
                INSERT INTO allowed_approvers (repository_id, approver_name)
                VALUES ($1, $2)
                RETURNING id, repository_id, approver_name, created_at
                "#,
                &[&new_approver.repository_id, &new_approver.approver_name],
            )
            .await
            .map_err(|e| {
                OperationError::with_context(
                    e,
                    "create",
                    "AllowedApprover",
                    &format!("approver_name: {}", new_approver.approver_name),
                )
            })?;

        let approver = Self::from_row(&row);

        util::log_action(
            tx,
            EntityType::System,
            0,
            "approver_added",
            Some(&format!("Added approver: {}", approver.approver_name)),
            None,
        )
        .await
        .map_err(OperationError::LogQuery)?;

        Ok(approver)
    }

    /// Find approvers for repository
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn find_by_repository(
        tx: &Transaction<'_>,
        repository_id: DbRepositoryId,
    ) -> Result<Vec<Self>, OperationError> {
        let rows = tx
            .query(
                "SELECT id, repository_id, approver_name, created_at FROM allowed_approvers WHERE repository_id = $1",
                &[&repository_id],
            )
            .await
            .map_err(|e| OperationError::with_context(e, "find_by_repository", "AllowedApprover", &format!("repository_id: {}", repository_id)))?;

        Ok(rows.iter().map(Self::from_row).collect())
    }

    /// Check if user is allowed approver
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn is_allowed_approver(
        tx: &Transaction<'_>,
        repository_id: DbRepositoryId,
        approver_name: &str,
    ) -> Result<bool, OperationError> {
        let row = tx
            .query_one(
                r#"
                SELECT EXISTS (
                    SELECT 1 FROM allowed_approvers
                    WHERE repository_id = $1 AND approver_name = $2
                )
                "#,
                &[&repository_id, &approver_name],
            )
            .await
            .map_err(|e| {
                OperationError::with_context(
                    e,
                    "is_allowed_approver",
                    "AllowedApprover",
                    &format!(
                        "repository_id: {}, approver_name: {}",
                        repository_id, approver_name
                    ),
                )
            })?;

        Ok(row.get::<_, bool>(0))
    }
}

/// `LogEntry` operations
impl LogEntry {
    /// Find logs for entity
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn find_by_entity(
        tx: &Transaction<'_>,
        entity_type: EntityType,
        entity_id: i32,
    ) -> Result<Vec<Self>, OperationError> {
        let rows = tx
            .query(
                r#"
                SELECT id, entity_type, entity_id, action, description, reason, timestamp
                FROM logs WHERE entity_type = $1 AND entity_id = $2 ORDER BY timestamp DESC
                "#,
                &[&entity_type, &entity_id],
            )
            .await
            .map_err(|e| {
                OperationError::with_context(
                    e,
                    "find_by_entity",
                    "LogEntry",
                    &format!("entity_type: {:?}, entity_id: {}", entity_type, entity_id),
                )
            })?;

        Ok(rows.iter().map(Self::from_row).collect())
    }

    /// Find recent logs
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn find_recent(
        tx: &Transaction<'_>,
        limit: i64,
    ) -> Result<Vec<Self>, OperationError> {
        let rows = tx
            .query(
                r#"
                SELECT id, entity_type, entity_id, action, description, reason, timestamp
                FROM logs ORDER BY timestamp DESC LIMIT $1
                "#,
                &[&limit],
            )
            .await
            .map_err(|e| {
                OperationError::with_context(
                    e,
                    "find_recent",
                    "LogEntry",
                    &format!("limit: {}", limit),
                )
            })?;

        Ok(rows.iter().map(Self::from_row).collect())
    }
}

/// `PrCommit` operations
impl PrCommit {
    /// Create a new PR-commit relationship
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn create(
        tx: &Transaction<'_>,
        pull_request_id: DbPullRequestId,
        commit_id: DbCommitId,
        sequence_order: i32,
        commit_type: CommitType,
    ) -> Result<Self, OperationError> {
        let row = tx
            .query_one(
                r#"
                INSERT INTO pr_commits (pull_request_id, commit_id, sequence_order, commit_type)
                VALUES ($1, $2, $3, $4)
                RETURNING id, pull_request_id, commit_id, sequence_order, commit_type, is_current, created_at, updated_at
                "#,
                &[&pull_request_id, &commit_id, &sequence_order, &commit_type],
            )
            .await
            .map_err(|e| OperationError::with_context(e, "create", "PrCommit", &format!("pr_id: {}, commit_id: {}", pull_request_id, commit_id)))?;

        Ok(Self::from_row(&row))
    }

    /// Find all PR-commit relationships for a pull request
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn find_current_non_merge_by_pr(
        tx: &Transaction<'_>,
        pull_request_id: DbPullRequestId,
    ) -> Result<Vec<Self>, OperationError> {
        let rows = tx
            .query(
                r#"
                SELECT id, pull_request_id, commit_id, sequence_order, commit_type, is_current, created_at, updated_at
                FROM pr_commits
                WHERE pull_request_id = $1 AND is_current = true AND commit_type != 'merge'
                ORDER BY sequence_order
                "#,
                &[&pull_request_id],
            )
            .await
            .map_err(|e| OperationError::with_context(e, "find_by_pr", "PrCommit", &format!("pull_request_id: {}", pull_request_id)))?;

        Ok(rows.iter().map(Self::from_row).collect())
    }

    /// Find current PR-commit relationships for a pull request
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn find_current_by_pr(
        tx: &Transaction<'_>,
        pull_request_id: DbPullRequestId,
    ) -> Result<Vec<Self>, OperationError> {
        let rows = tx
            .query(
                r#"
                SELECT id, pull_request_id, commit_id, sequence_order, commit_type, is_current, created_at, updated_at
                FROM pr_commits
                WHERE pull_request_id = $1 AND is_current = true
                ORDER BY sequence_order
                "#,
                &[&pull_request_id],
            )
            .await
            .map_err(|e| OperationError::with_context(e, "find_current_by_pr", "PrCommit", &format!("pull_request_id: {}", pull_request_id)))?;

        Ok(rows.iter().map(Self::from_row).collect())
    }

    /// Find previous tip commits for a pull request
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn find_previous_tips_by_pr(
        tx: &Transaction<'_>,
        pull_request_id: DbPullRequestId,
    ) -> Result<Vec<Self>, OperationError> {
        let rows = tx
            .query(
                r#"
                SELECT id, pull_request_id, commit_id, sequence_order, commit_type, is_current, created_at, updated_at
                FROM pr_commits
                WHERE pull_request_id = $1 AND is_current = false AND commit_type = 'tip'
                ORDER BY updated_at DESC
                "#,
                &[&pull_request_id],
            )
            .await
            .map_err(|e| OperationError::with_context(e, "find_previous_tips_by_pr", "PrCommit", &format!("pull_request_id: {}", pull_request_id)))?;

        Ok(rows.iter().map(Self::from_row).collect())
    }

    /// Find all PR-commit relationships for a specific commit
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn find_by_commit(
        tx: &Transaction<'_>,
        commit_id: DbCommitId,
    ) -> Result<Vec<Self>, OperationError> {
        let rows = tx
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
            .map_err(|e| OperationError::with_context(e, "find_by_commit", "PrCommit", &format!("commit_id: {}", commit_id)))?;

        Ok(rows.iter().map(Self::from_row).collect())
    }

    /// Update PR-commit relationship status
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn update_status(
        tx: &Transaction<'_>,
        id: DbPrCommitId,
        sequence_order: Option<i32>,
        commit_type: Option<CommitType>,
        is_current: Option<bool>,
    ) -> Result<(), OperationError> {
        let mut set_clauses = Vec::new();
        let mut params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = Vec::new();
        let mut param_count = 1;

        if let Some(seq) = &sequence_order {
            set_clauses.push(format!("sequence_order = ${}", param_count));
            params.push(seq);
            param_count += 1;
        }

        if let Some(ct) = &commit_type {
            set_clauses.push(format!("commit_type = ${}", param_count));
            params.push(ct);
            param_count += 1;
        }

        if let Some(current) = &is_current {
            set_clauses.push(format!("is_current = ${}", param_count));
            params.push(current);
            param_count += 1;
        }

        if set_clauses.is_empty() {
            return Ok(());
        }

        params.push(&id);
        let query = format!(
            "UPDATE pr_commits SET {} WHERE id = ${}",
            set_clauses.join(", "),
            param_count
        );

        tx.execute(&query, &params).await.map_err(|e| {
            OperationError::with_context(e, "update_status", "PrCommit", &format!("id: {}", id))
        })?;

        Ok(())
    }
}
