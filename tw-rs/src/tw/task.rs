// SPDX-License-Identifier: GPL-3.0-or-later

use core::fmt;
use serde_json;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use super::serde_types::{self, CiStatus, MergeStatus, ReviewStatus};
use crate::git::GitCommit;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrTask {
    uuid: Uuid,
    description: String,
    tip_commit_uuid: Uuid,

    project: String,
    repo_root: PathBuf,
    review_status: ReviewStatus,
    review_notes: String,

    title: String,
    author: String,
    number: usize,
    pub(super) merge_status: MergeStatus,
    base_commit: GitCommit,
    base_ref: String,
    merge_uuid: Uuid,
    merge_change_id: String,
}

impl PrTask {
    /// The UUID of the taskwarrior task backing this [`Task`].
    pub fn uuid(&self) -> &Uuid {
        &self.uuid
    }

    /// The project in the form `org.repo`. The taskwarrior project is this
    /// string prefixed by `local-ci.`.
    pub fn project(&self) -> &str {
        &self.project
    }

    pub fn repo_root(&self) -> &Path {
        &self.repo_root
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn number(&self) -> usize {
        self.number
    }

    pub fn base_commit(&self) -> &GitCommit {
        &self.base_commit
    }

    pub fn base_ref(&self) -> &str {
        &self.base_ref
    }

    pub fn author(&self) -> &str {
        &self.author
    }

    pub fn review_status(&self) -> &ReviewStatus {
        &self.review_status
    }

    pub fn review_notes(&self) -> &str {
        &self.review_notes
    }

    pub fn merge_status(&self) -> &MergeStatus {
        &self.merge_status
    }

    pub fn merge_change_id(&self) -> &str {
        &self.merge_change_id
    }

    pub fn merge_commit<'tc>(&self, collection: &'tc super::TaskCollection) -> &'tc CommitTask {
        collection
            .commit(&self.merge_uuid)
            .expect("merge UUID in collection")
    }

    pub fn commits<'tc>(
        &self,
        collection: &'tc super::TaskCollection,
    ) -> impl Iterator<Item = &'tc CommitTask> {
        core::iter::successors(collection.commit(&self.tip_commit_uuid), |comm| {
            comm.parent_commit_uuid
                .as_ref()
                .and_then(|uuid| collection.commit(uuid))
        })
    }

    pub(super) fn dep_uuid(&self) -> &Uuid {
        &self.tip_commit_uuid
    }
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
    repo_root: PathBuf,
    review_status: ReviewStatus,
    review_notes: String,

    commit_id: GitCommit,
    is_tip: bool,
    is_merge_commit: bool,
    is_clean_merge: bool,
    pub(super) ci_status: CiStatus,
    pub(super) local_ci_commit_id: Option<String>,
    pub(super) derivation: Option<String>,
    claimed_by: Option<String>,
}

impl CommitTask {
    /// The UUID of the taskwarrior task backing this [`Task`].
    pub fn uuid(&self) -> &Uuid {
        &self.uuid
    }

    /// The project in the form `org.repo`. The taskwarrior project is this
    /// string prefixed by `local-ci.`.
    pub fn project(&self) -> &str {
        &self.project
    }

    pub fn repo_root(&self) -> &Path {
        &self.repo_root
    }

    /// The description of the task.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// The commit ID.
    pub fn commit_id(&self) -> &GitCommit {
        &self.commit_id
    }

    pub fn review_status(&self) -> &ReviewStatus {
        &self.review_status
    }

    pub fn review_notes(&self) -> &str {
        &self.review_notes
    }

    pub fn ci_status(&self) -> &CiStatus {
        &self.ci_status
    }

    pub fn is_tip(&self) -> bool {
        self.is_tip
    }

    pub fn is_merge_commit(&self) -> bool {
        self.is_merge_commit
    }

    pub fn is_clean_merge(&self) -> bool {
        self.is_clean_merge
    }

    pub(super) fn dep_uuid(&self) -> Option<&Uuid> {
        self.parent_commit_uuid.as_ref()
    }
}

