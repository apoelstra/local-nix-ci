// SPDX-License-Identifier: GPL-3.0-or-later

use core::borrow::Borrow;
use core::{fmt, mem};
use std::collections::HashMap;

use lcilib::{Db, PrNumber};
use lcilib::db::{DbQueryError, MergeStatus};
use lcilib::db::models::{
    CiStatus, CommitToTest, DbRepositoryId, DbStackId, Repository, RepositoryError, ReviewStatus, Stack
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
    GetRepository {
        e: RepositoryError,
        id: DbRepositoryId,
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
            Self::GetRepository { id, .. } => {
                write!(f, "getting repository from database with ID {id}")
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
            Self::GetRepository { ref e, .. } => Some(e),
        }
    }
}

pub struct Stacks {
    inner: Vec<(Stack, Vec<CommitToTest>, Repository)>,
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
            let repo = Repository::get_by_id(&tx, stack.repository_id)
                .await
                .map_err(|e| Error::GetRepository { e, id: stack.repository_id })?;
            branch_map
                .entry((stack.repository_id, stack.target_branch.clone()))
                .or_default()
                .push((priority, inner.len()));

            inner.push((stack, commits, repo));
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
        let repo = Repository::get_by_id(tx.borrow(), stack.repository_id)
            .await
            .map_err(|e| Error::GetRepository { e, id: stack.repository_id })?;

        Ok(Self {
            inner: vec![(stack, commits, repo)],
            high_priority: vec![0],
            low_priority: vec![],
        })
    }

    /// Iterator over all the stacks.
    pub fn all(&self) -> impl Iterator<Item = &(Stack, Vec<CommitToTest>, Repository)> {
        self.inner.iter()
    }

    /// Iterator over high-priority stacks.
    pub fn high_priority(&self) -> impl Iterator<Item = (&Stack, &[CommitToTest])> {
        self.high_priority.iter().map(|idx| (&self.inner[*idx].0, self.inner[*idx].1.as_slice()))
    }

    /// Iterator over low-priority stacks.
    pub fn low_priority(&self) -> impl Iterator<Item = (&Stack, &[CommitToTest])> {
        self.low_priority.iter().map(|idx| (&self.inner[*idx].0, self.inner[*idx].1.as_slice()))
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
    pub async fn process(&mut self, db: &mut Db) -> Result<bool, ProcessError> {
        let mut work_done = false;
        let mut last_error = None;

        // This loop is equivalent to Vec::retain except uglier and less efficient. We need
        // to do it because retain cannot use async methods in its closure.
        for (stack, commits, repo) in mem::take(&mut self.inner) {
            let stack_id = stack.id; // stack is moved into process_single_stack
            match process_single_stack(db, stack, commits, repo).await {
                Ok(ProcessAction::Retain(stack, commits, repo)) => {
                    self.inner.push((stack, commits, repo));
                    work_done = true;
                }
                Ok(ProcessAction::RetainUnchanged(stack, commits, repo)) => {
                    self.inner.push((stack, commits, repo));
                }
                Ok(ProcessAction::Deleted) => {
                    work_done = true;
                    // no further action
                }
                Err(e) => {
                    log::warn(&e, format_args!(
                        "failed to process stack with db ID {}; ignoring stack for this cycle",
                        stack_id
                    ));
                    last_error = Some(e);
                }
            };
        }

        match last_error {
            Some(e) => Err(e),
            None => Ok(work_done),
        }
    }
}

#[derive(Debug)]
pub enum ProcessError {
    TxCreate(lcilib::tokio_postgres::Error),
    TxCommit(lcilib::tokio_postgres::Error),
    DeleteStack(DbQueryError),
    GitListParents {
        e: git::Error,
        commit_id: git::CommitId,
    },
    GitResolveMergeRef {
        e: git::Error,
        branch: String,
    },
    CheckGpgSigned {
        e: jj::Error,
        change_id: jj::ChangeId,
    },
    MarkNonCurrent {
        e: DbQueryError,
        commit_id: git::CommitId,
    },
    GetPr {
        e: DbQueryError,
        commit_id: git::CommitId,
    },
    ComputeMergeDescription {
        e: git::MergeDescriptionError,
        commit_id: git::CommitId,
        pr_number: PrNumber,
    },
    GetAckWeight {
        e: DbQueryError,
        pr_number: PrNumber,
    },
    GitResolveTreeRef {
        e: git::Error,
        commit_id: git::CommitId,
    },
    ReplaceCommitId {
        e: DbQueryError,
        old_commit_id: git::CommitId,
        new_commit_id: git::CommitId,
    },
}

