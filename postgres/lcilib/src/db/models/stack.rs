// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::{DateTime, Utc};
use core::borrow::Borrow;
use core::fmt;
use postgres_types::{FromSql, ToSql};

use super::{CommitToTest, DbCommitId, DbRepositoryId};
use crate::db::{DbQueryError, EntityType, Transaction, util::log_action};

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
        tx: &Transaction<'_>,
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
        tx: &Transaction<'_>,
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

        tx.inner.query_one(&query, &params)
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

    /// Add commit to stack
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn add_commit(
        &self,
        tx: &Transaction<'_>,
        commit_id: DbCommitId,
        sequence_order: i32,
    ) -> Result<(), DbQueryError> {
        tx.inner.execute(
            "INSERT INTO stack_commits (stack_id, commit_id, sequence_order) VALUES ($1, $2, $3)",
            &[&self, &commit_id, &sequence_order],
        )
        .await
        .map_err(|error| DbQueryError {
            action: "insert",
            entity_type: EntityType::Stack,
            raw_id: Some(self.bare_i32()),
            clauses: vec![
                "stack_id".into(),
                "commit_id".into(),
                "sequence_order".into(),
            ],
            error,
        })?;

        log_action(
            tx,
            EntityType::Stack,
            self.bare_i32(),
            "commit_added",
            Some(&format!("Added commit {} to stack", commit_id)),
            None,
        )
        .await?;

        Ok(())
    }

    /// Get commits for this stack in order
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn get_commits(
        &self,
        tx: &Transaction<'_>,
    ) -> Result<Vec<CommitToTest>, DbQueryError> {
        let rows = tx
            .inner
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
                JOIN stack_commits sc ON c.id = sc.commit_id
                LEFT JOIN pr_commits pc ON c.id = pc.commit_id AND pc.is_current = true
                LEFT JOIN pull_requests pr ON pc.pull_request_id = pr.id
                WHERE sc.stack_id = $1
                ORDER BY sc.sequence_order ASC
                "#,
                &[&self],
            )
            .await
            .map_err(|error| DbQueryError {
                action: "get_commits",
                entity_type: EntityType::Stack,
                raw_id: Some(self.bare_i32()),
                clauses: vec![],
                error,
            })?;

        let mut commits_map: std::collections::HashMap<super::DbCommitId, CommitToTest> = std::collections::HashMap::new();
        
        for row in &rows {
            let commit_id = row.get("id");
            
            let commit = commits_map.entry(commit_id).or_insert_with(|| CommitToTest::from_row(row));
            
            // Add PR association if it exists
            if let Ok(pr_id) = row.try_get::<_, super::DbPullRequestId>("pr_id") {
                let commit_type = row.get("commit_type");
                let pr = super::PullRequest {
                    id: pr_id,
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
                commit.prs.push((pr, commit_type));
            }
        }
        
        // Return commits in the original order
        let mut result = Vec::new();
        for row in &rows {
            let commit_id = row.get("id");
            if let Some(commit) = commits_map.remove(&commit_id) {
                result.push(commit);
            }
        }
        
        Ok(result)
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
        tx: &Transaction<'_>,
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

    /// Get all stacks
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn get_all(tx: impl Borrow<Transaction<'_>>) -> Result<Vec<Self>, DbQueryError> {
        let rows = tx
            .borrow()
            .inner
            .query(
                r#"
                SELECT id, repository_id, target_branch, created_at, updated_at
                FROM stacks ORDER BY created_at ASC
                "#,
                &[],
            )
            .await
            .map_err(|error| DbQueryError {
                action: "get_all_stacks",
                entity_type: EntityType::Stack,
                raw_id: None,
                clauses: vec![],
                error,
            })?;

        Ok(rows.iter().map(Self::from_row).collect())
    }

    /// Get all stacks for a given repository and target branch.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn get_all_for_target_branch(
        tx: &Transaction<'_>,
        repo: DbRepositoryId,
        target_branch: &str,
    ) -> Result<Vec<Self>, DbQueryError> {
        let rows = tx
            .inner
            .query(
                r#"
                SELECT id, repository_id, target_branch, created_at, updated_at
                FROM stacks
                WHERE repository_id = $1 AND target_branch = $2
                ORDER BY created_at ASC
                "#,
                &[&repo, &target_branch],
            )
            .await
            .map_err(|error| DbQueryError {
                action: "get_stacks_for_target_branch",
                entity_type: EntityType::Repository,
                raw_id: Some(repo.bare_i32()),
                clauses: vec![],
                error,
            })?;

        Ok(rows.iter().map(Self::from_row).collect())
    }

    /// Deletes the stack from the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn delete(self, tx: &Transaction<'_>) -> Result<(), DbQueryError> {
        tx.execute(
            "DELETE FROM stacks WHERE id = $1",
            &[&self.id],
        )
        .await
        .map_err(|error| DbQueryError {
            action: "delete_stack",
            entity_type: EntityType::Stack,
            raw_id: Some(self.id.bare_i32()),
            clauses: vec![],
            error,
        })
        .map(|_| ())
    }
}
