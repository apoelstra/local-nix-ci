// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;
use uuid::Uuid;
use serde_json;

#[derive(Debug, Clone)]
pub struct Task {
    pub uuid: Uuid,
    pub project: String,
    pub repo_dir: PathBuf,
    pub ci_status: CiStatus,
    pub data: TaskData,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReviewStatus {
    Unreviewed,
    NeedsChange,
    Nacked,
    Approved,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CiStatus {
    Unstarted,
    Started,
    Success,
    Failed,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MergeStatus {
    Unstarted,
    NeedSig,
    Pushed,
}

#[derive(Debug, Clone)]
pub enum TaskData {
    Commit {
        commit_id: String,
        /// Whether this is the merge commit for some PR.
        is_merge: bool,
        /// Whether this is the tip commit of some PR and should be tested in detail.
        is_tip: bool,
        /// List of PRs this commit appears in
        pr_refs: Vec<Uuid>,
        /// Derivation path for CI
        derivation: Option<String>,
        /// Which CI box has claimed this job
        claimed_by: Option<String>,
    },
    Pr {
        title: String,
        author: String,
        number: usize,
        url: String,
        review_status: ReviewStatus,
        review_notes: Option<String>,
        merge_status: MergeStatus,
        /// Ordered list of commit UUIDs in this PR
        commit_refs: Vec<Uuid>,
        /// UUID of the merge commit for this PR
        merge_commit_ref: Option<Uuid>,
        /// UUID of the tip commit for this PR
        tip_commit_ref: Option<Uuid>,
        /// Base commit ID for merge
        base_commit: Option<String>,
        /// JJ change ID of merge commit
        jj_change_id: Option<String>,
    },
}

#[derive(Debug)]
pub enum TaskParseError {
    TaskWarriorParse(String),
    MissingUuid,
    InvalidUuid(uuid::Error),
    MissingProject,
    MissingRepoRoot,
    InvalidRepoRoot,
    InvalidCiStatus(String),
    InvalidReviewStatus(String),
    InvalidMergeStatus(String),
    InvalidPrNumber(String),
    CommitMissingId,
    PrHasCommitId,
    UnknownTaskType,
}

impl fmt::Display for TaskParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TaskWarriorParse(msg) => write!(f, "TaskWarrior parse error: {}", msg),
            Self::MissingUuid => write!(f, "Task missing UUID"),
            Self::InvalidUuid(e) => write!(f, "Invalid UUID: {}", e),
            Self::MissingProject => write!(f, "Task missing project"),
            Self::MissingRepoRoot => write!(f, "Task missing repo_root"),
            Self::InvalidRepoRoot => write!(f, "Invalid repo_root path"),
            Self::InvalidCiStatus(s) => write!(f, "Invalid ci_status: {}", s),
            Self::InvalidReviewStatus(s) => write!(f, "Invalid review_status: {}", s),
            Self::InvalidMergeStatus(s) => write!(f, "Invalid merge_status: {}", s),
            Self::InvalidPrNumber(s) => write!(f, "Invalid PR number: {}", s),
            Self::CommitMissingId => write!(f, "Commit task missing commit_id (expected for commit tasks)"),
            Self::PrHasCommitId => write!(f, "PR task has commit_id (should be empty for PR tasks)"),
            Self::UnknownTaskType => write!(f, "Unable to determine if task is commit or PR type"),
        }
    }
}

impl std::error::Error for TaskParseError {}

impl FromStr for ReviewStatus {
    type Err = TaskParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "unreviewed" => Ok(Self::Unreviewed),
            "needschange" => Ok(Self::NeedsChange),
            "nacked" => Ok(Self::Nacked),
            "approved" => Ok(Self::Approved),
            _ => Err(TaskParseError::InvalidReviewStatus(s.to_string())),
        }
    }
}

impl FromStr for CiStatus {
    type Err = TaskParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "unstarted" => Ok(Self::Unstarted),
            "started" => Ok(Self::Started),
            "success" => Ok(Self::Success),
            "failed" => Ok(Self::Failed),
            _ => Err(TaskParseError::InvalidCiStatus(s.to_string())),
        }
    }
}

impl FromStr for MergeStatus {
    type Err = TaskParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "unstarted" => Ok(Self::Unstarted),
            "needsig" => Ok(Self::NeedSig),
            "pushed" => Ok(Self::Pushed),
            _ => Err(TaskParseError::InvalidMergeStatus(s.to_string())),
        }
    }
}

