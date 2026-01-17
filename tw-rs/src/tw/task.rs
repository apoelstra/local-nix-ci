// SPDX-License-Identifier: GPL-3.0-or-later

use std::fmt;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use serde_json;

use crate::git::GitCommit;
use super::serde_types::{self, CiStatus, MergeStatus, ReviewStatus};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrTask {
    uuid: Uuid,
    description: String,
    tip_commit_uuid: Uuid,

    project: String,
    repo_dir: PathBuf,
    review_status: ReviewStatus,
    review_notes: String,

    title: String,
    author: String,
    number: usize,
    merge_status: MergeStatus,
    base_commit: GitCommit,
    merge_uuid: Uuid,
    merge_change_id: String,
}

impl PrTask {
    /// The UUID of the taskwarrior task backing this [`Task`].
    pub fn uuid(&self) -> &Uuid { &self.uuid }

    /// The project in the form `org.repo`. The taskwarrior project is this
    /// string prefixed by `local-ci.`.
    pub fn project(&self) -> &str { &self.project }

    pub fn title(&self) -> &str { &self.title }

    pub fn number(&self) -> usize { self.number }

    pub fn commits<'tc>(
        &self,
        collection: &'tc super::TaskCollection,
    ) -> impl Iterator<Item = &'tc CommitTask> {
        core::iter::successors(
            collection.commit(&self.tip_commit_uuid),
            |comm| comm.parent_commit_uuid.as_ref().and_then(|uuid| collection.commit(uuid)),
        )
    }

    pub(super) fn dep_uuid(&self) -> &Uuid { &self.tip_commit_uuid }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitTask {
    uuid: Uuid,
    description: String,
    // For merge commits this will be the non-trunk parent.
    // If the PR itself contains merge commits it will just be rejected when
    // it's added.
    parent_commit_uuid: Option<Uuid>,

    project: String,
    repo_dir: PathBuf,
    review_status: ReviewStatus,
    review_notes: String,

    commit_id: GitCommit,
    is_tip: bool,
    ci_status: CiStatus,
    derivation: Option<String>,
    claimed_by: Option<String>,
}

impl CommitTask {
    /// The UUID of the taskwarrior task backing this [`Task`].
    pub fn uuid(&self) -> &Uuid { &self.uuid }

    /// The project in the form `org.repo`. The taskwarrior project is this
    /// string prefixed by `local-ci.`.
    pub fn project(&self) -> &str { &self.project }

    /// The path to the git toplevel directory of the project.
    pub fn repo_dir(&self) -> &Path { &self.repo_dir }

    /// The description of the task.
    pub fn description(&self) -> &str { &self.description }

    /// The commit ID.
    pub fn commit_id(&self) -> &GitCommit { &self.commit_id }

    pub(super) fn dep_uuid(&self) -> Option<&Uuid> { self.parent_commit_uuid.as_ref() }
}

#[derive(Debug)]
pub struct TaskParseError {
    uuid: Option<Uuid>,
    error: TaskParseErrorInner,
}

impl fmt::Display for TaskParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use TaskParseErrorInner as I;
        if let Some(uuid) = self.uuid {
            write!(f, "[task {}] ", uuid)?;
        }
        match &self.error {
            I::TaskWarriorParse(_) => f.write_str("failed to parse taskwarrior json"),
            I::InvalidRepoRoot => f.write_str("Invalid repo_root path"),
            I::PrMissingTipCommit => f.write_str("PR task must have exactly one dependency (tip commit)"),
            I::PrMultipleDependencies => f.write_str("PR task has multiple dependencies, expected exactly one"),
            I::CommitMultipleDependencies => f.write_str("Commit task has multiple dependencies, expected at most one"),
            I::MissingField { task_ty, field } => write!(f, "missing field {field} for {task_ty}"),
        }
    }
}

