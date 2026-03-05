// SPDX-License-Identifier: GPL-3.0-or-later

use core::mem;
use std::collections::{HashMap, HashSet, VecDeque};

use crate::tw::{CommitTask, TaskCollection};
use crate::{CiStatus, MergeStatus, ReviewStatus};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum State {
    /// A task has been approved but not started
    Approved,
    /// The task has not been approved, so we won't test it, but we will test its downstream
    /// commits if there is nothing else to do. 
    Unapproved,
    /// The task was nacked or marked "needs change" and we won't test it or its downstream
    /// commits.
    Rejected,
    /// The task has run through CI and has failed
    Failed,
    /// The task has run through CI and succeeded
    Success,
}
    
/// A task in the run queue
#[derive(Clone, PartialEq, Eq, Debug)]
struct Task {
    task: CommitTask,
    state: State,
}

pub struct RunQueue {
    /// Map of all UUIDs tracked by the task manager.
    map: HashMap<uuid::Uuid, Task>,
    /// Map from each failed/rejected UUID to the set of dependent UUIDs which are
    /// blocked by this one.
    blocked_map: HashMap<uuid::Uuid, Vec<uuid::Uuid>>,
    /// Queue of ordinary commits; these should be tested before anything else.
    pr_commit_queue: VecDeque<uuid::Uuid>,
    /// Queue of commits which have parents that are not yet approved; if there is
    /// nothing else to do we should run through these.
    unapproved_parent_queue: VecDeque<uuid::Uuid>,
    /// Set of merge commits. We should do one of these if:all the commits in the corresponding
    /// PR have passed CI. If there are any conflicting merge commits marked "need signature",
    /// we should count the ACKs on that one and do that many commits from pr_commit_queue
    /// before doing this. This will give the user plenty of time to sign it.
    merge_commit_queue: HashSet<uuid::Uuid>,
    /// Counter of PR commits done since last merge commit
    pr_commits_since_merge: usize,
}

impl RunQueue {
    /// Get the best eligible merge commit, sorted by ACK count (highest first).
    /// If ignore_conflicts is true, ignore the conflict check and return any ready merge commit.
    fn get_best_eligible_merge_commit(
        &self,
        collection: &TaskCollection,
        ignore_conflicts: bool,
    ) -> Option<uuid::Uuid> {
        let mut eligible_merges = Vec::new();

        for &merge_uuid in &self.merge_commit_queue {
            // Find the PR that contains this merge commit
            let pr_task = collection.pulls()
                .find(|(_, pr)| pr.merge_uuid() == &merge_uuid)
                .map(|(_, pr)| pr);
            
            if let Some(pr_task) = pr_task {
                // Check if all commits in the PR have passed CI (State::Success)
                let all_commits_passed = pr_task.commits(collection)
                    .all(|commit| {
                        self.map.get(commit.uuid())
                            .map(|task| task.state == State::Success)
                            .unwrap_or(false)
                    });
                
                if all_commits_passed {
                    if ignore_conflicts {
                        // Add to eligible list regardless of conflicts
                        eligible_merges.push((merge_uuid, pr_task.ack_count()));
                    } else {
                        // Check for conflicting merge commits with CiStatus::Success
                        let has_conflicting_success = collection.pulls()
                            .any(|(_, other_pr)| {
                                // Same project and same base commit, but different PR
                                other_pr.project() == pr_task.project() &&
                                other_pr.base_ref() == pr_task.base_ref() &&
                                *other_pr.merge_status() == MergeStatus::NeedSig &&
                                other_pr.merge_uuid() != pr_task.merge_uuid() &&
                                // Check if the other merge commit has CiStatus::Success
                                self.map.get(other_pr.merge_uuid())
                                    .map(|task| task.state == State::Success)
                                    .unwrap_or(false)
                            });
                        
                        if !has_conflicting_success {
                            eligible_merges.push((merge_uuid, pr_task.ack_count()));
                        }
                    }
                }
            }
        }

        // Sort by ACK count (highest first) and return the best one
        eligible_merges.sort_by(|a, b| b.1.cmp(&a.1));
        eligible_merges.first().map(|(uuid, _)| *uuid)
    }