impl Task {
    pub fn from_str(task_str: &str) -> Result<Self, TaskParseError> {
        // Parse TaskWarrior JSON format
        let task_json: serde_json::Value = serde_json::from_str(task_str)
            .map_err(|e| TaskParseError::TaskWarriorParse(e.to_string()))?;

        // Extract common fields
        let uuid_str = task_json["uuid"]
            .as_str()
            .ok_or(TaskParseError::MissingUuid)?;
        let uuid = uuid_str.parse()
            .map_err(TaskParseError::InvalidUuid)?;

        let project = task_json["project"]
            .as_str()
            .ok_or(TaskParseError::MissingProject)?
            .to_string();

        let repo_root_str = task_json["repo_root"]
            .as_str()
            .ok_or(TaskParseError::MissingRepoRoot)?;
        let repo_dir = PathBuf::from(repo_root_str);
        if !repo_dir.is_absolute() {
            return Err(TaskParseError::InvalidRepoRoot);
        }

        let ci_status = task_json["ci_status"]
            .as_str()
            .unwrap_or("unstarted")
            .parse()?;

        // Determine task type based on presence of commit_id
        let commit_id = task_json["commit_id"]
            .as_str()
            .unwrap_or("");
        let has_commit_id = !commit_id.is_empty();

        let data = if has_commit_id {
            // This is a commit task
            let is_merge = task_json["base_commit"].as_str().is_some();
            
            // Check if TIP_COMMIT tag is present
            let tags = task_json["tags"].as_array().unwrap_or(&vec![]);
            let is_tip = tags.iter().any(|tag| tag.as_str() == Some("TIP_COMMIT"));

            TaskData::Commit {
                commit_id: commit_id.to_string(),
                is_merge,
                is_tip,
                pr_refs: Vec::new(), // Will be populated when building task collection
                derivation: task_json["derivation"].as_str().map(|s| s.to_string()),
                claimed_by: task_json["claimedby"].as_str().map(|s| s.to_string()),
            }
        } else {
            // This is a PR task
            let pr_number_str = task_json["pr_number"]
                .as_str()
                .ok_or(TaskParseError::UnknownTaskType)?;
            let number = pr_number_str.parse()
                .map_err(|_| TaskParseError::InvalidPrNumber(pr_number_str.to_string()))?;

            let title = task_json["pr_title"].as_str().unwrap_or("").to_string();
            let author = task_json["pr_author"].as_str().unwrap_or("").to_string();
            let url = task_json["pr_url"].as_str().unwrap_or("").to_string();

            let review_status = task_json["review_status"]
                .as_str()
                .unwrap_or("unreviewed")
                .parse()?;

            let merge_status = task_json["merge_status"]
                .as_str()
                .unwrap_or("unstarted")
                .parse()?;

            TaskData::Pr {
                title,
                author,
                number,
                url,
                review_status,
                review_notes: task_json["review_notes"].as_str().map(|s| s.to_string()),
                merge_status,
                commit_refs: Vec::new(), // Will be populated when building task collection
                merge_commit_ref: None,
                tip_commit_ref: None,
                base_commit: task_json["base_commit"].as_str().map(|s| s.to_string()),
                jj_change_id: task_json["jj_change_id"].as_str().map(|s| s.to_string()),
            }
        };

        Ok(Task {
            uuid,
            project,
            repo_dir,
            ci_status,
            data,
        })
    }

    pub fn is_commit(&self) -> bool {
        matches!(self.data, TaskData::Commit { .. })
    }

    pub fn is_pr(&self) -> bool {
        matches!(self.data, TaskData::Pr { .. })
    }
}

#[derive(Debug)]
pub struct TaskCollection {
    tasks: HashMap<Uuid, Task>,
}

impl TaskCollection {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
        }
    }

    pub fn add_task(&mut self, mut task: Task) -> Result<(), TaskCollectionError> {
        let uuid = task.uuid;
        
        // Insert the task first
        self.tasks.insert(uuid, task);
        
        // Now update cross-references
        self.update_cross_references()?;
        
        Ok(())
    }

    pub fn get_task(&self, uuid: &Uuid) -> Option<&Task> {
        self.tasks.get(uuid)
    }

    pub fn get_task_mut(&mut self, uuid: &Uuid) -> Option<&mut Task> {
        self.tasks.get_mut(uuid)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Uuid, &Task)> {
        self.tasks.iter()
    }

    fn update_cross_references(&mut self) -> Result<(), TaskCollectionError> {
        // First pass: collect all PR numbers and their UUIDs
        let mut pr_map: HashMap<usize, Uuid> = HashMap::new();
        for (uuid, task) in &self.tasks {
            if let TaskData::Pr { number, .. } = &task.data {
                pr_map.insert(*number, *uuid);
            }
        }

        // Second pass: update cross-references
        // We need to collect updates first to avoid borrowing issues
        let mut updates: Vec<(Uuid, TaskData)> = Vec::new();

        for (uuid, task) in &self.tasks {
            match &task.data {
                TaskData::Commit { .. } => {
                    // For commits, find which PRs they belong to by checking dependencies
                    // This is a simplified approach - in practice you'd need to parse
                    // TaskWarrior dependencies or use other mechanisms
                    let mut new_data = task.data.clone();
                    if let TaskData::Commit { ref mut pr_refs, .. } = new_data {
                        pr_refs.clear();
                        // TODO: Implement proper PR membership detection
                    }
                    updates.push((*uuid, new_data));
                }
                TaskData::Pr { .. } => {
                    // For PRs, find their commits by checking dependencies
                    let mut new_data = task.data.clone();
                    if let TaskData::Pr { ref mut commit_refs, ref mut merge_commit_ref, ref mut tip_commit_ref, .. } = new_data {
                        commit_refs.clear();
                        *merge_commit_ref = None;
                        *tip_commit_ref = None;
                        // TODO: Implement proper commit collection from dependencies
                    }
                    updates.push((*uuid, new_data));
                }
            }
        }

        // Apply updates
        for (uuid, new_data) in updates {
            if let Some(task) = self.tasks.get_mut(&uuid) {
                task.data = new_data;
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum TaskCollectionError {
    CrossReferenceUpdate(String),
}

impl fmt::Display for TaskCollectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CrossReferenceUpdate(msg) => write!(f, "Failed to update cross-references: {}", msg),
        }
    }
}

impl std::error::Error for TaskCollectionError {}
