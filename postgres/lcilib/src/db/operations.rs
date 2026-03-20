// SPDX-License-Identifier: GPL-3.0-or-later

use tokio_postgres::{Error, Transaction, Row};

use super::models::{Repository, NewRepository, Commit, NewCommit, UpdateCommit, PullRequest, NewPullRequest, UpdatePullRequest, Stack, NewStack, UpdateStack, Ack, NewAck, UpdateAck, AllowedApprover, NewAllowedApprover, LogEntry, PrCommit, CommitType};
use super::util::{self, EntityType};

/// Error type for database operations with contextual information
#[derive(Debug)]
pub enum OperationError {
    /// Database error with context
    Database {
        /// The underlying database error
        db_error: Error,
        /// The operation that failed (e.g., "create_repository", "find_commit_by_id")
        operation: String,
        /// The entity type being operated on (e.g., "Repository", "Commit")
        entity_type: String,
        /// Additional context about the operation
        context: Option<String>,
    },
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
    pub fn new(db_error: Error, operation: &str, entity_type: &str, context: Option<String>) -> Self {
        Self::Database {
            db_error,
            operation: operation.to_string(),
            entity_type: entity_type.to_string(),
            context,
        }
    }

    /// Create a database operation error with context
    pub fn with_context(db_error: Error, operation: &str, entity_type: &str, context: &str) -> Self {
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
    pub fn wrap(inner: OperationError, operation: &str, entity_type: &str, context: &str) -> Self {
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
            Self::Database { operation, entity_type, context, db_error } => {
                match context {
                    Some(ctx) => write!(
                        f,
                        "Database operation '{}' failed for {}: {} ({})",
                        operation, entity_type, ctx, db_error
                    ),
                    None => write!(
                        f,
                        "Database operation '{}' failed for {}: {}",
                        operation, entity_type, db_error
                    ),
                }
            }
            Self::NotFound { operation, entity_type, context } => {
                write!(
                    f,
                    "Operation '{}' failed: {} not found ({})",
                    operation, entity_type, context
                )
            }
            Self::Wrapped { inner, operation, entity_type, context } => {
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
            Self::NotFound { .. } => None,
            Self::Wrapped { inner, .. } => Some(inner.as_ref()),
        }
    }
}

/// Repository operations
impl Repository {
    /// Create a new repository
    /// 
    /// # Errors
    /// 
    /// Returns an error if the database operation fails.
    pub async fn create(tx: &Transaction<'_>, new_repo: NewRepository) -> Result<Self, OperationError> {
        let row = tx
            .query_one(
                r#"
                INSERT INTO repositories (name, path, nixfile_path)
                VALUES ($1, $2, $3)
                RETURNING id, name, path, nixfile_path, created_at
                "#,
                &[&new_repo.name, &new_repo.path, &new_repo.nixfile_path],
            )
            .await
            .map_err(|e| OperationError::with_context(e, "create", "Repository", &format!("name: {}", new_repo.name)))?;

        util::log_action(
            tx,
            EntityType::System,
            0,
            "repository_created",
            Some(&format!("Created repository: {}", new_repo.name)),
            None,
        ).await
        .map_err(|e| OperationError::with_context(e, "create", "Repository", "logging creation"))?;

        Ok(Self::from_row(&row))
    }

    /// Find repository by ID
    /// 
    /// # Errors
    /// 
    /// Returns an error if the database operation fails.
    pub async fn find_by_id(tx: &Transaction<'_>, id: i32) -> Result<Option<Self>, OperationError> {
        let rows = tx
            .query("SELECT id, name, path, nixfile_path, created_at FROM repositories WHERE id = $1", &[&id])
            .await
            .map_err(|e| OperationError::with_context(e, "find_by_id", "Repository", &format!("id: {}", id)))?;

        Ok(rows.first().map(Self::from_row))
    }

    /// Find repository by path
    /// 
    /// # Errors
    /// 
    /// Returns an error if the database operation fails.
    pub async fn find_by_path(tx: &Transaction<'_>, path: &str) -> Result<Option<Self>, OperationError> {
        let rows = tx
            .query("SELECT id, name, path, nixfile_path, created_at FROM repositories WHERE path = $1", &[&path])
            .await
            .map_err(|e| OperationError::with_context(e, "find_by_path", "Repository", &format!("path: {}", path)))?;

        Ok(rows.first().map(Self::from_row))
    }

