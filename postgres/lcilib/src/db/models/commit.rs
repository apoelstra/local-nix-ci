// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::{DateTime, Utc};
use core::fmt;
use postgres_types::{FromSql, ToSql};

use super::ReviewStatus;
use crate::db::{DbQueryError, EntityType, Transaction, models::CommitType, util::log_action};
use crate::git::CommitId;
use crate::jj::ChangeId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromSql, ToSql)]
#[postgres(transparent)]
pub struct DbCommitId(i32);

impl DbCommitId {
    /// An i32 representation of the commit ID.
    pub fn bare_i32(self) -> i32 {
        self.0
    }
}

impl fmt::Display for DbCommitId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[commit {}]", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromSql, ToSql)]
#[postgres(name = "ci_status")]
pub enum CiStatus {
    #[postgres(name = "unstarted")]
    Unstarted,
    #[postgres(name = "skipped")]
    Skipped,
    #[postgres(name = "failed")]
    Failed,
    #[postgres(name = "passed")]
    Passed,
}

impl fmt::Display for CiStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unstarted => write!(f, "unstarted"),
            Self::Skipped => write!(f, "skipped"),
            Self::Failed => write!(f, "failed"),
            Self::Passed => write!(f, "passed"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Commit {
    pub id: DbCommitId,
    pub repository_id: super::DbRepositoryId,
    pub git_commit_id: CommitId,
    pub jj_change_id: ChangeId,
    pub review_status: ReviewStatus,
    pub should_run_ci: bool,
    pub ci_status: CiStatus,
    pub nix_derivation: Option<String>,
    pub review_text: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewCommit {
    pub repository_id: super::DbRepositoryId,
    pub git_commit_id: CommitId,
    pub jj_change_id: ChangeId,
    pub review_status: ReviewStatus,
    pub should_run_ci: bool,
    pub ci_status: CiStatus,
    pub nix_derivation: Option<String>,
    pub review_text: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateCommit {
    pub review_status: Option<ReviewStatus>,
    pub should_run_ci: Option<bool>,
    pub ci_status: Option<CiStatus>,
    pub nix_derivation: Option<Option<String>>,
    pub review_text: Option<Option<String>>,
}

impl UpdateCommit {
    fn to_params_and_clauses(&self) -> (Vec<&(dyn ToSql + Sync)>, Vec<String>) {
        let mut set_clauses = Vec::new();
        let mut params = Vec::<&(dyn ToSql + Sync)>::new();
        let mut param_count = 1;
        if let Some(review_status) = &self.review_status {
            set_clauses.push(format!("review_status = ${}", param_count));
            params.push(review_status);
            param_count += 1;
        }

        if let Some(should_run_ci) = &self.should_run_ci {
            set_clauses.push(format!("should_run_ci = ${}", param_count));
            params.push(should_run_ci);
            param_count += 1;
        }

        if let Some(ci_status) = &self.ci_status {
            set_clauses.push(format!("ci_status = ${}", param_count));
            params.push(ci_status);
            param_count += 1;
        }

        if let Some(nix_derivation) = &self.nix_derivation {
            set_clauses.push(format!("nix_derivation = ${}", param_count));
            params.push(nix_derivation);
            param_count += 1;
        }

        if let Some(review_text) = &self.review_text {
            set_clauses.push(format!("review_text = ${}", param_count));
            params.push(review_text);
        }

        (params, set_clauses)
    }

    fn to_log_string(&self) -> String {
        use core::fmt::Write as _;

        let mut ret = String::new();
        if let Some(review_status) = &self.review_status {
            let _ = writeln!(ret, "    set review_status to {}", review_status);
        }

        if let Some(should_run_ci) = &self.should_run_ci {
            let _ = writeln!(ret, "    set should_run_ci to {}", should_run_ci);
        }

        if let Some(ci_status) = &self.ci_status {
            let _ = writeln!(ret, "    set ci_status to {}", ci_status);
        }

        if let Some(nix_derivation) = &self.nix_derivation {
            if let Some(nix_derivation) = nix_derivation {
                let _ = writeln!(ret, "    set nix_derivation to {}", nix_derivation);
            } else {
                let _ = writeln!(ret, "    set nix_derivation to NULL");
            }
        }

        if let Some(review_text) = &self.review_text {
            if let Some(review_text) = review_text {
                let _ = writeln!(ret, "    set review_text to {}", review_text);
            } else {
                let _ = writeln!(ret, "    set review_text to NULL");
            }
        }

        ret
    }
}

// Operations
impl DbCommitId {
    /// Updates a commit by its database ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails (the update or the log).
    pub async fn apply_update(
        self,
        tx: &Transaction<'_>,
        updates: &UpdateCommit,
    ) -> Result<Option<tokio_postgres::Row>, DbQueryError> {
        let ret = self.apply_update_no_log(tx, updates).await?;
        log_action(
            tx,
            EntityType::Commit,
            self.bare_i32(),
            "commit_updated",
            Some(&updates.to_log_string()),
            None,
        )
        .await?;
        Ok(ret)
    }

    /// Updates a commit by its database ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails (the update or the log).
    pub async fn apply_update_no_log(
        self,
        tx: &Transaction<'_>,
        updates: &UpdateCommit,
    ) -> Result<Option<tokio_postgres::Row>, DbQueryError> {
        let (mut params, clauses) = updates.to_params_and_clauses();
        if clauses.is_empty() {
            return Ok(None);
        }

        params.push(&self);
        let query = format!(
            r#"
            UPDATE commits SET {}
            WHERE id = ${}
            RETURNING id, repository_id, git_commit_id, jj_change_id, review_status,
                     should_run_ci, ci_status, nix_derivation, review_text, created_at
            "#,
            clauses.join(", "),
            clauses.len() + 1,
        );

        tx.inner.query_one(&query, &params)
            .await
            .map(Some)
            .map_err(|error| {
                // Would be useful to capture the params here but it's actually really
                // annoying to do (would need to put a lifetime on the error).
                DbQueryError {
                    action: "update",
                    entity_type: EntityType::Commit,
                    raw_id: Some(self.bare_i32()),
                    clauses,
                    error,
                }
            })
    }

    /// Gets the pull request associated with this commit via the `pr_commits` table.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails, if no PR is found, or if multiple PRs are found.
    pub async fn get_pull_request(
        self,
        tx: &Transaction<'_>,
    ) -> Result<super::PullRequest, DbQueryError> {
        let row = tx
            .inner
            .query_one(
                r#"
                SELECT pr.id, pr.repository_id, pr.pr_number, pr.title, pr.body, pr.author_login, 
                       pr.target_branch, pr.tip_commit_id, pr.merge_status, pr.review_status,
                       pr.priority, pr.ok_to_merge, pr.required_reviewers, pr.created_at, 
                       pr.updated_at, pr.synced_at
                FROM pull_requests pr
                JOIN pr_commits pc ON pr.id = pc.pull_request_id
                WHERE pc.commit_id = $1
                "#,
                &[&self],
            )
            .await
            .map_err(|error| DbQueryError {
                action: "get_pull_request",
                entity_type: EntityType::Commit,
                raw_id: Some(self.bare_i32()),
                clauses: vec![],
                error,
            })?;

        Ok(super::PullRequest::from_row(&row))
    }

    /// Replaces the commit ID for the commit.
    ///
    /// This should be called when a commit's description or signedness state has changed, but
    /// it has retained its jj change ID. It should **not** be called if the tree has changed
    /// or if the jj change ID has been changed; in that case you probably want to mark the
    /// commit as non-current in whatever PR you're considering, and make a new one.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails (the update or the log).
    pub async fn replace_commit_id(
        &self,
        tx: &Transaction<'_>,
        new_commit_id: &CommitId,
    ) -> Result<(), DbQueryError> {
        tx.execute(
            "UPDATE commits SET git_commit_id = $1 WHERE id = $2",
            &[&new_commit_id, self],
        )
        .await
        .map_err(|error| DbQueryError {
            action: "replace_commit_id",
            entity_type: EntityType::Commit,
            raw_id: Some(self.bare_i32()),
            clauses: vec![format!("git_commit_id = '{new_commit_id}'")],
            error,
        })?;

        log_action(
            tx,
            EntityType::Commit,
            self.bare_i32(),
            "replace_commit_id",
            Some("git_commit_id"),
            None,
        )
        .await?;

        Ok(())
    }

    /// Marks a commit as non-current for all PRs.
    ///
    /// Typically this should only be used for merge commits, since non-merge
    /// commits might be present in multiple PRs and it is unlikely that they
    /// will become out-of-date for all of them at once.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails (the update or the log).
    pub async fn mark_non_current_for_all_prs_and_stacks(
        &self,
        tx: &Transaction<'_>,
    ) -> Result<(), DbQueryError> {
        tx.execute(
            "UPDATE pr_commits SET is_current = false WHERE commit_id = $1",
            &[self],
        )
        .await
        .map_err(|error| DbQueryError {
            action: "mark_commit_not_current",
            entity_type: EntityType::Commit,
            raw_id: Some(self.bare_i32()),
            clauses: vec![],
            error,
        })?;

        // There should only be at most one stack that any given commit lives in
        tx.execute(
            "DELETE FROM stack_commits WHERE stack_id = $1 AND commit_id = $2",
            &[self],
        )
        .await
        .map_err(|error| DbQueryError {
            action: "delete_commit_from_stack",
            entity_type: EntityType::Commit,
            raw_id: Some(self.bare_i32()),
            clauses: vec![],
            error,
        })?;

        log_action(
            tx,
            EntityType::Commit,
            self.bare_i32(),
            "commit_marked_non_current",
            Some("is_current"),
            None,
        )
        .await?;

        Ok(())
    }
}

impl Commit {
    pub(crate) fn from_row(row: &tokio_postgres::Row) -> Self {
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

    /// Updates a commit.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails (the update or the log).
    pub async fn update(
        &self,
        tx: &Transaction<'_>,
        updates: &UpdateCommit,
    ) -> Result<Self, DbQueryError> {
        let ret = match self.id.apply_update_no_log(tx, updates).await? {
            Some(row) => Ok(Self::from_row(&row)),
            None => Ok(self.clone()),
        };
        log_action(
            tx,
            EntityType::Commit,
            self.id.bare_i32(),
            "commit_updated",
            Some(&format!(
                "updated commit {}\n{}",
                self.git_commit_id,
                updates.to_log_string()
            )),
            None,
        )
        .await?;
        ret
    }

}

#[derive(Debug, Clone)]
pub struct CommitToTest {
    pub id: DbCommitId,
    pub repository_id: super::DbRepositoryId,
    pub git_commit_id: CommitId,
    pub jj_change_id: ChangeId,
    pub review_status: ReviewStatus,
    pub should_run_ci: bool,
    pub ci_status: CiStatus,
    pub nix_derivation: Option<String>,
    pub prs: Vec<(super::PullRequest, CommitType)>,
}

impl CommitToTest {
    pub(crate) fn from_row(row: &tokio_postgres::Row) -> Self {
        Self {
            id: row.get("id"),
            repository_id: row.get("repository_id"),
            git_commit_id: row.get("git_commit_id"),
            jj_change_id: row.get("jj_change_id"),
            review_status: row.get("review_status"),
            should_run_ci: row.get("should_run_ci"),
            ci_status: row.get("ci_status"),
            nix_derivation: row.get("nix_derivation"),
            prs: vec![], // This will be populated by the calling code after grouping rows
        }
    }

    /// Replaces the commit ID for the commit.
    ///
    /// This should be called when a commit's description or signedness state has changed, but
    /// it has retained its jj change ID. It should **not** be called if the tree has changed
    /// or if the jj change ID has been changed; in that case you probably want to mark the
    /// commit as non-current in whatever PR you're considering, and make a new one.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails (the update or the log).
    pub async fn replace_commit_id(
        &mut self,
        tx: &Transaction<'_>,
        new_commit_id: CommitId,
    ) -> Result<(), DbQueryError> {
        self.id.replace_commit_id(tx, &new_commit_id).await?;
        self.git_commit_id = new_commit_id;
        Ok(())
    }
}

