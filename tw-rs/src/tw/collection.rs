// SPDX-License-Identifier: GPL-3.0-or-later

use core::fmt;
use std::borrow::Cow;
use std::collections::HashMap;
use std::io;
use std::io::BufRead as _;
use uuid::Uuid;
use xshell::{cmd, Shell};

use crate::git::{self, GitCommit};
use super::{CommitTask, PrTask};
use super::task::PrOrCommitTask;

#[derive(Debug)]
pub struct TaskCollection {
    #[allow(unused)] // FIXME remove this once we start using it
    commits: HashMap<Uuid, CommitTask>,
    pulls: HashMap<Uuid, PrTask>,
    // Map from projects and PR numbers to their UUIDs.
    pull_numbers: HashMap<(Cow<'static, str>, usize), Uuid>,
}

impl TaskCollection {
    /// Construct a [`TaskCollection`] by taking a shell with `TASKRC` set,
    /// and querying TaskWarrior for all tasks with the project set to `local-ci`.
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

    /// Query a pull request task given the project name (e.g. `apoelstra.local-nix-ci`) and PR number.
    pub fn pull_by_number(&self, project: &str, num: usize) -> Option<&PrTask> {
        self.pull_numbers.get(&(Cow::Borrowed(project), num)).and_then(|uuid| self.pulls.get(uuid))
    }

    /// Returns an iterator over all the pull requests in the database.
    pub fn pulls(&self) -> impl Iterator<Item = (&Uuid, &PrTask)> {
        self.pulls.iter()
    }

    /// Refreshes a PR's data by querying the local git repo and Github. If the PR does not
    /// yet exist, create it and add it to the database.
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
                    false,
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
        let pr_json = cmd!(task_shell, "gh pr view {num_str} --json commits,title,author,baseRefOid,headRefOid")
            .read()
            .map_err(TaskCollectionError::Shell)?;
        let pr_data: crate::gh::PrInfo = serde_json::from_str(&pr_json)
            .map_err(TaskCollectionError::ParseJson)?;
        
        // Assert that headRefOid is the last commit in the list
        if let Some(last_commit) = pr_data.commits.last() {
            if last_commit.oid != pr_data.head_commit {
                return Err(TaskCollectionError::InvalidPrData {
                    message: format!("headRefOid {} does not match last commit {}", pr_data.head_commit, last_commit.oid)
                });
            }
        } else {
            return Err(TaskCollectionError::InvalidPrData {
                message: "PR has no commits".to_string()
            });
        }
        
        // Before invoking task/jj/git fetch, check for merge commits and bail early, to make
        // an effort not to pollute the task database with crap from bad PRs.
        for commit_id in pr_data.commit_ids() {
            let parents = git::list_parents(task_shell, commit_id)
                .map_err(TaskCollectionError::Git)?;
            if parents.len() > 1 {
                return Err(TaskCollectionError::IllegalMergeCommit {
                    commit_id: commit_id.clone(),
                    pr_number: num,
                });
            }
        }
        
        // Try to fetch all commits
        for commit_id in pr_data.commit_ids() {
            git::fetch_commit(task_shell, commit_id)
                .map_err(TaskCollectionError::Git)?;
        }
        git::fetch_commit(task_shell, &pr_data.base_commit)
            .map_err(TaskCollectionError::Git)?;

        // Create merge commit directly using jj
        let head_commit = &pr_data.head_commit;
        let base_commit = &pr_data.base_commit;
        let merge_change_id = crate::jj::jj_new(task_shell, &[&head_commit, &base_commit])
            .map_err(TaskCollectionError::Jj)?;
        let merge_commit_id = crate::jj::jj_log(task_shell, "commit_id", &merge_change_id)
            .map_err(TaskCollectionError::Jj)?;
        
        // Check if the merge has conflicts
        let conflicts_check = crate::jj::jj_log(task_shell, "if(conflict,\"x\",\"\")", &merge_change_id)
            .map_err(TaskCollectionError::Jj)?;
        let has_conflicts = !conflicts_check.is_empty();

        // Add PR-specific fields to the task command, and run it.
        let description = format!("PR #{}: {}", num, pr_data.title);
        let task_cmd = task_cmd
            .arg(format!("repo_root:{}", repo.repo_root.display()))
            .arg(format!("pr_title:{}", pr_data.title))
            .arg(format!("pr_author:{}", pr_data.author.login))
            .arg(format!("pr_url:https://github.com/{}/pull/{}", repo.project_name.replace('.', "/"), num))
            .arg(format!("description:{}", description))
            .arg(format!("base_commit:{}", base_commit))
            .arg(format!("merge_change_id:{}", merge_change_id));
        let output = task_cmd.read().map_err(TaskCollectionError::Shell)?;
        
        // Create commit tasks for all commits in the PR and collect their UUIDs
        // Also build a mapping from commit_id to UUID for dependency resolution
        let mut commit_id_to_uuid = std::collections::HashMap::new();
        let mut commit_uuids = Vec::new();
        
        for commit_id in pr_data.commit_ids() {
            // Do an early check for merge commits.
            let parents = git::list_parents(task_shell, commit_id)
                .map_err(TaskCollectionError::Git)?;
            if parents.len() > 1 {
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
                if *commit_id == pr_data.commits.last().unwrap().oid {
                    let commit_uuid = commit_uuid.to_string();
                    let _ = cmd!(task_shell, "task {commit_uuid} modify +TIP_COMMIT").run();
                }
            }
        }
        
        // Now set up dependencies: each commit depends on its parent if the parent is in this PR
        for commit_id in pr_data.commit_ids() {
            if let Some(&commit_uuid) = commit_id_to_uuid.get(commit_id) {
                // Get the parent commit
                let parents = git::list_parents(task_shell, commit_id)
                    .map_err(TaskCollectionError::Git)?;

                let parent_commit_id = &parents[0];
                if let Some(&parent_uuid) = commit_id_to_uuid.get(parent_commit_id) {
                    let commit_uuid_str = commit_uuid.to_string();
                    let parent_uuid_str = parent_uuid.to_string();
                    let _ = cmd!(task_shell, "task {commit_uuid_str} modify depends:{parent_uuid_str}").run();
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
            let task_json = cmd!(task_shell, "task rc.json.array=off {pr_uuid_str} export")
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
    InvalidPrData {
        message: String,
    },
    Git(crate::git::Error),
    Jj(crate::jj::Error),
    TaskCreationFailed {
        message: String,
    },
    IllegalMergeCommit {
        commit_id: GitCommit,
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
            Self::InvalidPrData { message } => write!(f, "Invalid PR data: {}", message),
            Self::Git(_) => f.write_str("failed invoking git"),
            Self::Jj(_) => f.write_str("failed invoking jj"),
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
            Self::InvalidPrData { .. } => None,
            Self::Git(e) => Some(e),
            Self::Jj(e) => Some(e),
            Self::TaskCreationFailed { .. } => None,
            Self::IllegalMergeCommit { .. } => None,
        }
    }
}