    /// List all repositories
    /// 
    /// # Errors
    /// 
    /// Returns an error if the database operation fails.
    pub async fn list_all(tx: &Transaction<'_>) -> Result<Vec<Self>, OperationError> {
        let rows = tx
            .query("SELECT id, name, path, nixfile_path, created_at FROM repositories ORDER BY name", &[])
            .await
            .map_err(|e| OperationError::new(e, "list_all", "Repository", None))?;

        Ok(rows.iter().map(Self::from_row).collect())
    }

    fn from_row(row: &Row) -> Self {
        Self {
            id: row.get("id"),
            name: row.get("name"),
            path: row.get("path"),
            nixfile_path: row.get("nixfile_path"),
            created_at: row.get("created_at"),
        }
    }
}

/// Commit operations
impl Commit {
    /// Create a new commit
    /// 
    /// # Errors
    /// 
    /// Returns an error if the database operation fails.
    pub async fn create(tx: &Transaction<'_>, new_commit: NewCommit) -> Result<Self, OperationError> {
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
            .map_err(|e| OperationError::with_context(e, "create", "Commit", &format!("git_commit_id: {}", git_commit_str)))?;

        let commit = Self::from_row(&row);

        util::log_action(
            tx,
            EntityType::Commit,
            commit.id,
            "commit_created",
            Some(&format!("Created commit: {}", commit.git_commit_id)),
            None,
        ).await
        .map_err(|e| OperationError::with_context(e, "create", "Commit", "logging creation"))?;

        Ok(commit)
    }

    /// Find commit by ID
    /// 
    /// # Errors
    /// 
    /// Returns an error if the database operation fails.
    pub async fn find_by_id(tx: &Transaction<'_>, id: i32) -> Result<Option<Self>, OperationError> {
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
            .map_err(|e| OperationError::with_context(e, "find_by_id", "Commit", &format!("id: {}", id)))?;

        Ok(rows.first().map(Self::from_row))
    }

    /// Find commit by git commit ID
    /// 
    /// # Errors
    /// 
    /// Returns an error if the database operation fails.
    pub async fn find_by_git_id(tx: &Transaction<'_>, repository_id: i32, git_commit_id: &crate::git::GitCommit) -> Result<Option<Self>, OperationError> {
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
            .map_err(|e| OperationError::with_context(e, "find_by_git_id", "Commit", &format!("repository_id: {}, git_commit_id: {}", repository_id, git_commit_str)))?;

        Ok(rows.first().map(Self::from_row))
    }

