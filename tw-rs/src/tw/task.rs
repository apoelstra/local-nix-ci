// SPDX-License-Identifier: GPL-3.0-or-later

use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use uuid::Uuid;
use serde_json;

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
    url: String,
    merge_status: MergeStatus,
    base_commit: Option<String>,
    merge_change_id: Option<String>,
}

impl PrTask {
    /// The UUID of the taskwarrior task backing this [`Task`].
    pub fn uuid(&self) -> &Uuid { &self.uuid }

    /// The project in the form `org.repo`. The taskwarrior project is this
    /// string prefixed by `local-ci.`.
    pub fn project(&self) -> &str { &self.project }

    /// The path to the git toplevel directory of the project.
    pub fn repo_dir(&self) -> &Path { &self.repo_dir }

    pub fn title(&self) -> &str { &self.title }

    pub fn number(&self) -> usize { self.number }

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

    commit_id: String,
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

    pub(super) fn dep_uuid(&self) -> Option<&Uuid> { self.parent_commit_uuid.as_ref() }
}

#[derive(Copy, Debug, Clone, PartialEq, Eq)]
pub enum ReviewStatus {
    Unreviewed,
    NeedsChange,
    Nacked,
    Approved,
}

#[derive(Copy, Debug, Clone, PartialEq, Eq)]
pub enum CiStatus {
    Unstarted,
    Started,
    Success,
    Failed,
}

#[derive(Copy, Debug, Clone, PartialEq, Eq)]
pub enum MergeStatus {
    Unstarted,
    NeedSig,
    Pushed,
}

#[derive(Debug)]
pub enum TaskParseError {
    TaskWarriorParse(serde_json::Error),
    MissingUuid,
    InvalidUuid(uuid::Error),
    BadTask {
        uuid: Uuid,
        error: BadTaskError,
    },
}

impl fmt::Display for TaskParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TaskWarriorParse(_) => write!(f, "failed to parse task json"),
            Self::MissingUuid => write!(f, "task missing UUID"),
            Self::InvalidUuid(_) => write!(f, "invalid UUID"),
            Self::BadTask { uuid, .. } => write!(f, "malformed local-ci task {uuid}"),
        }
    }
}

impl std::error::Error for TaskParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::TaskWarriorParse(e) => Some(e),
            Self::MissingUuid => None,
            Self::InvalidUuid(e) => Some(e),
            Self::BadTask { error, .. } => Some(error),
        }
    }
}

#[derive(Debug)]
pub enum BadTaskError {
    MissingProject,
    MissingRepoRoot,
    InvalidRepoRoot,
    InvalidCiStatus(String),
    InvalidReviewStatus(String),
    InvalidMergeStatus(String),
    InvalidPrNumber(String),
    UnknownTaskType,
    PrMissingTipCommit,
    PrMultipleDependencies,
    CommitMultipleDependencies,
    MissingDependencyUuid,
    InvalidDependencyUuid(uuid::Error),
}

impl fmt::Display for BadTaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingProject => write!(f, "Task missing project"),
            Self::MissingRepoRoot => write!(f, "Task missing repo_root"),
            Self::InvalidRepoRoot => write!(f, "Invalid repo_root path"),
            Self::InvalidCiStatus(s) => write!(f, "Invalid ci_status: {}", s),
            Self::InvalidReviewStatus(s) => write!(f, "Invalid review_status: {}", s),
            Self::InvalidMergeStatus(s) => write!(f, "Invalid merge_status: {}", s),
            Self::InvalidPrNumber(s) => write!(f, "Invalid PR number: {}", s),
            Self::UnknownTaskType => write!(f, "Unable to determine if task is commit or PR type"),
            Self::PrMissingTipCommit => write!(f, "PR task must have exactly one dependency (tip commit)"),
            Self::PrMultipleDependencies => write!(f, "PR task has multiple dependencies, expected exactly one"),
            Self::CommitMultipleDependencies => write!(f, "Commit task has multiple dependencies, expected at most one"),
            Self::MissingDependencyUuid => write!(f, "Dependency UUID string is missing or null"),
            Self::InvalidDependencyUuid(e) => write!(f, "Invalid dependency UUID: {}", e),
        }
    }
}

impl std::error::Error for BadTaskError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidDependencyUuid(e) => Some(e),
            _ => None,
        }
    }
}

impl BadTaskError {
    fn with_uuid(self, uuid: Uuid) -> TaskParseError {
        TaskParseError::BadTask {
            uuid,
            error: self,
        }
    }
}

impl FromStr for ReviewStatus {
    type Err = BadTaskError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "unreviewed" => Ok(Self::Unreviewed),
            "needschange" => Ok(Self::NeedsChange),
            "nacked" => Ok(Self::Nacked),
            "approved" => Ok(Self::Approved),
            _ => Err(BadTaskError::InvalidReviewStatus(s.to_string())),
        }
    }
}

impl FromStr for CiStatus {
    type Err = BadTaskError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "unstarted" => Ok(Self::Unstarted),
            "started" => Ok(Self::Started),
            "success" => Ok(Self::Success),
            "failed" => Ok(Self::Failed),
            _ => Err(BadTaskError::InvalidCiStatus(s.to_string())),
        }
    }
}

impl FromStr for MergeStatus {
    type Err = BadTaskError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "unstarted" => Ok(Self::Unstarted),
            "needsig" => Ok(Self::NeedSig),
            "pushed" => Ok(Self::Pushed),
            _ => Err(BadTaskError::InvalidMergeStatus(s.to_string())),
        }
    }
}

