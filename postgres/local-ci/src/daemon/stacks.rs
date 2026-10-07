// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Context as _;

use core::borrow::Borrow;
use core::{fmt, mem};
use std::collections::HashMap;

use lcilib::Db;
use lcilib::db::{DbQueryError, MergeStatus};
use lcilib::db::models::{
    CiStatus, CommitToTest, DbRepositoryId, DbStackId, Repository, ReviewStatus, Stack
};
use lcilib::{git, jj};
use lcilib::jj::is_commit_gpg_signed;

use super::{log, util};

#[derive(Debug)]
pub enum Error {
    GetStacks(DbQueryError),
    GetCommits {
        e: DbQueryError,
        id: DbStackId,
    },
    GetSingleStack {
        e: DbQueryError,
        id: DbStackId,
    },
    Priority {
        e: util::PriorityError,
        id: DbStackId,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::GetStacks(_) => f.write_str("getting stacks from database"),
            Self::GetCommits { id, .. } => {
                write!(f, "getting commits from database for stack with ID {id}")
            },
            Self::GetSingleStack { id, .. } => {
                write!(f, "getting stack from database with ID {id}")
            },
            Self::Priority { id, .. } => {
                write!(f, "calculating priority for stack with database ID {id}")
            },
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            Self::GetStacks(ref e) => Some(e),
            Self::GetCommits { ref e, .. } => Some(e),
            Self::GetSingleStack { ref e, .. } => Some(e),
            Self::Priority { ref e, .. } => Some(e),
        }
    }
}

pub struct Stacks {
    inner: Vec<(Stack, Vec<CommitToTest>)>,
    high_priority: Vec<usize>,
    low_priority: Vec<usize>,
}

impl Stacks {
    /// Returns all the high-priority and low-priority stacks across all repos.
    pub async fn find_all(
       tx: impl Borrow<lcilib::Transaction<'_>>,
    ) -> Result<Self, Error> {
        let tx = tx.borrow();
        let mut inner = vec![];
        // Find all stacks grouped by DB and branch
        let mut branch_map =
            HashMap::<(DbRepositoryId, String), Vec<(f64, usize)>>::new();
        for stack in Stack::get_all(tx).await.map_err(Error::GetStacks)? {
            let commits = stack.id.get_commits(tx)
                .await
                .map_err(|e| Error::GetCommits { e, id: stack.id })?;
            let priority = util::calculate_stack_priority(&commits, tx)
                .await
                .map_err(|e| Error::Priority { e, id: stack.id })?;

            branch_map
                .entry((stack.repository_id, stack.target_branch.clone()))
                .or_default()
                .push((priority, inner.len()));

            inner.push((stack, commits));
        }

        let mut high_priority = vec![];
        let mut low_priority = vec![];

        for mut stacks in branch_map.into_values() {
            // priority order, so pop() takes the highest-priority stack
            stacks.sort_by(|a, b| a.0.total_cmp(&b.0));
            if let Some((_, idx)) = stacks.pop() {
                high_priority.push(idx);
            }
            low_priority.extend(stacks.into_iter().map(|(_, idx)| idx));
        }

        Ok(Self { inner, high_priority, low_priority })
    }

    /// Costructs a stack list with a single stack in it.
    pub async fn single(
       tx: impl Borrow<lcilib::Transaction<'_>>,
       id: DbStackId,
    ) -> Result<Self, Error> {
        let stack = Stack::find_by_id(tx.borrow(), id)
            .await
            .map_err(|e| Error::GetSingleStack { e, id })?;
        let commits = stack.id.get_commits(tx.borrow())
            .await
            .map_err(|e| Error::GetCommits { e, id: stack.id })?;

        Ok(Self {
            inner: vec![(stack, commits)],
            high_priority: vec![0],
            low_priority: vec![],
        })
    }

    /// Iterator over high-priority stacks.
    pub fn high_priority(&self) -> impl Iterator<Item = &(Stack, Vec<CommitToTest>)> {
        self.high_priority.iter().map(|idx| &self.inner[*idx])
    }

    /// Iterator over low-priority stacks.
    pub fn low_priority(&self) -> impl Iterator<Item = &(Stack, Vec<CommitToTest>)> {
        self.low_priority.iter().map(|idx| &self.inner[*idx])
    }

