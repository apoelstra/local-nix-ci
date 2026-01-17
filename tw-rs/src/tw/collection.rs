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

    /// Looks up a commit by UUID
    pub fn commit(&self, uuid: &Uuid) -> Option<&CommitTask> {
        self.commits.get(uuid)
    }

    /// Query a commit task given the project name and commit ID.
    pub fn commit_by_id(&self, project: &str, commit_id: &GitCommit) -> Option<&CommitTask> {
        // We need to search through commits to find one with matching project and commit_id
        self.commits.values().find(|task| {
            task.project() == project && task.commit_id() == commit_id
        })
    }

    /// Creates or updates a commit task. Returns the UUID of the task (new or existing).
    /// If the task already exists, updates repo_root and description if they differ.
    pub fn insert_or_refresh_commit(
        &mut self,
        task_shell: &Shell,
        project_name: &str,
        repo_root: &std::path::Path,
        commit_id: &GitCommit,
    ) -> Result<Uuid, TaskCollectionError> {
        // Try to fetch the commit first
        git::fetch_commit(task_shell, commit_id)
            .map_err(TaskCollectionError::Git)?;

        let description = format!("Commit {}", commit_id);
        
        // Check if a task already exists for this commit
        // We'll search for existing tasks with this commit_id
        let search_output = cmd!(task_shell, "task rc.json.array=off project:local-ci.{project_name} commit_id:{commit_id} export")
            .read()
            .map_err(TaskCollectionError::Shell)?;

        if search_output.trim().is_empty() {
            // No existing task, create a new one
            let commit_task_cmd = cmd!(
                task_shell,
                "task add rc.confirmation=off rc.verbose=new-uuid project:local-ci.{project_name} commit_id:{commit_id} repo_root:{repo_root} description:{description}"
            );
            let commit_output = commit_task_cmd.read().map_err(TaskCollectionError::Shell)?;
            
            // Extract UUID from commit task creation
            let idx = match commit_output.find("Created task ") {
                Some(idx) => idx + 13,
                None => return Err(TaskCollectionError::TaskCreationFailed {
                    message: "Failed to extract UUID from commit task creation".to_string()
                }),
            };
            
            let commit_uuid = Uuid::try_parse(&commit_output[idx..idx + 36])
                .map_err(TaskCollectionError::ParseUuid)?;
            
            // Parse and store the new commit task
            let uuid_s = commit_uuid.to_string();
            let task_json = cmd!(task_shell, "task rc.json.array=off {uuid_s} export")
                .read()
                .map_err(TaskCollectionError::Shell)?;
            
            let commit_task = super::task::PrOrCommitTask::from_json(&task_json)
                .map_err(TaskCollectionError::ParseTask)?;
            
            if let super::task::PrOrCommitTask::Commit(commit_task) = commit_task {
                self.commits.insert(commit_uuid, commit_task);
            }
            
            Ok(commit_uuid)
        } else {
            // Task exists, parse it and check if we need to update repo_root or description
            let existing_task = super::task::PrOrCommitTask::from_json(&search_output)
                .map_err(TaskCollectionError::ParseTask)?;
            
            if let super::task::PrOrCommitTask::Commit(existing_task) = existing_task {
                let uuid = *existing_task.uuid();
                let uuid_str = uuid.to_string();
                let mut needs_update = false;
                
                // Check if repo_root differs
                if existing_task.repo_dir() != repo_root {
                    cmd!(task_shell, "task {uuid_str} modify repo_root:{repo_root}")
                        .run()
                        .map_err(TaskCollectionError::Shell)?;
                    needs_update = true;
                }
                
                // Check if description differs
                if existing_task.description() != &description {
                    cmd!(task_shell, "task {uuid_str} modify description:{description}")
                        .run()
                        .map_err(TaskCollectionError::Shell)?;
                    needs_update = true;
                }
                
                // If we updated anything, reload the task
                if needs_update {
                    let task_json = cmd!(task_shell, "task rc.json.array=off {uuid_str} export")
                        .read()
                        .map_err(TaskCollectionError::Shell)?;
                    
                    let updated_task = super::task::PrOrCommitTask::from_json(&task_json)
                        .map_err(TaskCollectionError::ParseTask)?;
                    
                    if let super::task::PrOrCommitTask::Commit(updated_task) = updated_task {
                        self.commits.insert(uuid, updated_task);
                    }
                }
                
                Ok(uuid)
            } else {
                Err(TaskCollectionError::TaskCreationFailed {
                    message: "Found task is not a commit task".to_string()
                })
            }
        }
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
        let merge_commit_id = merge_commit_id.parse::<GitCommit>()
            .expect("if jj new succeeded, then we should have a valid commit id");
        
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
            let commit_uuid = self.insert_or_refresh_commit(
                task_shell,
                &repo.project_name,
                &repo.repo_root,
                commit_id,
            )?;
            
            commit_uuids.push(commit_uuid);
            commit_id_to_uuid.insert(commit_id.clone(), commit_uuid);
            
            // Mark the last commit as TIP_COMMIT
            if *commit_id == pr_data.commits.last().unwrap().oid {
                let commit_uuid = commit_uuid.to_string();
                let _ = cmd!(task_shell, "task {commit_uuid} modify +TIP_COMMIT").run();
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
        
        // Create merge commit task
        let merge_commit_uuid = self.insert_or_refresh_commit(
            task_shell,
            &repo.project_name,
            &repo.repo_root,
            &merge_commit_id,
        )?;
        let merge_commit_uuid = merge_commit_uuid.to_string();
        
        // Add HAS_CONFLICTS tag if the merge has conflicts
        if has_conflicts {
            let _ = cmd!(task_shell, "task {merge_commit_uuid} modify +HAS_CONFLICTS").run();
        }

        // Obtain the PR's UUID, either from the output of `task add` or from our database,
        // and do the final modifications to hook up the merge commit and PR commits.
        let pr_uuid = if new_uuid {
            // Extract UUID from the output
            let idx = match output.find("Created task ") {
                Some(idx) => idx + 13,
                None => panic!("Did not find 'Created task' in output of task add: {output}"),
            };
            let pr_uuid_str = &output[idx..idx + 36];
            Uuid::try_parse(pr_uuid_str)
                .map_err(TaskCollectionError::ParseUuid)?
        } else {
            *self.pull_numbers.get(&(Cow::Borrowed(&repo.project_name), num))
                .expect("PR was in main lookup table but not num/project name lookup tabel")
        };
        let pr_uuid_str = pr_uuid.to_string();
            
        // Add dependency to the PR task for the tip commit
        let _ = cmd!(task_shell, "task {pr_uuid_str} modify depends:").run(); // Clear dependencies
        let tip_commit_uuid = commit_uuids.last().expect("checked above");
        let _ = cmd!(task_shell, "task {pr_uuid_str} modify depends:{tip_commit_uuid_str}").run();
        // Add UDA to the PR task for the merge commit
        let _ = cmd!(task_shell, "task {pr_uuid_str} modify merge_uuid:{merge_commit_uuid}").run();
            
        // (Re-)insert the PR UUID into our PR number lookup table
        self.pull_numbers.insert(
            (Cow::Owned(repo.project_name.clone()), num),
            pr_uuid,
        );
            
        // (Re-)load the task from the Taskwarrior and put it in our map.
        let task_json = cmd!(task_shell, "task rc.json.array=off {pr_uuid_str} export")
            .read()
            .map_err(TaskCollectionError::Shell)?;
            
        let pr_task = super::task::PrOrCommitTask::from_json(&task_json)
            .map_err(TaskCollectionError::ParseTask)?;
            
        if let super::task::PrOrCommitTask::Pr(pr_task) = pr_task {
            self.pulls.insert(pr_uuid, pr_task);
            return Ok(self.pulls.get(&pr_uuid).unwrap());
        } else {
            panic!("Somehow created non-PR task");
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