    pub fn status(&self) {
        // PR Commit Queue
        println!("PR Commit Queue ({} commits):", self.pr_commit_queue.len());
        for &uuid in &self.pr_commit_queue {
            let task = self.map.get(&uuid).unwrap();
            let pr_display = format_pr_numbers(&task.task.prs());
            println!("  {}{}", pr_display, task.task.commit_id());
        }
        println!();

        // Merge Commit Queue
        println!("Merge Commit Set ({} commits):", self.merge_commit_queue.len());
        for &uuid in &self.merge_commit_queue {
            let task = self.map.get(&uuid).unwrap();
            let pr_display = format_pr_numbers(&task.task.prs());
            println!("  {}{}", pr_display, task.task.commit_id());
        }
        println!();

        // Unapproved Parent Queue
        if !self.unapproved_parent_queue.is_empty() {
            println!("Unapproved Parent Queue ({} commits):", self.unapproved_parent_queue.len());
            for &uuid in &self.unapproved_parent_queue {
                let task = self.map.get(&uuid).unwrap();
                let pr_display = format_pr_numbers(&task.task.prs());
                println!("  {}{} (waiting for parent approval)", pr_display, task.task.commit_id());
            }
            println!();
        }

        // Blocked Commits
        if !self.blocked_map.is_empty() {
            println!("Blocked Commits:");
            for (&blocking_uuid, blocked_uuids) in &self.blocked_map {
                let blocking_task = self.map.get(&blocking_uuid).unwrap();
                let blocking_pr_display = format_pr_numbers(&blocking_task.task.prs());
                let blocking_state = match blocking_task.state {
                    State::Failed => "[Failed]",
                    State::Rejected => "[Rejected]",
                    _ => "[Unknown]",
                };
                
                for &blocked_uuid in blocked_uuids {
                    let blocked_task = self.map.get(&blocked_uuid).unwrap();
                    let blocked_pr_display = format_pr_numbers(&blocked_task.task.prs());
                    println!("  {}{} {} blocked by {}{} {}", 
                        blocked_pr_display, blocked_task.task.commit_id(), blocked_task.task.description(),
                        blocking_pr_display, blocking_task.task.commit_id(), blocking_state);
                }
            }
            println!();
        }
    }

    pub fn refresh_merge_commits(
        &mut self,
        logger: &super::log::Logger,
        task_shell: &crate::Shell,
        collection: &mut TaskCollection,
        
    ) {
        // First, refresh all the merge commits
        let mut refreshed_merge_commits = HashSet::with_capacity(self.merge_commit_queue.len());
        for merge_uuid in mem::take(&mut self.merge_commit_queue) {
            match collection.refresh_merge_commit(task_shell, &merge_uuid) {
                Ok(new_uuids) => {
                    if new_uuids.is_empty() {
                        refreshed_merge_commits.insert(merge_uuid);

                    }
                    refreshed_merge_commits.extend(new_uuids);
                }
                Err(e) => {
                    logger.warn(format_args!(
                        "Dropping merge commit {} until next queue refresh (failed to refresh commit)",
                        merge_uuid,
                    ));
                    logger.warn(format_args!("{:?}", e));
                }
            };
        }
        self.merge_commit_queue = refreshed_merge_commits;
    }

    pub fn pop_next_task(&mut self, collection: &TaskCollection) -> Option<CommitTask> {
        // Phase 1: Always try merge commits first (if any are eligible); resetting the counter
        if let Some(merge_uuid) = self.get_best_eligible_merge_commit(collection, false) {
            self.merge_commit_queue.remove(&merge_uuid);
            self.pr_commits_since_merge = 0;
            return self.map.remove(&merge_uuid).map(|task| task.task);
        }

        // Phase 2: If no merge commits available, do up to 4 PR commits; incrementing the counter
        if self.pr_commits_since_merge < 2 {
            if let Some(uuid) = self.pr_commit_queue.pop_front() {
                self.pr_commits_since_merge += 1;
                return self.map.remove(&uuid).map(|task| task.task);
            }
        }

        // Phase 3: After 4 PR commits, do any available merge commit (even if it would be ineligible by the conflict check); resetting the counter
        if self.pr_commits_since_merge >= 2 || self.pr_commit_queue.is_empty() {
            if let Some(merge_uuid) = self.get_best_eligible_merge_commit(collection, true) {
                self.merge_commit_queue.remove(&merge_uuid);
                self.pr_commits_since_merge = 0;
                return self.map.remove(&merge_uuid).map(|task| task.task);
            }
        }

        // Phase 4: If there are no merge commits, just do PR commits. If there are no PR commits, try unapproved parent queue.
        if let Some(uuid) = self.pr_commit_queue.pop_front() {
            self.pr_commits_since_merge += 1;
            return self.map.remove(&uuid).map(|task| task.task);
        }

        if let Some(uuid) = self.unapproved_parent_queue.pop_front() {
            return self.map.remove(&uuid).map(|task| task.task);
        }

        None
    }