pub(super) enum PrOrCommitTask {
    Pr(PrTask),
    Commit(CommitTask),
}

impl PrOrCommitTask {
    pub fn from_json(task_str: &str) -> Result<Self, TaskParseError> {
        // Parse TaskWarrior JSON format
        let task_json: serde_json::Value = serde_json::from_str(task_str)
            .map_err(TaskParseError::TaskWarriorParse)?;

        // Extract common fields
        let uuid_str = task_json["uuid"]
            .as_str()
            .ok_or(TaskParseError::MissingUuid)?;
        let uuid = uuid_str.parse()
            .map_err(TaskParseError::InvalidUuid)?;

        let project = task_json["project"]
            .as_str()
            .ok_or(BadTaskError::MissingProject)
            .map_err(|e| e.with_uuid(uuid))?
            .to_string();

        let repo_root_str = task_json["repo_root"]
            .as_str()
            .ok_or(BadTaskError::MissingRepoRoot)
            .map_err(|e| e.with_uuid(uuid))?;
        let repo_dir = PathBuf::from(repo_root_str);
        if !repo_dir.is_absolute() {
            return Err(BadTaskError::InvalidRepoRoot.with_uuid(uuid));
        }

        let description = task_json["description"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let review_status = task_json["review_status"]
            .as_str()
            .unwrap_or("unreviewed")
            .parse::<ReviewStatus>()
            .map_err(|e| e.with_uuid(uuid))?;

        let ci_status = task_json["ci_status"]
            .as_str()
            .unwrap_or("unstarted")
            .parse::<CiStatus>()
            .map_err(|e| e.with_uuid(uuid))?;

        let depends = task_json["depends"].as_array().map(Vec::as_slice).unwrap_or(&[]);

        // Determine task type based on presence of commit_id
        let commit_id = task_json["commit_id"]
            .as_str()
            .unwrap_or("");

        if commit_id.is_empty() {
            // This is a PR task - must have exactly one dependency (tip commit)
            if depends.is_empty() {
                return Err(BadTaskError::PrMissingTipCommit.with_uuid(uuid));
            }
            if depends.len() > 1 {
                return Err(BadTaskError::PrMultipleDependencies.with_uuid(uuid));
            }

            let tip_commit_uuid_str = depends[0]
                .as_str()
                .ok_or(BadTaskError::MissingDependencyUuid)
                .map_err(|e| e.with_uuid(uuid))?;
            let tip_commit_uuid = tip_commit_uuid_str.parse()
                .map_err(BadTaskError::InvalidDependencyUuid)
                .map_err(|e| e.with_uuid(uuid))?;

            let pr_number_str = task_json["pr_number"]
                .as_str()
                .ok_or(BadTaskError::UnknownTaskType)
                .map_err(|e| e.with_uuid(uuid))?;
            let number = pr_number_str.parse()
                .map_err(|_| BadTaskError::InvalidPrNumber(pr_number_str.to_string()))
                .map_err(|e| e.with_uuid(uuid))?;

            let title = task_json["pr_title"].as_str().unwrap_or("").to_string();
            let author = task_json["pr_author"].as_str().unwrap_or("").to_string();
            let url = task_json["pr_url"].as_str().unwrap_or("").to_string();

            let merge_status = task_json["merge_status"]
                .as_str()
                .unwrap_or("unstarted")
                .parse::<MergeStatus>()
                .map_err(|e| e.with_uuid(uuid))?;

            Ok(PrOrCommitTask::Pr(PrTask {
                uuid,
                tip_commit_uuid,
                project,
                repo_dir,
                review_status,
                review_notes: task_json["review_notes"].as_str().unwrap_or("").to_string(),
                description,
                
                title,
                author,
                number,
                url,
                merge_status,
                base_commit: task_json["base_commit"].as_str().map(|s| s.to_string()),
                merge_change_id: task_json["jj_change_id"].as_str().map(|s| s.to_string()),
            }))
        } else {
            // This is a commit task - can have 0 or 1 dependencies (parent commit)
            if depends.len() > 1 {
                return Err(BadTaskError::CommitMultipleDependencies.with_uuid(uuid));
            }

            let parent_commit_uuid = if depends.is_empty() {
                None
            } else {
                let parent_uuid_str = depends[0]
                    .as_str()
                    .ok_or(BadTaskError::MissingDependencyUuid)
                    .map_err(|e| e.with_uuid(uuid))?;
                Some(parent_uuid_str.parse()
                    .map_err(BadTaskError::InvalidDependencyUuid)
                    .map_err(|e| e.with_uuid(uuid))?)
            };
            
            // Check if TIP_COMMIT tag is present
            let tags = task_json["tags"].as_array().map(Vec::as_slice).unwrap_or(&[]);
            let is_tip = tags.iter().any(|tag| tag.as_str() == Some("TIP_COMMIT"));

            Ok(PrOrCommitTask::Commit(CommitTask {
                uuid,
                parent_commit_uuid,
                
                commit_id: commit_id.to_string(),
                project,
                repo_dir,
                review_status,
                review_notes: task_json["review_notes"].as_str().unwrap_or("").to_string(),
                description,

                ci_status,
                is_tip,
                derivation: task_json["derivation"].as_str().map(|s| s.to_string()),
                claimed_by: task_json["claimedby"].as_str().map(|s| s.to_string()),
            }))
        }
    }
}

