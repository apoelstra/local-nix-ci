// SPDX-License-Identifier: GPL-3.0-or-later

use core::fmt;
use std::borrow::Cow;
use std::collections::{HashMap, hash_map::Entry};
use std::io;
use std::io::BufRead as _;
use uuid::Uuid;
use xshell::{Shell, cmd};

use super::task::PrOrCommitTask;
use super::{CommitTask, PrTask};
use crate::gh::{get_acks_from_github, compute_merge_description, GetAcksError, MergeDescriptionError};
use crate::git::{self, GitCommit};
use crate::tw::serde_types::{CiStatus, MergeStatus, ReviewStatus};
use crate::tw::shell::{self, UniqueUuidError, get_or_insert_unique_uuid};

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
            let new_task =
                PrOrCommitTask::from_json(&json).map_err(TaskCollectionError::ParseTask)?;
            match new_task {
                PrOrCommitTask::Commit(new_task) => {
                    assert_eq!(commits.insert(*new_task.uuid(), new_task), None)
                }
                PrOrCommitTask::Pr(new_task) => {
                    assert_eq!(pulls.insert(*new_task.uuid(), new_task), None)
                }
            }
        }

        // Check that we have all the tasks we need. (We don't need to check for
        // circularity or other forms of non-DAGgedness since taskwarrior does
        // these checks for us.)
        let mut pull_numbers = HashMap::new();
        for task in pulls.values() {
            let mut next_uuid = Some(*task.dep_uuid());
            while let Some(uuid) = next_uuid {
                if let Some(commit) = commits.get_mut(&uuid) {
                    commit.prs.push(task.number());
                    next_uuid = commit.dep_uuid().copied();
                } else {
                    return Err(TaskCollectionError::MissingUuid {
                        missing: uuid,
                        needed_by: *task.uuid(),
                    });
                }
            }
            pull_numbers.insert(
                (Cow::Owned(task.project().to_owned()), task.number()),
                *task.uuid(),
            );

            // Also check merge commit.
            if let Some(commit) = commits.get_mut(task.merge_uuid()) {
                commit.prs.push(task.number());
            } else {
                return Err(TaskCollectionError::MissingUuid {
                    missing: *task.merge_uuid(),
                    needed_by: *task.uuid(),
                });
            }
        }
        // In case there are commits with no PRs, also iterate over the whole
        // commit list looking for missing UUIDs.
        for task in commits.values() {
            if let Some(uuid) = task.dep_uuid()
                && !commits.contains_key(uuid)
            {
                return Err(TaskCollectionError::MissingUuid {
                    missing: *uuid,
                    needed_by: *task.uuid(),
                });
            }
        }

        Ok(TaskCollection {
            commits,
            pulls,
            pull_numbers,
        })
    }

    /// Query a pull request task given the project name (e.g. `apoelstra.local-nix-ci`) and PR number.
    pub fn pull_by_number(&self, project: &str, num: usize) -> Option<&PrTask> {
        self.pull_numbers
            .get(&(Cow::Borrowed(project), num))
            .and_then(|uuid| self.pulls.get(uuid))
    }

    /// Returns an iterator over all the pull requests in the database.
    pub fn pulls(&self) -> impl Iterator<Item = (&Uuid, &PrTask)> {
        self.pulls.iter()
    }

    /// Returns an iterator over all commits in the database.
    pub fn commits(&self) -> impl Iterator<Item = (&Uuid, &CommitTask)> {
        self.commits.iter()
    }

    /// Looks up a commit by UUID
    pub fn commit(&self, uuid: &Uuid) -> Option<&CommitTask> {
        self.commits.get(uuid)
    }

    /// Query a commit task given the project name and commit ID.
    pub fn commit_by_id(&self, project: &str, commit_id: &GitCommit) -> Option<&CommitTask> {
        // We need to search through commits to find one with matching project and commit_id
        self.commits
            .values()
            .find(|task| task.project() == project && task.commit_id() == commit_id)
    }

    /// Creates or updates a commit task. Returns the UUID of the task (new or existing).
    /// If the task already exists, updates repo_root and description if they differ.
    pub fn insert_or_refresh_commit(
        &mut self,
        task_shell: &Shell,
        project_name: &str,
        repo_root: &std::path::Path,
        commit_id: &GitCommit,
    ) -> Result<&CommitTask, TaskCollectionError> {
        // Try to fetch the commit first
        git::fetch_commit(task_shell, commit_id).map_err(TaskCollectionError::Git)?;

        // Check if this is a merge commit and if it's a clean merge
        let parents = git::list_parents(task_shell, commit_id).map_err(TaskCollectionError::Git)?;
        let is_merge_commit = parents.len() > 1;
        let is_clean_merge = if is_merge_commit {
            // Check if 'git show' output is empty (clean merge)
            let show_output = cmd!(task_shell, "git show --format= {commit_id}")
                .read()
                .map_err(TaskCollectionError::Shell)?;
            show_output.trim().is_empty()
        } else {
            false
        };

        let mut update_fields = vec![
            format!("repo_root:{}", repo_root.display()),
            format!("description:Commit {commit_id}"),
        ];
        // Add merge commit tags if applicable
        if is_merge_commit {
            update_fields.push("+MERGE_COMMIT".to_owned());
            if is_clean_merge {
                update_fields.push("+CLEAN_MERGE".to_owned());
            }
        }

        // Check if a task already exists for this commit
        // We'll search for existing tasks with this commit_id
        let commit_uuid = get_or_insert_unique_uuid(
            task_shell,
            &[
                &format!("project:local-ci.{project_name}"),
                &format!("commit_id:{commit_id}"),
            ],
            &update_fields,
        )
        .map_err(TaskCollectionError::UniqueUuid)?;

        // Parse and (re-)store the new commit task
        let uuid_s = commit_uuid.to_string();
        let task_json = cmd!(task_shell, "task rc.json.array=off {uuid_s} export")
            .read()
            .map_err(TaskCollectionError::Shell)?;

        let commit_task = super::task::PrOrCommitTask::from_json(&task_json)
            .map_err(TaskCollectionError::ParseTask)?;

        if let super::task::PrOrCommitTask::Commit(commit_task) = commit_task {
            let entry = self.commits.entry(commit_uuid);
            Ok(entry.insert_entry(commit_task).into_mut())
        } else {
            panic!("Somehow created non-committask");
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
        let (mut task_cmd, existing_uuid) = match self
            .pull_numbers
            .get(&(Cow::Borrowed(&repo.project_name), num))
        {
            Some(uuid) => {
                let uuid_s = uuid.to_string();
                (cmd!(task_shell, "task {uuid_s} modify"), Some(*uuid))
            }
            None => {
                let project_name = &repo.project_name;
                let num = num.to_string();
                (
                    cmd!(
                        task_shell,
                        "task add rc.confirmation=off rc.verbose=new-uuid project:local-ci.{project_name} pr_number:{num}"
                    ),
                    None,
                )
            }
        };

        // Get PR data from GitHub
        let pr_json = cmd!(
            task_shell,
            "gh pr view {num_str} --json commits,title,author,headRefOid,baseRefName"
        )
        .read()
        .map_err(TaskCollectionError::Shell)?;
        let pr_data: crate::gh::PrInfo =
            serde_json::from_str(&pr_json).map_err(TaskCollectionError::ParseJson)?;

        // Assert that headRefOid is the last commit in the list
        if let Some(last_commit) = pr_data.commits.last() {
            if last_commit.oid != pr_data.head_commit {
                return Err(TaskCollectionError::InvalidPrData {
                    message: format!(
                        "headRefOid {} does not match last commit {}",
                        pr_data.head_commit, last_commit.oid
                    ),
                });
            }
        } else {
            return Err(TaskCollectionError::InvalidPrData {
                message: "PR has no commits".to_string(),
            });
        }

        // Before invoking task/jj/git fetch, check for merge commits and bail early, to make
        // an effort not to pollute the task database with crap from bad PRs.
        for commit_id in pr_data.commit_ids() {
            let parents =
                git::list_parents(task_shell, commit_id).map_err(TaskCollectionError::Git)?;
            if parents.len() > 1 {
                return Err(TaskCollectionError::IllegalMergeCommit {
                    commit_id: commit_id.clone(),
                    pr_number: num,
                });
            }
        }

        // Try to fetch all commits
        for commit_id in pr_data.commit_ids() {
            git::fetch_commit(task_shell, commit_id).map_err(TaskCollectionError::Git)?;
        }
        // Fetch the base ref, and create the merge commit, if needed.
        let base_commit = git::fetch_resolve_ref(task_shell, &pr_data.base_ref)
            .map_err(TaskCollectionError::Git)?;
        let head_commit = &pr_data.head_commit;

        let mut create_new_merge = true;
        if let Some(uuid) = existing_uuid {
            let old = &self.pulls[&uuid];
            let old_tip = self.commits[old.dep_uuid()].commit_id();
            if base_commit == *old.base_commit() && head_commit == old_tip {
                create_new_merge = false;
            }

        }

        if create_new_merge {
            let merge_change_id = crate::jj::jj_new(task_shell, &[&base_commit, head_commit])
                .map_err(TaskCollectionError::Jj)?;
            let merge_commit_id = crate::jj::jj_log(task_shell, "commit_id", &merge_change_id)
                .map_err(TaskCollectionError::Jj)?;
            let merge_commit_id = merge_commit_id
                .parse::<GitCommit>()
                .expect("if jj new succeeded, then we should have a valid commit id");

            // Create merge commit task
            let merge_commit = self.insert_or_refresh_commit(
                task_shell,
                &repo.project_name,
                &repo.repo_root,
                &merge_commit_id,
            )?;
            let merge_commit_uuid = merge_commit.uuid().to_string();

            // Add HAS_CONFLICTS tag if the merge has conflicts
            let conflicts_check =
                crate::jj::jj_log(task_shell, "if(conflict,\"x\",\"\")", &merge_change_id)
                    .map_err(TaskCollectionError::Jj)?;
            if !conflicts_check.is_empty() {
                let _ = cmd!(task_shell, "task {merge_commit_uuid} modify +HAS_CONFLICTS").quiet().run();
            }

            task_cmd = task_cmd
                .arg(format!("merge_uuid:{}", merge_commit_uuid))
                .arg("merge_status:unstarted")
                .arg(format!("merge_change_id:{}", merge_change_id));
        }

        // Add PR-specific fields to the task command, and run it.
        let description = format!("PR #{}: {}", num, pr_data.title);
        let task_cmd = task_cmd
            .arg(format!("repo_root:{}", repo.repo_root.display()))
            .arg(format!("pr_title:{}", pr_data.title))
            .arg(format!("pr_author:{}", pr_data.author.login))
            .arg(format!(
                "pr_url:https://github.com/{}/pull/{}",
                repo.project_name.replace('.', "/"),
                num
            ))
            .arg(format!("description:{}", description))
            .arg(format!("base_commit:{}", base_commit))
            .arg(format!("base_ref:{}", pr_data.base_ref));
        let output = task_cmd.read().map_err(TaskCollectionError::Shell)?;

        // Create commit tasks for all commits in the PR and collect their UUIDs
        // Also build a mapping from commit_id to UUID for dependency resolution
        let mut commit_id_to_uuid = std::collections::HashMap::new();
        let mut commit_uuids = Vec::new();

        for commit_id in pr_data.commit_ids() {
            let commit = self.insert_or_refresh_commit(
                task_shell,
                &repo.project_name,
                &repo.repo_root,
                commit_id,
            )?;

            commit_uuids.push(*commit.uuid());
            commit_id_to_uuid.insert(commit_id.clone(), *commit.uuid());

            // Mark the last commit as TIP_COMMIT
            if *commit_id == pr_data.commits.last().unwrap().oid {
                let commit_uuid = commit.uuid().to_string();
                let _ = cmd!(task_shell, "task {commit_uuid} modify +TIP_COMMIT").quiet().run();
            }
        }

        // Now set up dependencies: each commit depends on its parent if the parent is in this PR
        for commit_id in pr_data.commit_ids() {
            if let Some(&commit_uuid) = commit_id_to_uuid.get(commit_id) {
                // Get the parent commit
                let parents =
                    git::list_parents(task_shell, commit_id).map_err(TaskCollectionError::Git)?;

                let parent_commit_id = &parents[0];
                if let Some(&parent_uuid) = commit_id_to_uuid.get(parent_commit_id) {
                    let commit_uuid_str = commit_uuid.to_string();
                    let parent_uuid_str = parent_uuid.to_string();
                    let _ = cmd!(
                        task_shell,
                        "task {commit_uuid_str} modify depends:{parent_uuid_str}"
                    )
                    .quiet()
                    .run();
                }
            }
        }

        // Obtain the PR's UUID, either from the output of `task add` or from our database,
        // and do the final modifications to hook up the and PR commits. We do this separately
        // with 'task modify' because the 'depends:' key is weird.
        let pr_uuid = match existing_uuid {
            Some(uuid) => uuid,
            None => {
                // Extract UUID from the output
                let idx = match output.find("Created task ") {
                    Some(idx) => idx + 13,
                    None => panic!("Did not find 'Created task' in output of task add: {output}"),
                };
                let pr_uuid_str = &output[idx..idx + 36];
                Uuid::try_parse(pr_uuid_str).map_err(TaskCollectionError::ParseUuid)?
            }
        };
        let pr_uuid_str = pr_uuid.to_string();

        // Add dependency to the PR task for the tip commit
        let _ = cmd!(task_shell, "task {pr_uuid_str} modify depends:").quiet().run(); // Clear dependencies
        let tip_commit_uuid_str = commit_uuids.last().expect("checked above").to_string();
        let _ = cmd!(
            task_shell,
            "task {pr_uuid_str} modify depends:{tip_commit_uuid_str}"
        )
        .quiet()
        .run();

        // (Re-)insert the PR UUID into our PR number lookup table
        self.pull_numbers
            .insert((Cow::Owned(repo.project_name.clone()), num), pr_uuid);

        // (Re-)load the task from the Taskwarrior and put it in our map.
        let task_json = cmd!(task_shell, "task rc.json.array=off {pr_uuid_str} export")
            .read()
            .map_err(TaskCollectionError::Shell)?;

        let pr_task = super::task::PrOrCommitTask::from_json(&task_json)
            .map_err(TaskCollectionError::ParseTask)?;

        if let super::task::PrOrCommitTask::Pr(pr_task) = pr_task {
            // Count ACKs and update merge description.
            let merge_change_id = pr_task.merge_change_id().to_owned();
            let acks = get_acks_from_github(task_shell, num, head_commit)
                .map_err(TaskCollectionError::GetAcks)?;
            let description = compute_merge_description(
                task_shell,
                &pr_task,
                pr_task.tip_commit(self).commit_id(),
                &merge_change_id,
                &acks,
            ).map_err(TaskCollectionError::MergeDescription)?;

            // Add to map
            self.pulls.insert(pr_uuid, pr_task);

            // Update the description (has no effect on the task database and we don't
            // even keep track of this; it will invalidate the merge_commit_id but this
            // is fine; we only keep track of that commit up to its parents (see above
            // logic for deciding when to recreate it).
            if let Err(e) = cmd!(task_shell, "jj describe --quiet -r {merge_change_id} -m {description}").quiet().run() {
                eprintln!(
                    "Warning: Failed to update description for PR #{}: {}. If you need this updated, try running 'refresh' again.", 
                    num, e,
                );
            }

            // Check if PR is ready for merge after inserting/updating
            let _ = self.check_and_update_pr_merge_readiness(&pr_uuid);
            
            Ok(&self.pulls[&pr_uuid])
        } else {
            panic!("Somehow created non-PR task");
        }
    }

    pub fn update_commit_local_ci_commit_id(
        &mut self,
        uuid: &uuid::Uuid,
        commit_id: String
    ) -> Result<(), UpdateError> {
        let sh = crate::tw::task_shell()
            .map_err(UpdateError::CreateShell)?;

        let mut entry = match self.commits.entry(*uuid) {
            Entry::Vacant(_) => return Err(UpdateError::UnknownUuid(*uuid)),
            Entry::Occupied(hole) => hole,
        };

        let uuid_str = uuid.to_string();
        cmd!(sh, "task {uuid_str} modify ci_git_commit:{commit_id}")
            .quiet()
            .run()
            .map_err(|e| UpdateError::ExecuteModify(*uuid, e))?;

        entry.get_mut().local_ci_commit_id = Some(commit_id);
        Ok(())
    }

    pub fn update_commit_derivation(
        &mut self,
        uuid: &uuid::Uuid,
        derivation: String
    ) -> Result<(), UpdateError> {
        let sh = crate::tw::task_shell()
            .map_err(UpdateError::CreateShell)?;

        let mut entry = match self.commits.entry(*uuid) {
            Entry::Vacant(_) => return Err(UpdateError::UnknownUuid(*uuid)),
            Entry::Occupied(hole) => hole,
        };

        let uuid_str = uuid.to_string();
        cmd!(sh, "task {uuid_str} modify derivation:{derivation}")
            .quiet()
            .run()
            .map_err(|e| UpdateError::ExecuteModify(*uuid, e))?;

        entry.get_mut().derivation = Some(derivation);
        Ok(())
    }

    pub fn update_commit_ci_status(
        &mut self,
        uuid: &uuid::Uuid,
        status: CiStatus
    ) -> Result<(), UpdateError> {
        let sh = crate::tw::task_shell()
            .map_err(UpdateError::CreateShell)?;

        let mut entry = match self.commits.entry(*uuid) {
            Entry::Vacant(_) => return Err(UpdateError::UnknownUuid(*uuid)),
            Entry::Occupied(hole) => hole,
        };

        let uuid_str = uuid.to_string();
        let status_str = match status {
            CiStatus::Unstarted => "unstarted",
            CiStatus::Started => "started", 
            CiStatus::Success => "success",
            CiStatus::Failed => "failed",
        };

        cmd!(sh, "task {uuid_str} modify ci_status:{status_str}")
            .quiet()
            .run()
            .map_err(|e| UpdateError::ExecuteModify(*uuid, e))?;

        entry.get_mut().ci_status = status;
        Ok(())
    }

    pub fn update_pr_merge_status(
        &mut self,
        uuid: &uuid::Uuid,
        status: MergeStatus
    ) -> Result<(), UpdateError> {
        let sh = crate::tw::task_shell()
            .map_err(UpdateError::CreateShell)?;

        let mut entry = match self.pulls.entry(*uuid) {
            Entry::Vacant(_) => return Err(UpdateError::UnknownUuid(*uuid)),
            Entry::Occupied(hole) => hole,
        };

        let uuid_str = uuid.to_string();
        let status_str = match status {
            MergeStatus::Unstarted => "unstarted",
            MergeStatus::NeedSig => "needsig", 
            MergeStatus::Pushed => "pushed",
        };

        cmd!(sh, "task {uuid_str} modify merge_status:{status_str}")
            .quiet()
            .run()
            .map_err(|e| UpdateError::ExecuteModify(*uuid, e))?;

        entry.get_mut().merge_status = status;
        Ok(())
    }

    pub fn update_commit_review_status(
        &mut self,
        uuid: &uuid::Uuid,
        status: ReviewStatus,
        review_notes: String,
    ) -> Result<(), UpdateError> {
        let sh = crate::tw::task_shell()
            .map_err(UpdateError::CreateShell)?;

        let mut entry = match self.commits.entry(*uuid) {
            Entry::Vacant(_) => return Err(UpdateError::UnknownUuid(*uuid)),
            Entry::Occupied(hole) => hole,
        };

        let uuid_str = uuid.to_string();
        let status_str = match status {
            ReviewStatus::Approved => "approved",
            ReviewStatus::Nacked => "nacked",
            ReviewStatus::NeedsChange => "needschange",
            ReviewStatus::Unreviewed => "unreviewed",
        };

        cmd!(sh, "task {uuid_str} modify review_status:{status_str} review_notes:{review_notes}")
            .quiet()
            .run()
            .map_err(|e| UpdateError::ExecuteModify(*uuid, e))?;

        let commit = entry.get_mut();
        commit.review_status = status;
        commit.review_notes = review_notes;
        Ok(())
    }

    pub fn update_pr_review_status(
        &mut self,
        uuid: &uuid::Uuid,
        status: ReviewStatus,
        review_notes: String,
    ) -> Result<(), UpdateError> {
        let sh = crate::tw::task_shell()
            .map_err(UpdateError::CreateShell)?;

        let mut entry = match self.pulls.entry(*uuid) {
            Entry::Vacant(_) => return Err(UpdateError::UnknownUuid(*uuid)),
            Entry::Occupied(hole) => hole,
        };

        let uuid_str = uuid.to_string();
        let status_str = match status {
            ReviewStatus::Approved => "approved",
            ReviewStatus::Nacked => "nacked",
            ReviewStatus::NeedsChange => "needschange",
            ReviewStatus::Unreviewed => "unreviewed",
        };

        cmd!(sh, "task {uuid_str} modify review_status:{status_str} review_notes:{review_notes}")
            .quiet()
            .run()
            .map_err(|e| UpdateError::ExecuteModify(*uuid, e))?;

        let pr = entry.get_mut();
        pr.review_status = status;
        pr.review_notes = review_notes;
        Ok(())
    }

    /// Checks if a PR is ready for merge and updates its status accordingly.
    /// Also auto-approves clean merge commits if all PR commits are approved.
    /// Returns true if the PR status was changed to needsig.
    pub fn check_and_update_pr_merge_readiness(
        &mut self,
        pr_uuid: &uuid::Uuid,
    ) -> Result<bool, UpdateError> {
        let pr_task = match self.pulls.get(pr_uuid) {
            Some(pr) => pr,
            None => return Err(UpdateError::UnknownUuid(*pr_uuid)),
        };

        // Check if PR is pushed -- then don't do anything.
        if *pr_task.merge_status() == MergeStatus::Pushed {
            return Ok(false);
        }

        // Check if PR itself is approved
        if *pr_task.review_status() != ReviewStatus::Approved {
            return Ok(false);
        }

        // Check if all commits are approved and CI successful
        let mut all_commits_ready = true;
        for commit in pr_task.commits(self) {
            // Skip merge commits for this check
            if commit.is_merge_commit() {
                continue;
            }
            if *commit.review_status() != ReviewStatus::Approved
                || *commit.ci_status() != CiStatus::Success
            {
                all_commits_ready = false;
                break;
            }
        }

        // Get the merge commit
        let pr_task = pr_task.clone(); // un-borrow self.pulls
        let mut merge_commit = self.commit(pr_task.merge_uuid())
            .expect("merge commit should exist")
            .clone();

        // Auto-approve clean merge commit if all commits are approved
        if merge_commit.is_clean_merge()
            && *merge_commit.review_status() == ReviewStatus::Unreviewed
        {
            let auto_review_notes = format!("Auto-approved clean merge commit for PR #{}", pr_task.number());
            self.update_commit_review_status(
                merge_commit.uuid(),
                ReviewStatus::Approved,
                auto_review_notes,
            )?;
            // Important: reload the merge commit after updating the task collection!
            merge_commit = self.commit(pr_task.merge_uuid())
                .expect("merge commit should exist")
                .clone();
        }

        // Check if merge commit is approved and CI successful
        if !all_commits_ready
            || *merge_commit.review_status() != ReviewStatus::Approved
            || *merge_commit.ci_status() != CiStatus::Success
        {
            return Ok(false);
        }

        // All conditions met - update to needsig if not already
        if *pr_task.merge_status() != MergeStatus::NeedSig {
            self.update_pr_merge_status(pr_uuid, MergeStatus::NeedSig)?;
            return Ok(true);
        }

        Ok(false)
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
    GetAcks(GetAcksError),
    MergeDescription(MergeDescriptionError),
    UniqueUuid(UniqueUuidError),
    Shell(xshell::Error),
    Utf8(io::Error),
    InvalidPrData {
        message: String,
    },
    Git(crate::git::Error),
    Jj(crate::jj::Error),
    IllegalMergeCommit {
        commit_id: GitCommit,
        pr_number: usize,
    },
}

impl fmt::Display for TaskCollectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingUuid { missing, needed_by } => {
                write!(
                    f,
                    "Missing task UUID {} needed by task {}",
                    missing, needed_by
                )
            }
            Self::ParseJson(_) => write!(f, "failed to parse json"),
            Self::ParseTask(_) => write!(f, "failed to parse task"),
            Self::ParseUuid(_) => write!(f, "failed to parse uuid"),
            Self::GetAcks(_) => write!(f, "failed to get ACKs from Github"),
            Self::MergeDescription(_) => write!(f, "failed to compute description for merge commit"),
            Self::UniqueUuid(_) => write!(f, "no unique UUID for filter"),
            Self::Shell(_) => write!(f, "shell command failed"),
            Self::Utf8(_) => write!(f, "UTF-8 encoding error"),
            Self::InvalidPrData { message } => write!(f, "Invalid PR data: {}", message),
            Self::Git(_) => f.write_str("failed invoking git"),
            Self::Jj(_) => f.write_str("failed invoking jj"),
            Self::IllegalMergeCommit {
                commit_id,
                pr_number,
            } => write!(f, "Illegal merge commit {} in PR #{}", commit_id, pr_number),
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
            Self::GetAcks(e) => Some(e),
            Self::MergeDescription(e) => Some(e),
            Self::UniqueUuid(e) => Some(e),
            Self::Shell(e) => Some(e),
            Self::Utf8(e) => Some(e),
            Self::InvalidPrData { .. } => None,
            Self::Git(e) => Some(e),
            Self::Jj(e) => Some(e),
            Self::IllegalMergeCommit { .. } => None,
        }
    }
}

#[derive(Debug)]
pub enum UpdateError {
    CreateShell(shell::Error),
    UnknownUuid(Uuid),
    ExecuteModify(Uuid, xshell::Error),
}

impl std::error::Error for UpdateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CreateShell(e) => Some(e),
            Self::UnknownUuid(_) => None,
            Self::ExecuteModify(_, e) => Some(e),
        }
    }
}

impl fmt::Display for UpdateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CreateShell(_) => f.write_str("failed to create shell"),
            Self::UnknownUuid(uuid) => write!(f, "unknown UUID {uuid}"),
            Self::ExecuteModify(uuid, _) => write!(f, "failed to update UUID {uuid}"),
        }
    }
}