    /// Whether the "high priority" stack list is empty.
    pub fn low_priority_is_empty(&self) -> bool {
        self.high_priority.is_empty()
    }

    /// Whether the "high priority" stack list is empty.
    pub fn high_priority_is_empty(&self) -> bool {
        self.high_priority.is_empty()
    }

    /// Whether both stack lists are empty.
    pub fn is_empty(&self) -> bool {
        self.high_priority.is_empty() && self.low_priority.is_empty()
    }

    /// For all stacks, check the current repository state and update them if they've changed.
    ///
    /// Returns a boolean indicating whether any change happened.
    pub async fn process(&mut self, db: &mut Db) -> anyhow::Result<bool> {
        let mut work_done = false;

        for (stack, commits) in mem::take(&mut self.inner) {
            let id = stack.id;
            if process_stack_updates(db, stack, commits)
                .await
                .with_context(|| format!("processing updates for stack {}", id))?
            {
                work_done = true;
            }
        }

        Ok(work_done)
    }
}

async fn process_stack_updates(
    db: &mut Db,
    stack: Stack,
    mut stack_commits: Vec<CommitToTest>,
) -> anyhow::Result<bool> {
    let Some(first_commit) = stack_commits.first() else {
        db.with_transaction(async |tx| stack.delete(&tx).await)
            .await
            .context("deleting empty stack from database")?;
        return Ok(false);
    };

    // Check if first commit's first parent matches target
    let repo = db
        .with_transaction(async |tx| Repository::get_by_id(&tx, stack.repository_id).await)
        .await
        .with_context(|| format!("repository {} for {}", stack.repository_id, stack.id))?;
    let parents = git::list_parents(&repo.repo_shell, &first_commit.git_commit_id)
        .await
        .context("getting commit parents")?;

    let target_commit = git::resolve_ref(&repo.repo_shell, &stack.target_branch)
        .await
        .context("resolving target branch")?;

    let needs_rebase = parents.is_empty() || parents[0].to_string() != target_commit.to_string();

    if needs_rebase {
        log::info(format_args!(
            "Stack {} needs rebase due to target branch change",
            stack.id
        ));

        // Check for GPG signatures before rebasing
        for commit in &stack_commits {
            if is_commit_gpg_signed(&repo.repo_shell, &commit.jj_change_id).await? {
                log::info(format_args!(
                    "Throwing away GPG signature for commit {} due to rebase",
                    commit.jj_change_id
                ));
            }
        }

        // Mark all commits as not current and delete stack
        db.with_transaction(async |tx| {
            for commit in &stack_commits {
                commit
                    .id
                    .mark_non_current_for_all_prs_and_stacks(&tx)
                    .await?;
            }
            stack.delete(&tx).await
        })
        .await
        .context("deleting stack")?;

        // The stack will be recreated naturally by the usual approved-PR logic.
        return Ok(true);
    }

    // Update descriptions and check for commit changes
    let mut work_done = false;
    let mut stack_poisoned = false;
    for (next_idx, commit) in (1..).zip(stack_commits.iter_mut()) {
        let pr = db
            .with_transaction(async |tx| commit.id.get_pull_request(&tx).await)
            .await
            .with_context(|| format!("getting PR for merge commit {}", commit.git_commit_id))?;
        // Update description (dummy implementation for now)
        let description = db
            .with_transaction(async |tx| git::compute_merge_description(&tx, &pr, commit).await)
            .await
            .with_context(|| {
                format!(
                    "computing merge description for merge of PR {} (commit {})",
                    pr.pr_number, commit.git_commit_id
                )
            })?;
        if let Err(e) =
            jj::update_commit_description(&repo.repo_shell, &commit.jj_change_id, &description)
                .await
        {
            log::warn(
                &e,
                format_args!(
                    "Failed to update description for commit {}",
                    commit.jj_change_id
                ),
            );
        }

        if commit.stack_sequence_order != Some(next_idx) {
            // A commit was deleted from the stack (perhaps by the
            // user calling 'pr reset', or by an earlier attempted
            // removal that went bad).
            stack_poisoned = true;
        }

        if pr.merge_status != MergeStatus::Pending {
            log::info(format_args!(
                "{} PR {} no longer 'pending'; now {}; removing commit {} and rest of stack",
                stack.id, pr.pr_number, pr.merge_status, commit.git_commit_id,
            ));
            stack_poisoned = true;
        }

        match pr.review_status {
            ReviewStatus::Approved => {}
            ReviewStatus::Rejected => {
                log::info(format_args!(
                    "{} PR {} marked rejected; removing commit {} and rest of stack",
                    stack.id, pr.pr_number, commit.git_commit_id,
                ));
                stack_poisoned = true;
            }
            ReviewStatus::Unreviewed => {
                // Unreviewed PRs are only allowed in stacks if ack weight >= 2.0.
                let weight = db
                    .with_transaction(async |tx| {
                        pr.get_ack_weight(&tx, repo.repo_shell.upstream()).await
                    })
                    .await
                    .with_context(|| format!("getting ack weight for PR {}", pr.pr_number))?;
                if weight < 2.0 {
                    log::info(format_args!(
                        "{} PR {} unreviewed and ACK weight dropped to {:.2} (< 2.0); removing commit {} and rest of stack",
                        stack.id, pr.pr_number, weight, commit.git_commit_id,
                    ));
                    stack_poisoned = true;
                }
            }
        }

        if commit.ci_status == CiStatus::Skipped {
            log::info(format_args!(
                "{} commit {} has been marked 'skipped' (due update to PR {}); removing rest of stack",
                stack.id, commit.git_commit_id, pr.pr_number,
            ));
            stack_poisoned = true;
        }
        // FIXME what to do about ci_status Failed -- we actually want to mark the whole PR as 'needs change'
        //  I guess to prevent the commit from being recreated? Or maybe it depends on the particular stack?
        //  For now just ignore it.

        // Check if git commit ID has changed, and if so, if this was a "real" change (tree changed,
        // which should never happen, but okay, let's check) or just an accounting change (update
        // description or sign, which may happen externally).
        if !stack_poisoned {
            match jj::get_current_git_commit_for_change_id(&repo.repo_shell, &commit.jj_change_id)
                .await
            {
                Ok(current_git_id) => {
                    if current_git_id != commit.git_commit_id {
                        // Get tree hash and parents to check what changed
                        let current_tree = git::resolve_ref(
                            &repo.repo_shell,
                            format!("{}^{{tree}}", current_git_id),
                        )
                        .await
                        .context("getting current tree hash")?;
                        let original_tree = git::resolve_ref(
                            &repo.repo_shell,
                            format!("{}^{{tree}}", commit.git_commit_id),
                        )
                        .await
                        .context("getting original tree hash")?;

                        if current_tree == original_tree {
                            // Tree unchanged -- just update the commit ID in place (lol)
                            db.with_transaction(async |tx| {
                                commit.replace_commit_id(&tx, current_git_id).await
                            })
                            .await
                            .context("updating git commit ID")?;

                            // Check for lost GPG signature
                            let was_signed =
                                is_commit_gpg_signed(&repo.repo_shell, &commit.jj_change_id)
                                    .await?;
                            if was_signed {
                                let is_signed = jj::is_commit_gpg_signed(
                                    &repo.repo_shell,
                                    &commit.jj_change_id,
                                )
                                .await?;
                                if !is_signed {
                                    log::info(format_args!(
                                        "Throwing away GPG signature on change {} due to commit ID change",
                                        commit.jj_change_id
                                    ));
                                }
                            }
                        } else {
                            // Tree changed - kill the rest of the stack
                            // FIXME this should be a warning and somehow have a std::Error associated to it (by refactoring functions probably)
                            log::info(format_args!(
                                "Change {} tree changed (commit {} to {}), marking as not current removing remainder of stack.",
                                commit.jj_change_id, commit.git_commit_id, current_git_id,
                            ));
                            stack_poisoned = true;
                        }
                        work_done = true;
                    }
                }
                Err(e) => {
                    log::warn(
                        &e,
                        format_args!(
                            "Failed to get current git commit for {}",
                            commit.jj_change_id
                        ),
                    );
                }
            }
        }

        if stack_poisoned {
            // FIXME: because we are using a separate transaction for each commit we're deleting,
            // in theory some could succeed but later ones fail, leaving the stack in a bad state
            // where middle commits are missing in the database (though ofc not in the git tree).
            // We should loop through and do a single transaction here.
            db.with_transaction(async |tx| {
                commit.id.mark_non_current_for_all_prs_and_stacks(&tx).await
            })
            .await
            .context("marking commit as not current for poisoned stack")?;
        }
    }

    Ok(work_done)
}