impl fmt::Display for ProcessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::TxCreate(_) => f.write_str("starting database transaction"),
            Self::TxCommit(_) => f.write_str("committing database transaction"),
            Self::DeleteStack(_) => f.write_str("deleting stack from database"),
            Self::GitListParents { ref commit_id, .. } => {
                write!(f, "querying git for parents of {commit_id}")
            },
            Self::GitResolveMergeRef { ref branch, .. } => {
                write!(f, "querying git for parents of {branch}")
            },
            Self::CheckGpgSigned { ref change_id, .. } => {
                write!(f, "checking whether change {change_id} is signed")
            },
            Self::MarkNonCurrent { ref commit_id, .. } => {
                write!(f, "marking commit {commit_id} as non-current for all PRs and stacks")
            },
            Self::GetPr { ref commit_id, .. } => {
                write!(f, "getting PR from database for merge commit {commit_id}")
            },
            Self::ComputeMergeDescription { ref commit_id, ref pr_number, .. } => {
                write!(f, "computing merge description for commit {commit_id} (PR {pr_number})")
            },
            Self::GetAckWeight { ref pr_number, .. } => {
                write!(f, "getting ACK weight from database for PR {pr_number}")
            },
            Self::GitResolveTreeRef { ref commit_id, .. } => {
                write!(f, "querying git for ref {commit_id}^{{tree}}")
            },
            Self::ReplaceCommitId { ref old_commit_id, ref new_commit_id, .. } => {
                write!(f, "replacing commit ID {old_commit_id} with {new_commit_id} (same tree) in database")
            },
        }
    }
}

impl std::error::Error for ProcessError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            Self::TxCreate(ref e) => Some(e),
            Self::TxCommit(ref e) => Some(e),
            Self::DeleteStack(ref e) => Some(e),
            Self::GitListParents { ref e, .. } => Some(e),
            Self::GitResolveMergeRef { ref e, .. } => Some(e),
            Self::CheckGpgSigned { ref e, .. } => Some(e),
            Self::MarkNonCurrent { ref e, .. } => Some(e),
            Self::GetPr { ref e, .. } => Some(e),
            Self::ComputeMergeDescription { ref e, .. } => Some(e),
            Self::GetAckWeight { ref e, .. } => Some(e),
            Self::GitResolveTreeRef { ref e, .. } => Some(e),
            Self::ReplaceCommitId { ref e, .. } => Some(e),
        }
    }
}

enum ProcessAction {
    Retain(Stack, Vec<CommitToTest>, Repository),
    RetainUnchanged(Stack, Vec<CommitToTest>, Repository),
    Deleted,
}