    pub fn new(collection: &TaskCollection) -> Self {
        let mut new = Self {
            map: HashMap::with_capacity(collection.n_commits()),
            blocked_map: HashMap::with_capacity(collection.n_commits()),
            pr_commit_queue: VecDeque::with_capacity(collection.n_commits()),
            unapproved_parent_queue: VecDeque::with_capacity(collection.n_commits()),
            merge_commit_queue: HashSet::with_capacity(collection.n_pulls()),
            pr_commits_since_merge: 0,
        };

        let mut iter = collection.commits();
        let mut stack = vec![];
        if let Some(next) = iter.next() {
            stack.push(next);
        }
        while let Some((uuid, commit)) = stack.pop() {
            // Jankily topo-sort the commit list by maintaining a stack
            // where we push-and-restart.
            if let Some(parent) = commit.dep_uuid() {
                // If we already know about the task just skip it.
                if !new.map.contains_key(&parent) {
                    stack.push((uuid, commit));
                    stack.push((parent, collection.commit(parent).unwrap()));
                    continue;
                }
            }
            // Queue the next thing.
            if stack.is_empty() {
                if let Some(next) = iter.next() {
                    stack.push(next);
                }
            }
            // If we already know about the task just skip it.
            if new.map.contains_key(&uuid) {
                continue;
            }

            // If the task is not approved, we don't need to do it.
            // But we may want to record it.
            match *commit.review_status() {
                ReviewStatus::Unreviewed => {
                    new.map.insert(*uuid, Task {
                        task: commit.clone(),
                        state: State::Unapproved,
                    });
                },
                ReviewStatus::NeedsChange | ReviewStatus::Nacked => {
                    new.map.insert(*uuid, Task {
                        task: commit.clone(),
                        state: State::Rejected,
                    });
                },
                ReviewStatus::ApprovedNoCi => {
                    // If the commit was "approved without CI" we just treat that as having passed CI.
                    new.map.insert(*uuid, Task {
                        task: commit.clone(),
                        state: State::Success,
                    });
                }
                ReviewStatus::Approved => {
                    // This is the interesting case. Once approved, a commit goes into the CI queue.
                    match *commit.ci_status() {
                        CiStatus::Success => {
                            new.map.insert(*uuid, Task {
                                task: commit.clone(),
                                state: State::Success,
                            });
                        },
                        // "Cancelled" is set manually to tell the CI not to bother with something, e.g.
                        // if the branch was force-pushed away or was already merged or something. If
                        // we cancel a job we should treat it as though it failed, for scheduling
                        // purposes.
                        CiStatus::Failed | CiStatus::Cancelled => {
                            new.map.insert(*uuid, Task {
                                task: commit.clone(),
                                state: State::Failed,
                            });
                        },
                        // FIXME for "started" tasks don't just start them, check the host and see if it's
                        // ours, and if not leave it be.
                        CiStatus::Unstarted | CiStatus::Started => {
                            // ok, let's do it
                            new.map.insert(*uuid, Task {
                                task: commit.clone(),
                                state: State::Approved,
                            });

                            if commit.is_merge_commit() {
                                new.merge_commit_queue.insert(*uuid);
                            } else {
                                // Walk up the dependency chain to check parent states
                                let mut current_uuid = commit.dep_uuid();
                                let mut has_rejected_or_failed_parent = false;
                                let mut has_unapproved_parent = false;
                                let mut blocking_parent = None;

                                while let Some(parent_uuid) = current_uuid {
                                    let parent_task = new.map.get(parent_uuid)
                                        .expect("Parent task should already be processed and in map");
                                    
                                    match parent_task.state {
                                        State::Rejected | State::Failed => {
                                            has_rejected_or_failed_parent = true;
                                            blocking_parent = Some(*parent_uuid);
                                            break;
                                        },
                                        State::Unapproved => {
                                            has_unapproved_parent = true;
                                        },
                                        State::Approved | State::Success => {
                                            // Continue walking up the chain
                                        },
                                    }
                                    current_uuid = parent_task.task.dep_uuid();
                                }

                                if has_rejected_or_failed_parent {
                                    if let Some(blocking_uuid) = blocking_parent {
                                        new.blocked_map.entry(blocking_uuid).or_insert_with(Vec::new).push(*uuid);
                                    }
                                } else if has_unapproved_parent {
                                    new.unapproved_parent_queue.push_back(*uuid);
                                } else {
                                    new.pr_commit_queue.push_back(*uuid);
                                }
                            }
                        },
                    }
                },
            }
        }

        new
    }
}

fn format_pr_numbers(prs: &[usize]) -> String {
    if prs.is_empty() {
        String::new()
    } else if prs.len() == 1 {
        format!("[#{}] ", prs[0])
    } else {
        let pr_list: Vec<String> = prs.iter().map(|pr| format!("#{}", pr)).collect();
        format!("[{}] ", pr_list.join("]["))
    }
}
