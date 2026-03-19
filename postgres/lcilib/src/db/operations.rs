// SPDX-License-Identifier: GPL-3.0-or-later

use tokio_postgres::{Error, Transaction, Row};

use super::models::*;
use super::util::{self, EntityType};

/// Repository operations
impl Repository {
    /// Create a new repository
    pub async fn create(tx: &Transaction<'_>, new_repo: NewRepository) -> Result<Self, Error> {
        let row = tx
            .query_one(
                r#"
                INSERT INTO repositories (name, path, nixfile_path)
                VALUES ($1, $2, $3)
                RETURNING id, name, path, nixfile_path, created_at
                "#,
                &[&new_repo.name, &new_repo.path, &new_repo.nixfile_path],
            )
            .await?;

        util::log_action(
            tx,
            EntityType::System,
            0,
            "repository_created",
            Some(&format!("Created repository: {}", new_repo.name)),
            None,
        ).await?;

        Ok(Self::from_row(&row))
    }

    /// Find repository by ID
    pub async fn find_by_id(tx: &Transaction<'_>, id: i32) -> Result<Option<Self>, Error> {
        let rows = tx
            .query("SELECT id, name, path, nixfile_path, created_at FROM repositories WHERE id = $1", &[&id])
            .await?;

        Ok(rows.first().map(Self::from_row))
    }

    /// Find repository by path
    pub async fn find_by_path(tx: &Transaction<'_>, path: &str) -> Result<Option<Self>, Error> {
        let rows = tx
            .query("SELECT id, name, path, nixfile_path, created_at FROM repositories WHERE path = $1", &[&path])
            .await?;

        Ok(rows.first().map(Self::from_row))
    }