async fn process_single_stack(
    db: &mut Db,
    stack: Stack,
    stack_commits: Vec<CommitToTest>,
    repo: Repository,
) -> Result<ProcessAction, ProcessError> {
    let tx = db.transaction().await.map_err(ProcessError::TxCreate)?;

    // 1. Delete any already-empty stacks.
    let Some(first_commit) = stack_commits.first() else {
        stack.delete(&tx).await.map_err(ProcessError::DeleteStack)?;
        tx.commit().await.map_err(ProcessError::TxCommit)?;
        return Ok(ProcessAction::Deleted);
    };

    // 2. Delete stack if its target branch doesn't match its parent.
    let parents = git::list_parents(&repo.repo_shell, &first_commit.git_commit_id)
        .await
        .map_err(|e| ProcessError::GitListParents { e, commit_id: first_commit.git_commit_id.clone() })?;

    let target_commit = git::resolve_ref(&repo.repo_shell, &stack.target_branch)
        .await
        .map_err(|e| ProcessError::GitResolveMergeRef { e, branch: stack.target_branch.clone() })?;

    let needs_rebase = parents.is_empty() || parents[0].to_string() != target_commit.to_string();

    if needs_rebase {
        log::info(format_args!(
            "Stack {} needs rebase due to target branch change",
            stack.id
        ));

        // Check for GPG signatures before rebasing
        for commit in &stack_commits {
            if is_commit_gpg_signed(&repo.repo_shell, &commit.jj_change_id)
                .await
                .map_err(|e| ProcessError::CheckGpgSigned { e, change_id: commit.jj_change_id.clone() })?
            {
                log::info(format_args!(
                    "Throwing away GPG signature for commit {} due to rebase",
                    commit.jj_change_id
                ));
            }
        }

        // Mark all commits as not current and delete stack
        for commit in &stack_commits {
            commit
                .id
                .mark_non_current_for_all_prs_and_stacks(&tx)
                .await
                .map_err(|e| ProcessError::MarkNonCurrent { e, commit_id: commit.git_commit_id.clone() })?
        }
        stack.delete(&tx).await.map_err(ProcessError::DeleteStack)?;
        tx.commit().await.map_err(ProcessError::TxCommit)?;
        // The stack will be recreated naturally by the usual approved-PR logic.
        return Ok(ProcessAction::Deleted);
    }

    // 3. We are going to retain the stack. Update descriptions and check for commit changes
    let mut work_done = false;
    let mut stack_poisoned = false;
    let mut new_stack_commits = vec![];
    for (next_idx, mut commit) in (1..).zip(stack_commits.into_iter()) {
        let pr = commit.id.get_pull_request(&tx)
            .await
            .map_err(|e| ProcessError::GetPr { e, commit_id: commit.git_commit_id.clone() })?;
        // Update description (dummy implementation for now)
        let description = git::compute_merge_description(&tx, &pr, &commit)
            .await
            .map_err(|e| ProcessError::ComputeMergeDescription {
                e,
                commit_id: commit.git_commit_id.clone(),
                pr_number: pr.pr_number,
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
                let weight = pr.get_ack_weight(&tx, repo.repo_shell.upstream())
                    .await
                    .map_err(|e| ProcessError::GetAckWeight { e, pr_number: pr.pr_number })?;
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
                "{} commit {} has been marked 'skipped' (due to update to PR {}); removing rest of stack",
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
                        .map_err(|e| ProcessError::GitResolveTreeRef { e, commit_id: current_git_id.clone() })?;
                        let original_tree = git::resolve_ref(
                            &repo.repo_shell,
                            format!("{}^{{tree}}", commit.git_commit_id),
                        )
                        .await
                        .map_err(|e| ProcessError::GitResolveTreeRef { e, commit_id: commit.git_commit_id.clone() })?;

                        if current_tree == original_tree {
                            // Tree unchanged -- just update the commit ID in place (lol)
                            commit.replace_commit_id(&tx, current_git_id.clone())
                                .await
                                .map_err(|e| ProcessError::ReplaceCommitId { e, new_commit_id: current_git_id, old_commit_id: commit.git_commit_id.clone() })?;
                            // Typically this occurs because the user signed the commit (or
                            // un-signed it). It can also happen if the commit description changes,
                            // which can happen if the set of ACKs changes. Possibly we want to log
                            // a warning or something here if this causes a signature to be thrown
                            // away, but it's a bit of a PITA to check this and also unlikely the
                            // user will see it in the scrollback.
                        } else {
                            // Tree changed - kill the rest of the stack
                            // FIXME this should be a warning and somehow have a std::Error associated to it (by refactoring functions probably)
                            log::info(format_args!(
                                "Change {} tree changed (commit {} to {}), marking as not current removing remainder of stack.",
                                commit.jj_change_id, commit.git_commit_id.clone(), current_git_id.clone(),
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
            commit.id.mark_non_current_for_all_prs_and_stacks(&tx)
                .await
                .map_err(|e| ProcessError::MarkNonCurrent { e, commit_id: commit.git_commit_id })?
        } else {
            new_stack_commits.push(commit);
        }
    }

    // 4. Do another "is the stack empty" check.
    let ret = if new_stack_commits.is_empty() {
        stack.delete(&tx).await.map_err(ProcessError::DeleteStack)?;
        ProcessAction::Deleted
    } else if work_done {
        ProcessAction::Retain(stack, new_stack_commits, repo)
    } else {
        ProcessAction::RetainUnchanged(stack, new_stack_commits, repo)
    };

    tx.commit().await.map_err(ProcessError::TxCommit)?;
    Ok(ret)
}
