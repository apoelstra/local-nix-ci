// SPDX-License-Identifier: GPL-3.0-or-later

use core::fmt;
use std::borrow::Cow;
use std::collections::{HashMap, hash_map::Entry};
use std::io;
use std::io::BufRead as _;
use uuid::Uuid;
use xshell::{cmd, Shell};

use super::{CommitTask, PrTask};
use super::task::PrOrCommitTask;

#[derive(Debug)]
pub struct TaskCollection {
    commits: HashMap<Uuid, CommitTask>,
    pulls: HashMap<Uuid, PrTask>,
    // Map from projects and PR numbers to their UUIDs.
    pull_numbers: HashMap<(Cow<'static, str>, usize), Uuid>,
}

impl TaskCollection {
    pub fn new(task_shell: &Shell) -> Result<Self, TaskCollectionError> {
        let mut pulls = HashMap::new();
        let mut commits = HashMap::new();

        let output = cmd!(task_shell, "task rc.json.array=off project:local-ci export")
            .output()
            .map_err(TaskCollectionError::Shell)?;

        for json in output.stdout.lines() {
            let json = json.map_err(TaskCollectionError::Utf8)?;
            let new_task = PrOrCommitTask::from_json(&json)
                .map_err(TaskCollectionError::ParseTask)?;
            match new_task {
                PrOrCommitTask::Commit(new_task) => {
                    assert_eq!(commits.insert(*new_task.uuid(), new_task), None)
                },
                PrOrCommitTask::Pr(new_task) => {
                    assert_eq!(pulls.insert(*new_task.uuid(), new_task), None)
                },
            }
        }

        // Check that we have all the tasks we need. (We don't need to check for
        // circularity or other forms of non-DAGgedness since taskwarrior does
        // these checks for us.)
        let mut pull_numbers = HashMap::new();
        for task in pulls.values() {
            let uuid = task.dep_uuid();
            if !commits.contains_key(uuid) {
                return Err(TaskCollectionError::MissingUuid {
                    missing: *uuid,
                    needed_by: *task.uuid(),
                });
            }
            pull_numbers.insert(
                (Cow::Owned(task.project().to_owned()), task.number()),
                *task.uuid(),
            );
        }
        for task in commits.values() {
            if let Some(uuid) = task.dep_uuid() {
                if !commits.contains_key(uuid) {
                    return Err(TaskCollectionError::MissingUuid {
                        missing: *uuid,
                        needed_by: *task.uuid(),
                    });
                }
            }
        }
                
        Ok(TaskCollection { commits, pulls, pull_numbers })
    }

    pub fn pull_by_number(&self, project: &str, num: usize) -> Option<&PrTask> {
        self.pull_numbers.get(&(Cow::Borrowed(project), num)).and_then(|uuid| self.pulls.get(uuid))
    }

    pub fn commits(&self) -> impl Iterator<Item = (&Uuid, &CommitTask)> {
        self.commits.iter()
    }

    pub fn pulls(&self) -> impl Iterator<Item = (&Uuid, &PrTask)> {
        self.pulls.iter()
    }