    /// List all repositories
    pub async fn list_all(tx: &Transaction<'_>) -> Result<Vec<Self>, Error> {
        let rows = tx
            .query("SELECT id, name, path, nixfile_path, created_at FROM repositories ORDER BY name", &[])
            .await?;

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
    pub async fn create(tx: &Transaction<'_>, new_commit: NewCommit) -> Result<Self, Error> {
        let row = tx
            .query_one(
                r#"
                INSERT INTO commits (repository_id, git_commit_id, jj_change_id, review_status, 
                                   should_run_ci, ci_status, commit_type, nix_derivation)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                RETURNING id, repository_id, git_commit_id, jj_change_id, review_status,
                         should_run_ci, ci_status, commit_type, nix_derivation, created_at, updated_at
                "#,
                &[
                    &new_commit.repository_id,
                    &new_commit.git_commit_id,
                    &new_commit.jj_change_id,
                    &new_commit.review_status.as_str(),
                    &new_commit.should_run_ci,
                    &new_commit.ci_status.as_str(),
                    &new_commit.commit_type.as_str(),
                    &new_commit.nix_derivation,
                ],
            )
            .await?;

        let commit = Self::from_row(&row);

        util::log_action(
            tx,
            EntityType::Commit,
            commit.id,
            "commit_created",
            Some(&format!("Created commit: {}", commit.git_commit_id)),
            None,
        ).await?;

        Ok(commit)
    }

    /// Find commit by ID
    pub async fn find_by_id(tx: &Transaction<'_>, id: i32) -> Result<Option<Self>, Error> {
        let rows = tx
            .query(
                r#"
                SELECT id, repository_id, git_commit_id, jj_change_id, review_status,
                       should_run_ci, ci_status, commit_type, nix_derivation, created_at, updated_at
                FROM commits WHERE id = $1
                "#,
                &[&id],
            )
            .await?;

        Ok(rows.first().map(Self::from_row))
    }

    /// Find commit by git commit ID
    pub async fn find_by_git_id(tx: &Transaction<'_>, repository_id: i32, git_commit_id: &str) -> Result<Option<Self>, Error> {
        let rows = tx
            .query(
                r#"
                SELECT id, repository_id, git_commit_id, jj_change_id, review_status,
                       should_run_ci, ci_status, commit_type, nix_derivation, created_at, updated_at
                FROM commits WHERE repository_id = $1 AND git_commit_id = $2
                "#,
                &[&repository_id, &git_commit_id],
            )
            .await?;

        Ok(rows.first().map(Self::from_row))
    }

    /// Find commits by repository
    pub async fn find_by_repository(tx: &Transaction<'_>, repository_id: i32) -> Result<Vec<Self>, Error> {
        let rows = tx
            .query(
                r#"
                SELECT id, repository_id, git_commit_id, jj_change_id, review_status,
                       should_run_ci, ci_status, commit_type, nix_derivation, created_at, updated_at
                FROM commits WHERE repository_id = $1 ORDER BY created_at DESC
                "#,
                &[&repository_id],
            )
            .await?;

        Ok(rows.iter().map(Self::from_row).collect())
    }

    /// Find commits that need CI
    pub async fn find_needing_ci(tx: &Transaction<'_>) -> Result<Vec<Self>, Error> {
        let rows = tx
            .query(
                r#"
                SELECT id, repository_id, git_commit_id, jj_change_id, review_status,
                       should_run_ci, ci_status, commit_type, nix_derivation, created_at, updated_at
                FROM commits 
                WHERE should_run_ci = true AND ci_status = 'unstarted'
                ORDER BY created_at ASC
                "#,
                &[],
            )
            .await?;

        Ok(rows.iter().map(Self::from_row).collect())
    }

    /// Update commit
    pub async fn update(&self, tx: &Transaction<'_>, updates: UpdateCommit) -> Result<Self, Error> {
        let mut set_clauses = Vec::new();
        let mut params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = Vec::new();
        let mut param_count = 1;

        if let Some(review_status) = &updates.review_status {
            set_clauses.push(format!("review_status = ${}", param_count));
            params.push(review_status.as_str2());
            param_count += 1;
        }

        if let Some(should_run_ci) = &updates.should_run_ci {
            set_clauses.push(format!("should_run_ci = ${}", param_count));
            params.push(should_run_ci);
            param_count += 1;
        }

        if let Some(ci_status) = &updates.ci_status {
            set_clauses.push(format!("ci_status = ${}", param_count));
            params.push(ci_status.as_str2());
            param_count += 1;
        }

        if let Some(commit_type) = &updates.commit_type {
            set_clauses.push(format!("commit_type = ${}", param_count));
            params.push(commit_type.as_str2());
            param_count += 1;
        }

        if let Some(nix_derivation) = &updates.nix_derivation {
            set_clauses.push(format!("nix_derivation = ${}", param_count));
            params.push(nix_derivation);
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
                     should_run_ci, ci_status, commit_type, nix_derivation, created_at, updated_at
            "#,
            set_clauses.join(", "),
            param_count
        );

        let row = tx.query_one(&query, &params).await?;
        let updated_commit = Self::from_row(&row);

        util::log_action(
            tx,
            EntityType::Commit,
            self.id,
            "commit_updated",
            Some(&format!("Updated commit: {}", self.git_commit_id)),
            None,
        ).await?;

        Ok(updated_commit)
    }

    fn from_row(row: &Row) -> Self {
        Self {
            id: row.get("id"),
            repository_id: row.get("repository_id"),
            git_commit_id: row.get("git_commit_id"),
            jj_change_id: row.get("jj_change_id"),
            review_status: ReviewStatus::from_str(row.get("review_status")).unwrap(),
            should_run_ci: row.get("should_run_ci"),
            ci_status: CiStatus::from_str(row.get("ci_status")).unwrap(),
            commit_type: CommitType::from_str(row.get("commit_type")).unwrap(),
            nix_derivation: row.get("nix_derivation"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }
}

/// Pull request operations
impl PullRequest {
    /// Create a new pull request
    pub async fn create(tx: &Transaction<'_>, new_pr: NewPullRequest) -> Result<Self, Error> {
        let row = tx
            .query_one(
                r#"
                INSERT INTO pull_requests (repository_id, pr_number, tip_commit_id, review_status,
                                         priority, ok_to_merge, required_reviewers)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                RETURNING id, repository_id, pr_number, tip_commit_id, review_status, priority,
                         ok_to_merge, required_reviewers, created_at, updated_at, synced_at
                "#,
                &[
                    &new_pr.repository_id,
                    &new_pr.pr_number,
                    &new_pr.tip_commit_id,
                    &new_pr.review_status.as_str(),
                    &new_pr.priority,
                    &new_pr.ok_to_merge,
                    &new_pr.required_reviewers,
                ],
            )
            .await?;

        let pr = Self::from_row(&row);

        util::log_action(
            tx,
            EntityType::PullRequest,
            pr.id,
            "pr_created",
            Some(&format!("Created PR #{}", pr.pr_number)),
            None,
        ).await?;

        Ok(pr)
    }

    /// Find pull request by ID
    pub async fn find_by_id(tx: &Transaction<'_>, id: i32) -> Result<Option<Self>, Error> {
        let rows = tx
            .query(
                r#"
                SELECT id, repository_id, pr_number, tip_commit_id, review_status, priority,
                       ok_to_merge, required_reviewers, created_at, updated_at, synced_at
                FROM pull_requests WHERE id = $1
                "#,
                &[&id],
            )
            .await?;

        Ok(rows.first().map(Self::from_row))
    }

    /// Find pull request by PR number
    pub async fn find_by_number(tx: &Transaction<'_>, repository_id: i32, pr_number: i32) -> Result<Option<Self>, Error> {
        let rows = tx
            .query(
                r#"
                SELECT id, repository_id, pr_number, tip_commit_id, review_status, priority,
                       ok_to_merge, required_reviewers, created_at, updated_at, synced_at
                FROM pull_requests WHERE repository_id = $1 AND pr_number = $2
                "#,
                &[&repository_id, &pr_number],
            )
            .await?;

        Ok(rows.first().map(Self::from_row))
    }

    /// Find pull requests ready for merge
    pub async fn find_ready_for_merge(tx: &Transaction<'_>) -> Result<Vec<Self>, Error> {
        let rows = tx
            .query(
                r#"
                SELECT id, repository_id, pr_number, tip_commit_id, review_status, priority,
                       ok_to_merge, required_reviewers, created_at, updated_at, synced_at
                FROM pull_requests 
                WHERE review_status = 'approved' AND ok_to_merge = true
                ORDER BY priority DESC, created_at ASC
                "#,
                &[],
            )
            .await?;

        Ok(rows.iter().map(Self::from_row).collect())
    }

    /// Get commits for this pull request in order
    pub async fn get_commits(&self, tx: &Transaction<'_>) -> Result<Vec<Commit>, Error> {
        let rows = tx
            .query(
                r#"
                SELECT c.id, c.repository_id, c.git_commit_id, c.jj_change_id, c.review_status,
                       c.should_run_ci, c.ci_status, c.commit_type, c.nix_derivation, 
                       c.created_at, c.updated_at
                FROM commits c
                JOIN pr_commits pc ON c.id = pc.commit_id
                WHERE pc.pull_request_id = $1
                ORDER BY pc.sequence_order ASC
                "#,
                &[&self.id],
            )
            .await?;

        Ok(rows.iter().map(Commit::from_row).collect())
    }

    /// Add commit to pull request
    pub async fn add_commit(&self, tx: &Transaction<'_>, commit_id: i32, sequence_order: i32) -> Result<(), Error> {
        tx.execute(
            "INSERT INTO pr_commits (pull_request_id, commit_id, sequence_order) VALUES ($1, $2, $3)",
            &[&self.id, &commit_id, &sequence_order],
        ).await?;

        util::log_action(
            tx,
            EntityType::PullRequest,
            self.id,
            "commit_added",
            Some(&format!("Added commit {} to PR #{}", commit_id, self.pr_number)),
            None,
        ).await?;

        Ok(())
    }

    /// Update pull request
    pub async fn update(&self, tx: &Transaction<'_>, updates: UpdatePullRequest) -> Result<Self, Error> {
        let mut set_clauses = Vec::new();
        let mut params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = Vec::new();
        let mut param_count = 1;

        if let Some(tip_commit_id) = &updates.tip_commit_id {
            set_clauses.push(format!("tip_commit_id = ${}", param_count));
            params.push(tip_commit_id);
            param_count += 1;
        }

        if let Some(review_status) = &updates.review_status {
            set_clauses.push(format!("review_status = ${}", param_count));
            params.push(review_status.as_str2());
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
            RETURNING id, repository_id, pr_number, tip_commit_id, review_status, priority,
                     ok_to_merge, required_reviewers, created_at, updated_at, synced_at
            "#,
            set_clauses.join(", "),
            param_count
        );

        let row = tx.query_one(&query, &params).await?;
        let updated_pr = Self::from_row(&row);

        util::log_action(
            tx,
            EntityType::PullRequest,
            self.id,
            "pr_updated",
            Some(&format!("Updated PR #{}", self.pr_number)),
            None,
        ).await?;

        Ok(updated_pr)
    }

    fn from_row(row: &Row) -> Self {
        Self {
            id: row.get("id"),
            repository_id: row.get("repository_id"),
            pr_number: row.get("pr_number"),
            tip_commit_id: row.get("tip_commit_id"),
            review_status: ReviewStatus::from_str(row.get("review_status")).unwrap(),
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
    pub async fn create(tx: &Transaction<'_>, new_stack: NewStack) -> Result<Self, Error> {
        let row = tx
            .query_one(
                r#"
                INSERT INTO stacks (repository_id, target_branch, status)
                VALUES ($1, $2, $3)
                RETURNING id, repository_id, target_branch, status, created_at, updated_at
                "#,
                &[&new_stack.repository_id, &new_stack.target_branch, &new_stack.status.as_str()],
            )
            .await?;

        let stack = Self::from_row(&row);

        util::log_action(
            tx,
            EntityType::Stack,
            stack.id,
            "stack_created",
            Some(&format!("Created stack for branch: {}", stack.target_branch)),
            None,
        ).await?;

        Ok(stack)
    }

    /// Find stack by ID
    pub async fn find_by_id(tx: &Transaction<'_>, id: i32) -> Result<Option<Self>, Error> {
        let rows = tx
            .query(
                "SELECT id, repository_id, target_branch, status, created_at, updated_at FROM stacks WHERE id = $1",
                &[&id],
            )
            .await?;

        Ok(rows.first().map(Self::from_row))
    }

    /// Find pending stacks
    pub async fn find_pending(tx: &Transaction<'_>) -> Result<Vec<Self>, Error> {
        let rows = tx
            .query(
                r#"
                SELECT id, repository_id, target_branch, status, created_at, updated_at 
                FROM stacks WHERE status = 'pending' ORDER BY created_at ASC
                "#,
                &[],
            )
            .await?;

        Ok(rows.iter().map(Self::from_row).collect())
    }

    /// Get commits for this stack in order
    pub async fn get_commits(&self, tx: &Transaction<'_>) -> Result<Vec<Commit>, Error> {
        let rows = tx
            .query(
                r#"
                SELECT c.id, c.repository_id, c.git_commit_id, c.jj_change_id, c.review_status,
                       c.should_run_ci, c.ci_status, c.commit_type, c.nix_derivation, 
                       c.created_at, c.updated_at
                FROM commits c
                JOIN stack_commits sc ON c.id = sc.commit_id
                WHERE sc.stack_id = $1
                ORDER BY sc.sequence_order ASC
                "#,
                &[&self.id],
            )
            .await?;

        Ok(rows.iter().map(Commit::from_row).collect())
    }

    /// Add commit to stack
    pub async fn add_commit(&self, tx: &Transaction<'_>, commit_id: i32, sequence_order: i32) -> Result<(), Error> {
        tx.execute(
            "INSERT INTO stack_commits (stack_id, commit_id, sequence_order) VALUES ($1, $2, $3)",
            &[&self.id, &commit_id, &sequence_order],
        ).await?;

        util::log_action(
            tx,
            EntityType::Stack,
            self.id,
            "commit_added",
            Some(&format!("Added commit {} to stack", commit_id)),
            None,
        ).await?;

        Ok(())
    }

    /// Update stack
    pub async fn update(&self, tx: &Transaction<'_>, updates: UpdateStack) -> Result<Self, Error> {
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
            params.push(status.as_str2());
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

        let row = tx.query_one(&query, &params).await?;
        let updated_stack = Self::from_row(&row);

        util::log_action(
            tx,
            EntityType::Stack,
            self.id,
            "stack_updated",
            Some(&format!("Updated stack for branch: {}", self.target_branch)),
            None,
        ).await?;

        Ok(updated_stack)
    }

    fn from_row(row: &Row) -> Self {
        Self {
            id: row.get("id"),
            repository_id: row.get("repository_id"),
            target_branch: row.get("target_branch"),
            status: MergeStatus::from_str(row.get("status")).unwrap(),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }
}

/// ACK operations
impl Ack {
    /// Create a new ACK
    pub async fn create(tx: &Transaction<'_>, new_ack: NewAck) -> Result<Self, Error> {
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
                    &new_ack.status.as_str(),
                ],
            )
            .await?;

        let ack = Self::from_row(&row);

        util::log_action(
            tx,
            EntityType::Ack,
            ack.id,
            "ack_created",
            Some(&format!("Created ACK from {}", ack.reviewer_name)),
            None,
        ).await?;

        Ok(ack)
    }

    /// Find ACK by ID
    pub async fn find_by_id(tx: &Transaction<'_>, id: i32) -> Result<Option<Self>, Error> {
        let rows = tx
            .query(
                r#"
                SELECT id, pull_request_id, commit_id, reviewer_name, message, status, created_at, updated_at
                FROM acks WHERE id = $1
                "#,
                &[&id],
            )
            .await?;

        Ok(rows.first().map(Self::from_row))
    }

    /// Find ACKs for pull request
    pub async fn find_by_pull_request(tx: &Transaction<'_>, pull_request_id: i32) -> Result<Vec<Self>, Error> {
        let rows = tx
            .query(
                r#"
                SELECT id, pull_request_id, commit_id, reviewer_name, message, status, created_at, updated_at
                FROM acks WHERE pull_request_id = $1 ORDER BY created_at ASC
                "#,
                &[&pull_request_id],
            )
            .await?;

        Ok(rows.iter().map(Self::from_row).collect())
    }

    /// Find pending ACKs
    pub async fn find_pending(tx: &Transaction<'_>) -> Result<Vec<Self>, Error> {
        let rows = tx
            .query(
                r#"
                SELECT id, pull_request_id, commit_id, reviewer_name, message, status, created_at, updated_at
                FROM acks WHERE status = 'pending' ORDER BY created_at ASC
                "#,
                &[],
            )
            .await?;

        Ok(rows.iter().map(Self::from_row).collect())
    }

    /// Update ACK
    pub async fn update(&self, tx: &Transaction<'_>, updates: UpdateAck) -> Result<Self, Error> {
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
            params.push(status.as_str2());
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

        let row = tx.query_one(&query, &params).await?;
        let updated_ack = Self::from_row(&row);

        util::log_action(
            tx,
            EntityType::Ack,
            self.id,
            "ack_updated",
            Some(&format!("Updated ACK from {}", self.reviewer_name)),
            None,
        ).await?;

        Ok(updated_ack)
    }

    fn from_row(row: &Row) -> Self {
        Self {
            id: row.get("id"),
            pull_request_id: row.get("pull_request_id"),
            commit_id: row.get("commit_id"),
            reviewer_name: row.get("reviewer_name"),
            message: row.get("message"),
            status: AckStatus::from_str(row.get("status")).unwrap(),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }
}

/// Allowed approver operations
impl AllowedApprover {
    /// Create a new allowed approver
    pub async fn create(tx: &Transaction<'_>, new_approver: NewAllowedApprover) -> Result<Self, Error> {
        let row = tx
            .query_one(
                r#"
                INSERT INTO allowed_approvers (repository_id, approver_name)
                VALUES ($1, $2)
                RETURNING id, repository_id, approver_name, created_at
                "#,
                &[&new_approver.repository_id, &new_approver.approver_name],
            )
            .await?;

        let approver = Self::from_row(&row);

        util::log_action(
            tx,
            EntityType::System,
            0,
            "approver_added",
            Some(&format!("Added approver: {}", approver.approver_name)),
            None,
        ).await?;

        Ok(approver)
    }

    /// Find approvers for repository
    pub async fn find_by_repository(tx: &Transaction<'_>, repository_id: i32) -> Result<Vec<Self>, Error> {
        let rows = tx
            .query(
                "SELECT id, repository_id, approver_name, created_at FROM allowed_approvers WHERE repository_id = $1",
                &[&repository_id],
            )
            .await?;

        Ok(rows.iter().map(Self::from_row).collect())
    }

    /// Check if user is allowed approver
    pub async fn is_allowed_approver(tx: &Transaction<'_>, repository_id: i32, approver_name: &str) -> Result<bool, Error> {
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
            .await?;

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
    pub async fn find_by_entity(tx: &Transaction<'_>, entity_type: EntityType, entity_id: i32) -> Result<Vec<Self>, Error> {
        let rows = tx
            .query(
                r#"
                SELECT id, entity_type, entity_id, action, description, reason, timestamp
                FROM logs WHERE entity_type = $1 AND entity_id = $2 ORDER BY timestamp DESC
                "#,
                &[&entity_type.as_str(), &entity_id],
            )
            .await?;

        Ok(rows.iter().map(Self::from_row).collect())
    }

    /// Find recent logs
    pub async fn find_recent(tx: &Transaction<'_>, limit: i64) -> Result<Vec<Self>, Error> {
        let rows = tx
            .query(
                r#"
                SELECT id, entity_type, entity_id, action, description, reason, timestamp
                FROM logs ORDER BY timestamp DESC LIMIT $1
                "#,
                &[&limit],
            )
            .await?;

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