impl std::error::Error for TaskParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        use TaskParseErrorInner as I;
        match &self.error {
            I::TaskWarriorParse(e) => Some(e),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum TaskParseErrorInner {
    TaskWarriorParse(serde_json::Error),
    InvalidRepoRoot,
    PrMissingTipCommit,
    PrMultipleDependencies,
    CommitMultipleDependencies,
    MissingField { task_ty: &'static str, field: &'static str },
}

impl TaskParseErrorInner {
    fn with_uuid(self, uuid: Uuid) -> TaskParseError {
        TaskParseError {
            uuid: Some(uuid),
            error: self,
        }
    }
}

pub(super) enum PrOrCommitTask {
    Pr(PrTask),
    Commit(CommitTask),
}

impl PrOrCommitTask {
    pub fn from_json(task_str: &str) -> Result<Self, TaskParseError> {
        fn unwrap_field<T>(
            uuid: Uuid,
            task_ty: &'static str,
            field: &'static str,
            object: Option<T>,
        ) -> Result<T, TaskParseError> {
            object.ok_or(TaskParseError {
                uuid: Some(uuid),
                error: TaskParseErrorInner::MissingField { task_ty, field },
            })
        }

        #[derive(serde::Deserialize)]
        struct JustUuid {
            uuid: Uuid,
        }

        // Start by attempting to parse just the UUID. This should always
        // succeed, and lets us tag every other error with the UUID.
        let uuid = serde_json::from_str::<JustUuid>(task_str)
            .map_err(TaskParseErrorInner::TaskWarriorParse)
            .map_err(|error| TaskParseError { uuid: None, error })?
            .uuid;
        let err_with_uuid = |e: TaskParseErrorInner| e.with_uuid(uuid);
        
        // Parse the full structure.
        let task_json: serde_types::Task = serde_json::from_str(task_str)
            .map_err(TaskParseErrorInner::TaskWarriorParse)
            .map_err(err_with_uuid)?;

        if !task_json.repo_root.is_absolute() {
            return Err(TaskParseErrorInner::InvalidRepoRoot.with_uuid(uuid));
        }

        // Because Rust is annoying we have to call has_tag before
        // moving any fields out of `task_json`.
        let is_tip = task_json.has_tag("TIP_COMMIT");

        let project = task_json
            .project
            .strip_prefix("local-ci.")
            .map(str::to_owned)
            .unwrap_or(task_json.project);

        if let Some(commit_id) = task_json.commit_id {
            // This is a commit task - can have 0 or 1 dependencies (parent commit)
            if task_json.depends.len() > 1 {
                return Err(TaskParseErrorInner::CommitMultipleDependencies.with_uuid(uuid));
            }

            let parent_commit_uuid = task_json.depends.first().copied();
           
            Ok(PrOrCommitTask::Commit(CommitTask {
                uuid,
                parent_commit_uuid,
                commit_id,
                project,
                repo_dir: task_json.repo_root,
                review_status: task_json.review_status,
                review_notes: task_json.review_notes,
                description: task_json.description,

                ci_status: task_json.ci_status,
                is_tip,
                derivation: task_json.derivation,
                claimed_by: task_json.claimed_by,
            }))
        } else {
            // This is a PR task - must have exactly one dependency (tip commit)
            if task_json.depends.is_empty() {
                return Err(TaskParseErrorInner::PrMissingTipCommit.with_uuid(uuid));
            }
            if task_json.depends.len() > 1 {
                return Err(TaskParseErrorInner::PrMultipleDependencies.with_uuid(uuid));
            }
            let title = unwrap_field(uuid, "pr", "pr_title", task_json.pr_title)?;
            let author = unwrap_field(uuid, "pr", "pr_author", task_json.pr_author)?;
            let number = unwrap_field(uuid, "pr", "pr_number", task_json.pr_number)?;
            let base_commit = unwrap_field(uuid, "pr", "base_commit", task_json.base_commit)?;
            let merge_change_id = unwrap_field(uuid, "pr", "merge_change_id", task_json.merge_change_id)?;
            let merge_uuid = unwrap_field(uuid, "pr", "merge_uuid", task_json.merge_uuid)?;
            
            Ok(PrOrCommitTask::Pr(PrTask {
                uuid,
                tip_commit_uuid: task_json.depends[0],
                project,
                repo_dir: task_json.repo_root,
                review_status: task_json.review_status,
                review_notes: task_json.review_notes,
                description: task_json.description,
                
                title,
                author,
                number,
                merge_status: task_json.merge_status,
                base_commit,
                merge_change_id,
                merge_uuid,
            }))
        }
    }
}