    pub fn insert_or_refresh_pr(
        &mut self,
        task_shell: &Shell,
        repo: &crate::repo::Repository,
        num: usize,
    ) -> Result<&PrTask, TaskCollectionError> {
        // cmd! macro needs strings or paths, so we gotta stringify a couple things.
        // Also, move into repo.repo_root just in case we're not already.
        let _guard = task_shell.push_dir(&repo.repo_root);
        let num_str = num.to_string();

        // Create 'task add' or 'task modify' command as appropriate.
        let (task_cmd, new_uuid) = match self.pull_numbers.get(&(Cow::Borrowed(&repo.project_name), num)) {
            Some(uuid) => {
                let uuid = uuid.to_string();
                (
                    cmd!(task_shell, "task {uuid} modify"),
                    true,
                )
            }
            None => {
                let project_name = &repo.project_name;
                let num = num.to_string();
                (
                    cmd!(task_shell, "task add rc.confirmation=off rc.verbose=new-uuid project:local-ci.{project_name} pr_number:{num}"),
                    true,
                )
            }
        };

        // Get PR data from GitHub
        let pr_json = cmd!(task_shell, "gh pr view {num_str} --json commits,title,author,baseRefName,baseRefOid,headRefOid")
            .read()
            .map_err(TaskCollectionError::Shell)?;
        
        let pr_data: serde_json::Value = serde_json::from_str(&pr_json)
            .map_err(TaskCollectionError::ParseJson)?;
        let title = pr_data["title"].as_str().unwrap_or("").to_string();
        let author = pr_data["author"]["login"].as_str().unwrap_or("").to_string();
        let base_ref_oid = pr_data["baseRefOid"].as_str()
            .ok_or(TaskCollectionError::MissingPrField { field: "baseRefOid" })?;
        let head_ref_oid = pr_data["headRefOid"].as_str()
            .ok_or(TaskCollectionError::MissingPrField { field: "headRefOid" })?;
        
        // Get commits from the PR
        let commits: Vec<String> = pr_data["commits"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|c| c["oid"].as_str().map(|s| s.to_string()))
            .collect();
        
        // Assert that headRefOid is the last commit in the list
        if let Some(last_commit) = commits.last() {
            if last_commit != head_ref_oid {
                return Err(TaskCollectionError::InvalidPrData {
                    message: format!("headRefOid {} does not match last commit {}", head_ref_oid, last_commit)
                });
            }
        } else {
            return Err(TaskCollectionError::InvalidPrData {
                message: "PR has no commits".to_string()
            });
        }
        
        // Check that none of the commits in the PR are merge commits (have multiple parents)
        for commit_id in &commits {
            let parents_output = cmd!(task_shell, "git rev-list --parents -n 1 {commit_id}")
                .read()
                .map_err(TaskCollectionError::Shell)?;
            let parents: Vec<&str> = parents_output.trim().split_whitespace().collect();
            
            // First element is the commit itself, rest are parents
            if parents.len() > 2 {
                return Err(TaskCollectionError::IllegalMergeCommit {
                    commit_id: commit_id.clone(),
                    pr_number: num,
                });
            }
        }
        
        // Fetch all commits from remotes
        // First try to fetch all commits from origin
        let mut fetch_successful = true;
        // Try to fetch all commits individually
        for commit_id in &commits {
            let mut fetched = false;
            for remote in &["origin", "upstream"] {
                let fetch_result = cmd!(task_shell, "git fetch -q {remote} {commit_id}")
                    .run();
                if fetch_result.is_ok() {
                    fetched = true;
                    break;
                }
            }
            fetch_successful &= fetched;
        }
            
        // Also fetch the base commit
        {
            let mut fetched = false;
            for remote in &["origin", "upstream"] {
                let base_fetch_result = cmd!(task_shell, "git fetch -q {remote} {base_ref_oid}")
                    .run();
                if base_fetch_result.is_ok() {
                    fetched = true;
                    break;
                }
            }
            fetch_successful &= fetched;
        }
        
        if !fetch_successful {
            return Err(TaskCollectionError::FetchFailed {
                pr_number: num,
                message: "Failed to fetch commits from both origin and upstream remotes".to_string()
            });
        }
        
        // Create merge commit directly using jj
        let mut merge_change_id = String::new();
        let base_commit = base_ref_oid.to_string();
        
        // Create merge commit using jj with the fetched commit OIDs
        let jj_new_output = cmd!(task_shell, "jj --config signing.behavior=drop --color never new --no-edit -r {base_ref_oid} -r {head_ref_oid}")
            .read_stderr()
            .map_err(|e| TaskCollectionError::MergeCommitCreationFailed {
                pr_number: num,
                message: format!("jj new command failed: {}", e)
            })?;
        
        // Parse the output to extract the change ID
        // Look for patterns like "Created new commit" followed by change ID
        for line in dbg!(jj_new_output).lines() {
            if line.contains("Created new commit") {
                // Extract change ID from the line - jj change IDs use letters 'k' through 'z'
                if let Some(change_id_match) = line.split_whitespace()
                    .find(|word| word.len() >= 8 && dbg!(word).chars().all(|c| c >= 'k' && c <= 'z')) {
                    merge_change_id = change_id_match.to_string();
                    break;
                }
            }
        }
        
        // If we couldn't parse from output, fall back to jj log (racy but should work)
        if merge_change_id.is_empty() {
            let log_output = cmd!(task_shell, "jj log --no-pager --no-graph -T change_id -r")
                .arg(&format!("latest({head_ref_oid}+ & {base_ref_oid}+)")) // unsure how to escape this for cmd!
                .read()
                .map_err(|e| TaskCollectionError::JjCommandFailed {
                    pr_number: num,
                    message: format!("jj log command failed: {}", e)
                })?;
            merge_change_id = log_output.trim()[..12.min(log_output.trim().len())].to_string();
        }
        
        if merge_change_id.is_empty() {
            return Err(TaskCollectionError::JjCommandFailed {
                pr_number: num,
                message: "Failed to extract change ID from jj output".to_string()
            });
        }
        
        // Get the commit ID for this change
        let commit_output = cmd!(task_shell, "jj log --no-graph -r {merge_change_id} -T commit_id")
            .read()
            .map_err(|e| TaskCollectionError::JjCommandFailed {
                pr_number: num,
                message: format!("Failed to get commit ID for change {}: {}", merge_change_id, e)
            })?;
        let merge_commit_id = commit_output.trim().to_string();
        
        // Check if the merge has conflicts
        let conflicts_check = cmd!(task_shell, "jj log --quiet -r")
            .arg(&format!("{merge_change_id} & ~conflicts()"))
            .run();
        
        let has_conflicts = conflicts_check.is_err();
        
        let description = format!("PR #{}: {}", num, title);

        // Add PR-specific fields to the task command
        let task_cmd = task_cmd
            .arg(format!("repo_root:{}", repo.repo_root.display()))
            .arg(format!("pr_title:{}", title))
            .arg(format!("pr_author:{}", author))
            .arg(format!("pr_url:https://github.com/{}/pull/{}", repo.project_name.replace('.', "/"), num))
            .arg(format!("description:{}", description));
        
        // Add merge commit data if available
        let task_cmd = if !base_commit.is_empty() {
            task_cmd.arg(format!("base_commit:{}", base_commit))
        } else {
            task_cmd
        };
        
        let task_cmd = if !merge_change_id.is_empty() {
            task_cmd.arg(format!("merge_change_id:{}", merge_change_id))
        } else {
            task_cmd
        };
        
        // Run the command
        let output = task_cmd.read().map_err(TaskCollectionError::Shell)?;
        
        // Create commit tasks for all commits in the PR and collect their UUIDs
        // Also build a mapping from commit_id to UUID for dependency resolution
        let mut commit_id_to_uuid = std::collections::HashMap::new();
        let mut commit_uuids = Vec::new();
        
        for commit_id in &commits {
            // Check for merge commits (multiple parents)
            let parents_output = cmd!(task_shell, "git rev-list --parents -n 1 {commit_id}")
                .read()
                .map_err(TaskCollectionError::Shell)?;
            let parents: Vec<&str> = parents_output.trim().split_whitespace().collect();
            
            // First element is the commit itself, rest are parents
            if parents.len() > 2 {
                return Err(TaskCollectionError::IllegalMergeCommit {
                    commit_id: commit_id.clone(),
                    pr_number: num,
                });
            }
            
            let project_name = &repo.project_name;
            let repo_root = &repo.repo_root;
            let commit_task_cmd = cmd!(
                task_shell,
                "task add rc.confirmation=off rc.verbose=new-uuid project:local-ci.{project_name} commit_id:{commit_id} repo_root:{repo_root} description:Commit {commit_id}"
            );
            let commit_output = commit_task_cmd.read().map_err(TaskCollectionError::Shell)?;
            
            // Extract UUID from commit task creation
            let idx = match commit_output.find("Created task ") {
                Some(idx) => idx + 13,
                None => continue, // Skip if we can't find the UUID
            };
            
            if let Ok(commit_uuid) = Uuid::try_parse(&commit_output[idx..idx + 36]) {
                commit_uuids.push(commit_uuid);
                commit_id_to_uuid.insert(commit_id.clone(), commit_uuid);
                
                // Mark the last commit as TIP_COMMIT
                if commit_id == commits.last().unwrap() {
                    let commit_uuid = commit_uuid.to_string();
                    let _ = cmd!(task_shell, "task {commit_uuid} modify +TIP_COMMIT").run();
                }
            }
        }
        
        // Now set up dependencies: each commit depends on its parent if the parent is in this PR
        for commit_id in &commits {
            if let Some(&commit_uuid) = commit_id_to_uuid.get(dbg!(commit_id)) {
                // Get the parent commit
                let parents_output = cmd!(task_shell, "git rev-list --parents -n 1 {commit_id}")
                    .read()
                    .map_err(TaskCollectionError::Shell)?;
                let parents: Vec<&str> = parents_output.trim().split_whitespace().collect();
                
                // If there's exactly one parent and it's in our PR, set up the dependency
                if parents.len() == 2 {
                    let parent_commit_id = parents[1];
                    if let Some(&parent_uuid) = commit_id_to_uuid.get(parent_commit_id) {
                        let commit_uuid_str = commit_uuid.to_string();
                        let parent_uuid_str = parent_uuid.to_string();
                        let _ = cmd!(task_shell, "task {commit_uuid_str} modify depends:{parent_uuid_str}").run();
                    }
                }
            }
        }
        
        // Create merge commit task if we have merge commit data
        if !merge_commit_id.is_empty() {
            let project_name = &repo.project_name;
            let repo_root = &repo.repo_root;
            let num = num.to_string();
            let mut merge_task_cmd = cmd!(
                task_shell,
                "task add rc.confirmation=off rc.verbose=new-uuid project:local-ci.{project_name} commit_id:{merge_commit_id} repo_root:{repo_root} description:Merge commit {merge_commit_id} for PR #{num} +MERGE_COMMIT"
            );
            
            // Add HAS_CONFLICTS tag if the merge has conflicts
            if has_conflicts {
                merge_task_cmd = merge_task_cmd.arg("+HAS_CONFLICTS");
            }
            
            let merge_output = merge_task_cmd.read().map_err(TaskCollectionError::Shell)?;
            
            // Extract UUID from merge task creation
            let idx = match merge_output.find("Created task ") {
                Some(idx) => idx + 13,
                None => return Err(TaskCollectionError::TaskCreationFailed {
                    message: "Failed to extract UUID from merge task creation".to_string()
                }),
            };
            
            if let Ok(merge_uuid) = Uuid::try_parse(&merge_output[idx..idx + 36]) {
                commit_uuids.push(merge_uuid);
            }
        }

        // If we created a new task add it to our database.
        if new_uuid {
            // Extract UUID from the output
            let idx = match output.find("Created task ") {
                Some(idx) => idx + 13,
                None => panic!("Did not find 'Created task' in output of task add: {output}"),
            };
            let pr_uuid_str = &output[idx..idx + 36];
            let pr_uuid = Uuid::try_parse(pr_uuid_str)
                .map_err(TaskCollectionError::ParseUuid)?;
            
            // Add dependency to the PR task for only the tip commit
            if let Some(tip_commit_uuid) = commit_uuids.last() {
                let pr_uuid_str = pr_uuid.to_string();
                let tip_commit_uuid_str = tip_commit_uuid.to_string();
                let _ = cmd!(task_shell, "task {pr_uuid_str} modify depends:{tip_commit_uuid_str}").run();
            }
            
            // Insert the PR UUID into our lookup table
            self.pull_numbers.insert(
                (Cow::Owned(repo.project_name.clone()), num),
                pr_uuid,
            );
            
            // Create a new PrTask and insert it into our database
            // We need to reload the task data to get the complete task information
            let task_json = cmd!(task_shell, "task {pr_uuid_str} export")
                .read()
                .map_err(TaskCollectionError::Shell)?;
            
            let pr_task = super::task::PrOrCommitTask::from_json(&task_json)
                .map_err(TaskCollectionError::ParseTask)?;
            
            if let super::task::PrOrCommitTask::Pr(pr_task) = pr_task {
                self.pulls.insert(pr_uuid, pr_task);
                return Ok(self.pulls.get(&pr_uuid).unwrap());
            } else {
                return Err(TaskCollectionError::TaskCreationFailed {
                    message: "Created task is not a PR task".to_string()
                });
            }
        } else {
            // Update existing task - we need to get the UUID from the lookup table
            let pr_uuid = *self.pull_numbers.get(&(Cow::Borrowed(&repo.project_name), num))
                .ok_or(TaskCollectionError::InvalidPrData {
                    message: "PR UUID not found in lookup table".to_string()
                })?;
            let pr_uuid_str = pr_uuid.to_string();
            
            // Update dependencies - clear old ones and set only the tip commit
            let _ = cmd!(task_shell, "task {pr_uuid_str} modify depends:").run(); // Clear dependencies
            if let Some(tip_commit_uuid) = commit_uuids.last() {
                let tip_commit_uuid_str = tip_commit_uuid.to_string();
                let _ = cmd!(task_shell, "task {pr_uuid_str} modify depends:{tip_commit_uuid_str}").run();
            }
            
            // Reload the updated task data
            let task_json = cmd!(task_shell, "task {pr_uuid_str} export")
                .read()
                .map_err(TaskCollectionError::Shell)?;
            
            let pr_task = super::task::PrOrCommitTask::from_json(&task_json)
                .map_err(TaskCollectionError::ParseTask)?;
            
            if let super::task::PrOrCommitTask::Pr(pr_task) = pr_task {
                self.pulls.insert(pr_uuid, pr_task);
                return Ok(self.pulls.get(&pr_uuid).unwrap());
            } else {
                return Err(TaskCollectionError::TaskCreationFailed {
                    message: "Updated task is not a PR task".to_string()
                });
            }
        }

    }
}

#[derive(Debug)]
pub enum TaskCollectionError {
    MissingUuid {
        missing: Uuid,
        needed_by: Uuid,
    },
    ParseJson(serde_json::Error),
    ParseTask(super::TaskParseError),
    ParseUuid(uuid::Error),
    Shell(xshell::Error),
    Utf8(io::Error),
    MissingPrField {
        field: &'static str,
    },
    InvalidPrData {
        message: String,
    },
    FetchFailed {
        pr_number: usize,
        message: String,
    },
    MergeCommitCreationFailed {
        pr_number: usize,
        message: String,
    },
    JjCommandFailed {
        pr_number: usize,
        message: String,
    },
    TaskCreationFailed {
        message: String,
    },
    IllegalMergeCommit {
        commit_id: String,
        pr_number: usize,
    },
}

impl fmt::Display for TaskCollectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingUuid { missing, needed_by } => {
                write!(f, "Missing task UUID {} needed by task {}", missing, needed_by)
            }
            Self::ParseJson(_) => write!(f, "failed to parse json"),
            Self::ParseTask(_) => write!(f, "failed to parse task"),
            Self::ParseUuid(_) => write!(f, "failed to parse uuid"),
            Self::Shell(_) => write!(f, "shell command failed"),
            Self::Utf8(_) => write!(f, "UTF-8 encoding error"),
            Self::MissingPrField { field } => write!(f, "Missing required PR field: {}", field),
            Self::InvalidPrData { message } => write!(f, "Invalid PR data: {}", message),
            Self::FetchFailed { pr_number, message } => write!(f, "Failed to fetch commits for PR #{}: {}", pr_number, message),
            Self::MergeCommitCreationFailed { pr_number, message } => write!(f, "Failed to create merge commit for PR #{}: {}", pr_number, message),
            Self::JjCommandFailed { pr_number, message } => write!(f, "JJ command failed for PR #{}: {}", pr_number, message),
            Self::TaskCreationFailed { message } => write!(f, "Failed to create task: {}", message),
            Self::IllegalMergeCommit { commit_id, pr_number } => write!(f, "Illegal merge commit {} in PR #{}", commit_id, pr_number),
        }
    }
}

impl std::error::Error for TaskCollectionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::MissingUuid { .. } => None,
            Self::ParseJson(e) => Some(e),
            Self::ParseTask(e) => Some(e),
            Self::ParseUuid(e) => Some(e),
            Self::Shell(e) => Some(e),
            Self::Utf8(e) => Some(e),
            Self::MissingPrField { .. } => None,
            Self::InvalidPrData { .. } => None,
            Self::FetchFailed { .. } => None,
            Self::MergeCommitCreationFailed { .. } => None,
            Self::JjCommandFailed { .. } => None,
            Self::TaskCreationFailed { .. } => None,
            Self::IllegalMergeCommit { .. } => None,
        }
    }
}