    /// Find commits by repository
    /// 
    /// # Errors
    /// 
    /// Returns an error if the database operation fails.
    pub async fn find_by_repository(tx: &Transaction<'_>, repository_id: i32) -> Result<Vec<Self>, OperationError> {
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
            .map_err(|e| OperationError::with_context(e, "find_by_repository", "Commit", &format!("repository_id: {}", repository_id)))?;

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

    /// Update commit
    /// 
    /// # Errors
    /// 
    /// Returns an error if the database operation fails.
    pub async fn update(&self, tx: &Transaction<'_>, updates: UpdateCommit) -> Result<Self, OperationError> {
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

        let row = tx.query_one(&query, &params).await
            .map_err(|e| OperationError::with_context(e, "update", "Commit", &format!("id: {}, git_commit_id: {}", self.id, self.git_commit_id)))?;
        let updated_commit = Self::from_row(&row);

        // Log review text changes with the full new text
        if let Some(Some(new_review_text)) = &updates.review_text {
            util::log_action(
                tx,
                EntityType::Commit,
                self.id,
                "review_text_updated",
                Some(&format!("Updated review text for commit: {}", self.git_commit_id)),
                Some(new_review_text),
            ).await
            .map_err(|e| OperationError::with_context(e, "update", "Commit", "logging review text update"))?;
        } else {
            util::log_action(
                tx,
                EntityType::Commit,
                self.id,
                "commit_updated",
                Some(&format!("Updated commit: {}", self.git_commit_id)),
                None,
            ).await
            .map_err(|e| OperationError::with_context(e, "update", "Commit", "logging update"))?;
        }

        Ok(updated_commit)
    }

    /// Apply updates to a commit by ID with transaction management
    /// 
    /// # Errors
    /// 
    /// Returns an error if the database operation fails.
    pub async fn apply_update_by_id(tx: &Transaction<'_>, commit_id: i32, updates: UpdateCommit) -> Result<Self, OperationError> {
        // First find the commit
        let Some(commit) = Self::find_by_id(tx, commit_id).await? else {
            return Err(OperationError::not_found(
                "apply_update_by_id",
                "Commit",
                &format!("commit_id: {}", commit_id)
            ));
        };

        // Apply the update
        commit.update(tx, updates).await
    }

    fn from_row(row: &Row) -> Self {
        Self {
            id: row.get("id"),
            repository_id: row.get("repository_id"),
            git_commit_id: row.get("git_commit_id"),
            jj_change_id: row.get("jj_change_id"),
            review_status: row.get("review_status"),
            should_run_ci: row.get("should_run_ci"),
            ci_status: row.get("ci_status"),
            nix_derivation: row.get("nix_derivation"),
            review_text: row.get("review_text"),
            created_at: row.get("created_at"),
        }
    }
}

/// Pull request operations
impl PullRequest {
    /// Create a new pull request
    /// 
    /// # Errors
    /// 
    /// Returns an error if the database operation fails.
    pub async fn create(tx: &Transaction<'_>, new_pr: NewPullRequest) -> Result<Self, OperationError> {
        let row = tx
            .query_one(
                r#"
                INSERT INTO pull_requests (repository_id, pr_number, title, body, tip_commit_id, 
                                         review_status, priority, ok_to_merge, required_reviewers)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                RETURNING id, repository_id, pr_number, title, body, tip_commit_id, review_status, 
                         priority, ok_to_merge, required_reviewers, created_at, updated_at, synced_at
                "#,
                &[
                    &new_pr.repository_id,
                    &new_pr.pr_number,
                    &new_pr.title,
                    &new_pr.body,
                    &new_pr.tip_commit_id,
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
        PrCommit::create(tx, pr.id, new_pr.tip_commit_id, 1, CommitType::Single).await
            .map_err(|e| OperationError::wrap(e, "create", "PullRequest", "creating initial pr_commit record"))?;

        util::log_action(
            tx,
            EntityType::PullRequest,
            pr.id,
            "pr_created",
            Some(&format!("Created PR #{}", pr.pr_number)),
            None,
        ).await
        .map_err(|e| OperationError::with_context(e, "create", "PullRequest", "logging creation"))?;

        Ok(pr)
    }

    /// Find pull request by ID
    /// 
    /// # Errors
    /// 
    /// Returns an error if the database operation fails.
    pub async fn find_by_id(tx: &Transaction<'_>, id: i32) -> Result<Option<Self>, OperationError> {
        let rows = tx
            .query(
                r#"
                SELECT id, repository_id, pr_number, title, body, tip_commit_id, review_status, 
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
    pub async fn find_by_number(tx: &Transaction<'_>, repository_id: i32, pr_number: i32) -> Result<Option<Self>, OperationError> {
        let rows = tx
            .query(
                r#"
                SELECT id, repository_id, pr_number, title, body, tip_commit_id, review_status, 
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
                SELECT id, repository_id, pr_number, title, body, tip_commit_id, review_status, 
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
    pub async fn get_commits(&self, tx: &Transaction<'_>) -> Result<Vec<(Commit, CommitType)>, OperationError> {
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
            .map_err(|e| OperationError::with_context(e, "get_commits", "PullRequest", &format!("pr_id: {}, pr_number: {}", self.id, self.pr_number)))?;

        Ok(rows.iter().map(|row| (Commit::from_row(row), row.get("commit_type"))).collect())
    }

    /// Get previous tip commits for this pull request
    /// 
    /// # Errors
    /// 
    /// Returns an error if the database operation fails.
    pub async fn get_previous_tips(&self, tx: &Transaction<'_>) -> Result<Vec<Commit>, OperationError> {
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
            .map_err(|e| OperationError::with_context(e, "get_previous_tips", "PullRequest", &format!("pr_id: {}, pr_number: {}", self.id, self.pr_number)))?;

        Ok(rows.iter().map(Commit::from_row).collect())
    }

    /// Add commit to pull request
    /// 
    /// # Errors
    /// 
    /// Returns an error if the database operation fails.
    pub async fn add_commit(&self, tx: &Transaction<'_>, commit_id: i32, sequence_order: i32, commit_type: CommitType) -> Result<(), OperationError> {
        tx.execute(
            "INSERT INTO pr_commits (pull_request_id, commit_id, sequence_order, commit_type) VALUES ($1, $2, $3, $4)",
            &[&self.id, &commit_id, &sequence_order, &commit_type],
        ).await
        .map_err(|e| OperationError::with_context(e, "add_commit", "PullRequest", &format!("pr_id: {}, commit_id: {}", self.id, commit_id)))?;

        util::log_action(
            tx,
            EntityType::PullRequest,
            self.id,
            "commit_added",
            Some(&format!("Added commit {} to PR #{}", commit_id, self.pr_number)),
            None,
        ).await
        .map_err(|e| OperationError::with_context(e, "add_commit", "PullRequest", "logging commit addition"))?;

        Ok(())
    }

    /// Update pull request
    /// 
    /// # Errors
    /// 
    /// Returns an error if the database operation fails.
    pub async fn update(&self, tx: &Transaction<'_>, updates: UpdatePullRequest) -> Result<Self, OperationError> {
        let mut set_clauses = Vec::new();
        let mut params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = Vec::new();
        let mut param_count = 1;

        if let Some(title) = &updates.title {
            set_clauses.push(format!("title = ${}", param_count));
            params.push(title);
            param_count += 1;
        }

        if let Some(body) = &updates.body {
            set_clauses.push(format!("body = ${}", param_count));
            params.push(body);
            param_count += 1;
        }

        if let Some(tip_commit_id) = &updates.tip_commit_id {
            set_clauses.push(format!("tip_commit_id = ${}", param_count));
            params.push(tip_commit_id);
            param_count += 1;
        }

        if let Some(review_status) = &updates.review_status {
            set_clauses.push(format!("review_status = ${}", param_count));
            params.push(review_status);
            param_count += 1;
        }

        if let Some(priority) = &updates.priority {
            set_clauses.push(format!("priority = ${}", param_count));
            params.push(priority);
            param_count += 1;
        }

        if let Some(ok_to_merge) = &updates.ok_to_merge {
            set_clauses.push(format!("ok_to_merge = ${}", param_count));
            params.push(ok_to_merge);
            param_count += 1;
        }

        if let Some(required_reviewers) = &updates.required_reviewers {
            set_clauses.push(format!("required_reviewers = ${}", param_count));
            params.push(required_reviewers);
            param_count += 1;
        }

        if set_clauses.is_empty() {
            return Ok(self.clone());
        }

        params.push(&self.id);
        let query = format!(
            r#"
            UPDATE pull_requests SET {}
            WHERE id = ${}
            RETURNING id, repository_id, pr_number, title, body, tip_commit_id, review_status, 
                     priority, ok_to_merge, required_reviewers, created_at, updated_at, synced_at
            "#,
            set_clauses.join(", "),
            param_count
        );

        let row = tx.query_one(&query, &params).await
            .map_err(|e| OperationError::with_context(e, "update", "PullRequest", &format!("id: {}, pr_number: {}", self.id, self.pr_number)))?;
        let updated_pr = Self::from_row(&row);

        util::log_action(
            tx,
            EntityType::PullRequest,
            self.id,
            "pr_updated",
            Some(&format!("Updated PR #{}", self.pr_number)),
            None,
        ).await
        .map_err(|e| OperationError::with_context(e, "update", "PullRequest", "logging update"))?;

        Ok(updated_pr)
    }

    fn from_row(row: &Row) -> Self {
        Self {
            id: row.get("id"),
            repository_id: row.get("repository_id"),
            pr_number: row.get("pr_number"),
            title: row.get("title"),
            body: row.get("body"),
            tip_commit_id: row.get("tip_commit_id"),
            review_status: row.get("review_status"),
            priority: row.get("priority"),
            ok_to_merge: row.get("ok_to_merge"),
            required_reviewers: row.get("required_reviewers"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            synced_at: row.get("synced_at"),
        }
    }
}

/// Stack operations
impl Stack {
    /// Create a new stack
    /// 
    /// # Errors
    /// 
    /// Returns an error if the database operation fails.
    pub async fn create(tx: &Transaction<'_>, new_stack: NewStack) -> Result<Self, OperationError> {
        let row = tx
            .query_one(
                r#"
                INSERT INTO stacks (repository_id, target_branch, status)
                VALUES ($1, $2, $3)
                RETURNING id, repository_id, target_branch, status, created_at, updated_at
                "#,
                &[&new_stack.repository_id, &new_stack.target_branch, &new_stack.status],
            )
            .await
            .map_err(|e| OperationError::with_context(e, "create", "Stack", &format!("target_branch: {}", new_stack.target_branch)))?;

        let stack = Self::from_row(&row);

        util::log_action(
            tx,
            EntityType::Stack,
            stack.id,
            "stack_created",
            Some(&format!("Created stack for branch: {}", stack.target_branch)),
            None,
        ).await
        .map_err(|e| OperationError::with_context(e, "create", "Stack", "logging creation"))?;

        Ok(stack)
    }

    /// Find stack by ID
    /// 
    /// # Errors
    /// 
    /// Returns an error if the database operation fails.
    pub async fn find_by_id(tx: &Transaction<'_>, id: i32) -> Result<Option<Self>, OperationError> {
        let rows = tx
            .query(
                "SELECT id, repository_id, target_branch, status, created_at, updated_at FROM stacks WHERE id = $1",
                &[&id],
            )
            .await
            .map_err(|e| OperationError::with_context(e, "find_by_id", "Stack", &format!("id: {}", id)))?;

        Ok(rows.first().map(Self::from_row))
    }

    /// Find pending stacks
    /// 
    /// # Errors
    /// 
    /// Returns an error if the database operation fails.
    pub async fn find_pending(tx: &Transaction<'_>) -> Result<Vec<Self>, OperationError> {
        let rows = tx
            .query(
                r#"
                SELECT id, repository_id, target_branch, status, created_at, updated_at 
                FROM stacks WHERE status = 'pending' ORDER BY created_at ASC
                "#,
                &[],
            )
            .await
            .map_err(|e| OperationError::new(e, "find_pending", "Stack", None))?;

        Ok(rows.iter().map(Self::from_row).collect())
    }

    /// Get commits for this stack in order
    /// 
    /// # Errors
    /// 
    /// Returns an error if the database operation fails.
    pub async fn get_commits(&self, tx: &Transaction<'_>) -> Result<Vec<Commit>, OperationError> {
        let rows = tx
            .query(
                r#"
                SELECT c.id, c.repository_id, c.git_commit_id, c.jj_change_id, c.review_status,
                       c.should_run_ci, c.ci_status, c.nix_derivation, c.review_text, c.created_at
                FROM commits c
                JOIN stack_commits sc ON c.id = sc.commit_id
                WHERE sc.stack_id = $1
                ORDER BY sc.sequence_order ASC
                "#,
                &[&self.id],
            )
            .await
            .map_err(|e| OperationError::with_context(e, "get_commits", "Stack", &format!("stack_id: {}, target_branch: {}", self.id, self.target_branch)))?;

        Ok(rows.iter().map(Commit::from_row).collect())
    }

    /// Add commit to stack
    /// 
    /// # Errors
    /// 
    /// Returns an error if the database operation fails.
    pub async fn add_commit(&self, tx: &Transaction<'_>, commit_id: i32, sequence_order: i32) -> Result<(), OperationError> {
        tx.execute(
            "INSERT INTO stack_commits (stack_id, commit_id, sequence_order) VALUES ($1, $2, $3)",
            &[&self.id, &commit_id, &sequence_order],
        ).await
        .map_err(|e| OperationError::with_context(e, "add_commit", "Stack", &format!("stack_id: {}, commit_id: {}", self.id, commit_id)))?;

        util::log_action(
            tx,
            EntityType::Stack,
            self.id,
            "commit_added",
            Some(&format!("Added commit {} to stack", commit_id)),
            None,
        ).await
        .map_err(|e| OperationError::with_context(e, "add_commit", "Stack", "logging commit addition"))?;

        Ok(())
    }

    /// Update stack
    /// 
    /// # Errors
    /// 
    /// Returns an error if the database operation fails.
    pub async fn update(&self, tx: &Transaction<'_>, updates: UpdateStack) -> Result<Self, OperationError> {
        let mut set_clauses = Vec::new();
        let mut params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = Vec::new();
        let mut param_count = 1;

        if let Some(target_branch) = &updates.target_branch {
            set_clauses.push(format!("target_branch = ${}", param_count));
            params.push(target_branch);
            param_count += 1;
        }

        if let Some(status) = &updates.status {
            set_clauses.push(format!("status = ${}", param_count));
            params.push(status);
            param_count += 1;
        }

        if set_clauses.is_empty() {
            return Ok(self.clone());
        }

        params.push(&self.id);
        let query = format!(
            r#"
            UPDATE stacks SET {}
            WHERE id = ${}
            RETURNING id, repository_id, target_branch, status, created_at, updated_at
            "#,
            set_clauses.join(", "),
            param_count
        );

        let row = tx.query_one(&query, &params).await
            .map_err(|e| OperationError::with_context(e, "update", "Stack", &format!("id: {}, target_branch: {}", self.id, self.target_branch)))?;
        let updated_stack = Self::from_row(&row);

        util::log_action(
            tx,
            EntityType::Stack,
            self.id,
            "stack_updated",
            Some(&format!("Updated stack for branch: {}", self.target_branch)),
            None,
        ).await
        .map_err(|e| OperationError::with_context(e, "update", "Stack", "logging update"))?;

        Ok(updated_stack)
    }

    fn from_row(row: &Row) -> Self {
        Self {
            id: row.get("id"),
            repository_id: row.get("repository_id"),
            target_branch: row.get("target_branch"),
            status: row.get("status"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }
}

/// ACK operations
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
            ack.id,
            "ack_created",
            Some(&format!("Created ACK from {}", ack.reviewer_name)),
            None,
        ).await
        .map_err(|e| OperationError::with_context(e, "create", "Ack", "logging creation"))?;

        Ok(ack)
    }

    /// Find ACK by ID
    /// 
    /// # Errors
    /// 
    /// Returns an error if the database operation fails.
    pub async fn find_by_id(tx: &Transaction<'_>, id: i32) -> Result<Option<Self>, OperationError> {
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
    pub async fn find_by_pull_request(tx: &Transaction<'_>, pull_request_id: i32) -> Result<Vec<Self>, OperationError> {
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

    /// Update ACK
    /// 
    /// # Errors
    /// 
    /// Returns an error if the database operation fails.
    pub async fn update(&self, tx: &Transaction<'_>, updates: UpdateAck) -> Result<Self, OperationError> {
        let mut set_clauses = Vec::new();
        let mut params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = Vec::new();
        let mut param_count = 1;

        if let Some(commit_id) = &updates.commit_id {
            set_clauses.push(format!("commit_id = ${}", param_count));
            params.push(commit_id);
            param_count += 1;
        }

        if let Some(message) = &updates.message {
            set_clauses.push(format!("message = ${}", param_count));
            params.push(message);
            param_count += 1;
        }

        if let Some(status) = &updates.status {
            set_clauses.push(format!("status = ${}", param_count));
            params.push(status);
            param_count += 1;
        }

        if set_clauses.is_empty() {
            return Ok(self.clone());
        }

        params.push(&self.id);
        let query = format!(
            r#"
            UPDATE acks SET {}
            WHERE id = ${}
            RETURNING id, pull_request_id, commit_id, reviewer_name, message, status, created_at, updated_at
            "#,
            set_clauses.join(", "),
            param_count
        );

        let row = tx.query_one(&query, &params).await
            .map_err(|e| OperationError::with_context(e, "update", "Ack", &format!("id: {}, reviewer: {}", self.id, self.reviewer_name)))?;
        let updated_ack = Self::from_row(&row);

        util::log_action(
            tx,
            EntityType::Ack,
            self.id,
            "ack_updated",
            Some(&format!("Updated ACK from {}", self.reviewer_name)),
            None,
        ).await
        .map_err(|e| OperationError::with_context(e, "update", "Ack", "logging update"))?;

        Ok(updated_ack)
    }

    fn from_row(row: &Row) -> Self {
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
}

/// Allowed approver operations
impl AllowedApprover {
    /// Create a new allowed approver
    /// 
    /// # Errors
    /// 
    /// Returns an error if the database operation fails.
    pub async fn create(tx: &Transaction<'_>, new_approver: NewAllowedApprover) -> Result<Self, OperationError> {
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
            .map_err(|e| OperationError::with_context(e, "create", "AllowedApprover", &format!("approver_name: {}", new_approver.approver_name)))?;

        let approver = Self::from_row(&row);

        util::log_action(
            tx,
            EntityType::System,
            0,
            "approver_added",
            Some(&format!("Added approver: {}", approver.approver_name)),
            None,
        ).await
        .map_err(|e| OperationError::with_context(e, "create", "AllowedApprover", "logging creation"))?;

        Ok(approver)
    }

    /// Find approvers for repository
    /// 
    /// # Errors
    /// 
    /// Returns an error if the database operation fails.
    pub async fn find_by_repository(tx: &Transaction<'_>, repository_id: i32) -> Result<Vec<Self>, OperationError> {
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
    pub async fn is_allowed_approver(tx: &Transaction<'_>, repository_id: i32, approver_name: &str) -> Result<bool, OperationError> {
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
            .map_err(|e| OperationError::with_context(e, "is_allowed_approver", "AllowedApprover", &format!("repository_id: {}, approver_name: {}", repository_id, approver_name)))?;

        Ok(row.get::<_, bool>(0))
    }

    fn from_row(row: &Row) -> Self {
        Self {
            id: row.get("id"),
            repository_id: row.get("repository_id"),
            approver_name: row.get("approver_name"),
            created_at: row.get("created_at"),
        }
    }
}

/// Log entry operations
impl LogEntry {
    /// Find logs for entity
    /// 
    /// # Errors
    /// 
    /// Returns an error if the database operation fails.
    pub async fn find_by_entity(tx: &Transaction<'_>, entity_type: EntityType, entity_id: i32) -> Result<Vec<Self>, OperationError> {
        let rows = tx
            .query(
                r#"
                SELECT id, entity_type, entity_id, action, description, reason, timestamp
                FROM logs WHERE entity_type = $1 AND entity_id = $2 ORDER BY timestamp DESC
                "#,
                &[&entity_type, &entity_id],
            )
            .await
            .map_err(|e| OperationError::with_context(e, "find_by_entity", "LogEntry", &format!("entity_type: {:?}, entity_id: {}", entity_type, entity_id)))?;

        Ok(rows.iter().map(Self::from_row).collect())
    }

    /// Find recent logs
    /// 
    /// # Errors
    /// 
    /// Returns an error if the database operation fails.
    pub async fn find_recent(tx: &Transaction<'_>, limit: i64) -> Result<Vec<Self>, OperationError> {
        let rows = tx
            .query(
                r#"
                SELECT id, entity_type, entity_id, action, description, reason, timestamp
                FROM logs ORDER BY timestamp DESC LIMIT $1
                "#,
                &[&limit],
            )
            .await
            .map_err(|e| OperationError::with_context(e, "find_recent", "LogEntry", &format!("limit: {}", limit)))?;

        Ok(rows.iter().map(Self::from_row).collect())
    }

    fn from_row(row: &Row) -> Self {
        Self {
            id: row.get("id"),
            entity_type: row.get("entity_type"),
            entity_id: row.get("entity_id"),
            action: row.get("action"),
            description: row.get("description"),
            reason: row.get("reason"),
            timestamp: row.get("timestamp"),
        }
    }
}

/// PrCommit operations
impl PrCommit {
    /// Create a new PR-commit relationship
    /// 
    /// # Errors
    /// 
    /// Returns an error if the database operation fails.
    pub async fn create(
        tx: &Transaction<'_>,
        pull_request_id: i32,
        commit_id: i32,
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
    pub async fn find_by_pr(
        tx: &Transaction<'_>,
        pull_request_id: i32,
    ) -> Result<Vec<Self>, OperationError> {
        let rows = tx
            .query(
                r#"
                SELECT id, pull_request_id, commit_id, sequence_order, commit_type, is_current, created_at, updated_at
                FROM pr_commits 
                WHERE pull_request_id = $1 
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
        pull_request_id: i32,
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
        pull_request_id: i32,
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
        commit_id: i32,
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
        id: i32,
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

        tx.execute(&query, &params).await
            .map_err(|e| OperationError::with_context(e, "update_status", "PrCommit", &format!("id: {}", id)))?;

        Ok(())
    }

    fn from_row(row: &Row) -> Self {
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
}