#[derive(Debug)]
pub struct TaskParseError {
    uuid: Option<Uuid>,
    json: Option<String>,
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
            I::PrMissingTipCommit => {
                f.write_str("PR task must have exactly one dependency (tip commit)")
            }
            I::PrMultipleDependencies => {
                f.write_str("PR task has multiple dependencies, expected exactly one")
            }
            I::CommitMultipleDependencies => {
                f.write_str("Commit task has multiple dependencies, expected at most one")
            }
            I::MissingField { task_ty, field } => write!(f, "missing field {field} for {task_ty}"),
        }?;
        if let Some(ref json) = self.json {
            write!(f, "\nJSON: {}", json)?;
        }
        Ok(())
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
    PrMissingTipCommit,
    PrMultipleDependencies,
    CommitMultipleDependencies,
    MissingField {
        task_ty: &'static str,
        field: &'static str,
    },
}

impl TaskParseErrorInner {
    fn with_uuid(self, uuid: Uuid, json: String) -> TaskParseError {
        TaskParseError {
            uuid: Some(uuid),
            json: Some(json),
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
            json_str: &str,
            task_ty: &'static str,
            field: &'static str,
            object: Option<T>,
        ) -> Result<T, TaskParseError> {
            object.ok_or_else(|| TaskParseError {
                uuid: Some(uuid),
                json: Some(json_str.to_owned()),
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
            .map_err(|error| TaskParseError {
                uuid: None,
                json: None,
                error,
            })?
            .uuid;
        let err_with_uuid = |e: TaskParseErrorInner| e.with_uuid(uuid, task_str.to_owned());

        // Parse the full structure.
        let task_json: serde_types::Task = serde_json::from_str(task_str)
            .map_err(TaskParseErrorInner::TaskWarriorParse)
            .map_err(err_with_uuid)?;

        // Because Rust is annoying we have to call has_tag before
        // moving any fields out of `task_json`.
        let is_tip = task_json.has_tag("TIP_COMMIT");
        let is_merge_commit = task_json.has_tag("MERGE_COMMIT");
        let is_clean_merge = task_json.has_tag("CLEAN_MERGE");

        let project = task_json
            .project
            .strip_prefix("local-ci.")
            .map(str::to_owned)
            .unwrap_or(task_json.project);

        if let Some(commit_id) = task_json.commit_id {
            // This is a commit task - can have 0 or 1 dependencies (parent commit)
            if task_json.depends.len() > 1 {
                return Err(TaskParseErrorInner::CommitMultipleDependencies
                    .with_uuid(uuid, task_str.to_owned()));
            }

            let parent_commit_uuid = task_json.depends.first().copied();

            Ok(PrOrCommitTask::Commit(CommitTask {
                uuid,
                parent_commit_uuid,
                commit_id,
                project,
                repo_root: task_json.repo_root,
                review_status: task_json.review_status,
                review_notes: task_json.review_notes,
                description: task_json.description,

                ci_status: task_json.ci_status,
                is_tip,
                is_merge_commit,
                is_clean_merge,
                local_ci_commit_id: task_json.local_ci_commit_id,
                derivation: task_json.derivation,
                claimed_by: task_json.claimed_by,
            }))
        } else {
            // This is a PR task - must have exactly one dependency (tip commit)
            if task_json.depends.is_empty() {
                return Err(
                    TaskParseErrorInner::PrMissingTipCommit.with_uuid(uuid, task_str.to_owned())
                );
            }
            if task_json.depends.len() > 1 {
                return Err(TaskParseErrorInner::PrMultipleDependencies
                    .with_uuid(uuid, task_str.to_owned()));
            }
            let title = unwrap_field(uuid, task_str, "pr", "pr_title", task_json.pr_title)?;
            let author = unwrap_field(uuid, task_str, "pr", "pr_author", task_json.pr_author)?;
            let number = unwrap_field(uuid, task_str, "pr", "pr_number", task_json.pr_number)?;
            let base_commit =
                unwrap_field(uuid, task_str, "pr", "base_commit", task_json.base_commit)?;
            let merge_change_id = unwrap_field(
                uuid,
                task_str,
                "pr",
                "merge_change_id",
                task_json.merge_change_id,
            )?;
            let merge_uuid =
                unwrap_field(uuid, task_str, "pr", "merge_uuid", task_json.merge_uuid)?;
            let base_ref =
                unwrap_field(uuid, task_str, "pr", "base_ref", task_json.base_ref)?;

            Ok(PrOrCommitTask::Pr(PrTask {
                uuid,
                tip_commit_uuid: task_json.depends[0],
                project,
                repo_root: task_json.repo_root,
                review_status: task_json.review_status,
                review_notes: task_json.review_notes,
                description: task_json.description,

                title,
                author,
                number,
                merge_status: task_json.merge_status,
                base_commit,
                base_ref,
                merge_change_id,
                merge_uuid,
            }))
        }
    }
}
