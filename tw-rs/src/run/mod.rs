// SPDX-License-Identifier: GPL-3.0-or-later

mod merge_description;
mod state;

use crate::tw::TaskCollection;
use crate::tw::serde_types::{CiStatus, ReviewStatus, MergeStatus};
use crate::git::GitCommit;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};
use xshell::{Shell, cmd};
use chrono::Utc;

pub fn run(tasks: &mut TaskCollection) -> Result<(), anyhow::Error> {
    let mut state = state::CiState::new()?;
    let mut pr_alert_timers = HashMap::new();
    let mut backoff_sec = 30;
    let mut sleep_sec = backoff_sec; // trigger status update on first iteration
    let mut busy = false;

    eprintln!("[{}] Starting CI loop", Utc::now().format("%Y-%m-%d %H:%M:%S"));

    loop {
        // Check CI repo status periodically
        if let Err(e) = state.check_ci_repo_status() {
            eprintln!("[{}] Error checking CI repo status: {}", 
                     Utc::now().format("%Y-%m-%d %H:%M:%S"), e);
        }

        // Find next approved commit that needs CI
        let next_commit_uuid = find_next_commit_for_ci(tasks);

        // Handle idle/busy status
        if next_commit_uuid.is_none() {
            if busy {
                check_and_push_ready_prs(tasks)?;
                backoff_sec = 30;
                sleep_sec = 1;
                busy = false;
            } else {
                std::thread::sleep(Duration::from_secs(1));
                sleep_sec += 1;

                if sleep_sec >= backoff_sec {
                    if backoff_sec < 2400 {
                        backoff_sec *= 2;
                    }

                    check_and_push_ready_prs(tasks)?;
                    check_needsig_prs(tasks, &mut pr_alert_timers)?;

                    eprintln!("[{}] Nothing to do. (Next message in {} minutes.)", 
                             Utc::now().format("%Y-%m-%d %H:%M:%S"), 
                             backoff_sec / 60);
                    sleep_sec = 1;
                }
            }
            continue;
        }

        busy = true;
        let commit_uuid = next_commit_uuid.unwrap();
        let commit_task = tasks.commit(&commit_uuid).unwrap();
        
        eprintln!("[{}] Processing commit {} from project {}", 
                 Utc::now().format("%Y-%m-%d %H:%M:%S"), 
                 commit_task.commit_id(), 
                 commit_task.project());

        // Update commit status to started
        tasks.update_commit_ci_status(&commit_uuid, CiStatus::Started)?;

        // Process the commit
        let commit_task = tasks.commit(&commit_uuid).unwrap(); // Re-get after update
        let commit_task = commit_task.clone(); // un-borrow `tasks`
        match process_commit(tasks, &commit_task, &mut state) {
            Ok(success) => {
                if success {
                    eprintln!("[{}] Build succeeded for commit {}", 
                             Utc::now().format("%Y-%m-%d %H:%M:%S"), 
                             commit_task.commit_id());
                    tasks.update_commit_ci_status(&commit_uuid, CiStatus::Success)?;
                    
                    // Check all PRs containing this commit for merge readiness
                    let affected_prs: Vec<_> = tasks.pulls()
                        .filter(|(_, pr_task)| pr_task.commits(tasks).any(|c| c.commit_id() == commit_task.commit_id()))
                        .map(|(pr_uuid, pr_task)| (*pr_uuid, pr_task.number()))
                        .collect();

                    for (pr_uuid, pr_number) in affected_prs {
                        match tasks.check_and_update_pr_merge_readiness(&pr_uuid) {
                            Ok(true) => eprintln!("[{}] PR #{} is now ready for merge (status updated to needsig)", 
                                                 Utc::now().format("%Y-%m-%d %H:%M:%S"), pr_number),
                            Ok(false) => {}, // No change needed
                            Err(e) => eprintln!("[{}] Warning: Failed to check PR #{} merge readiness: {}", 
                                               Utc::now().format("%Y-%m-%d %H:%M:%S"), pr_number, e),
                        }
                    }
                    
                    check_for_pushable_merges(tasks, &commit_uuid)?;
                } else {
                    eprintln!("[{}] Build failed for commit {}", 
                             Utc::now().format("%Y-%m-%d %H:%M:%S"), 
                             commit_task.commit_id());
                    tasks.update_commit_ci_status(&commit_uuid, CiStatus::Failed)?;
                }
            }
            Err(e) => {
                eprintln!("[{}] Error processing commit {}: {}", 
                         Utc::now().format("%Y-%m-%d %H:%M:%S"), 
                         commit_task.commit_id(), 
                         e);
                tasks.update_commit_ci_status(&commit_uuid, CiStatus::Failed)?;
            }
        }

        eprintln!("[{}] Finished processing commit {}", 
                 Utc::now().format("%Y-%m-%d %H:%M:%S"), 
                 commit_task.commit_id());
    }
}

fn find_next_commit_for_ci(tasks: &TaskCollection) -> Option<uuid::Uuid> {
    for (uuid, commit) in tasks.commits() {
        if *commit.review_status() == ReviewStatus::Approved 
            && *commit.ci_status() == CiStatus::Unstarted {
            return Some(*uuid);
        }
    }
    None
}

fn process_commit(
    tasks: &mut TaskCollection, 
    commit_task: &crate::tw::CommitTask, 
    state: &mut state::CiState
) -> Result<bool> {
    let sh = Shell::new()?;
    let repo_root = commit_task.repo_root();
    let _push_dir = sh.push_dir(repo_root);

    // Compute nixfile path
    let project = commit_task.project();
    let nixfile_name = format!("{}.check-pr.nix", project);
    let nixfile_path = state.temp_nix_dir().join(&nixfile_name);

    if !nixfile_path.exists() {
        return Err(anyhow::anyhow!("Nixfile not found: {}", nixfile_path.display()));
    }

    // Find Cargo.lock files
    let lockfiles = find_cargo_lockfiles(&sh, commit_task.commit_id())?;
    
    // Check for Cargo.toml without Cargo.lock
    let commit_id = commit_task.commit_id();
    let has_cargo_toml = cmd!(sh, "git ls-tree -r --name-only {commit_id}")
        .read()?
        .lines()
        .any(|line| line.ends_with("Cargo.toml"));

    if has_cargo_toml && lockfiles.is_empty() {
        return Err(anyhow::anyhow!("Found Cargo.toml files but no Cargo.lock files"));
    }

    // Build cargo nixes JSON
    let cargo_nixes = if lockfiles.is_empty() {
        "{}".to_string()
    } else {
        let entries: Vec<String> = lockfiles.iter()
            .map(|lockfile| format!("\"{}\" = null", lockfile))
            .collect();
        format!("{{ {}; }}", entries.join("; "))
    };

    // Build commit JSON
    let is_tip = commit_task.is_tip();
    let commit_str = format!(
        "{{ commit = \"{}\"; isTip = {}; gitUrl = \"{}\"; cargoNixes = {}; }}",
        commit_task.commit_id(),
        is_tip,
        repo_root.display(),
        cargo_nixes
    );

    // Add ci_dirty_suffix to commit task if needed
    tasks.update_commit_local_ci_commit_id(
        commit_task.uuid(),
        state.local_ci_commit_id().to_owned(),
    ).context("failed adding local CI commit ID to task")?;

    // Instantiate derivation. Because we want to capture our streams even if they fail,
    // we must use std::process::Command directly rather than xshell.
    eprintln!("[{}] Instantiating derivation for commit {}", 
             Utc::now().format("%Y-%m-%d %H:%M:%S"), 
             commit_task.commit_id());

    let instantiate_result = Command::new("nix-instantiate")
        .arg("--arg").arg("inlineJsonConfig")
        .arg(format!("{{ gitDir = \"{}\"; projectName = \"{}\"; }}", repo_root.display(), project))
        .arg("--arg").arg("inlineCommitList")
        .arg(format!("[ {} ]", commit_str))
        .arg("--arg").arg("prNum").arg("\"\"")
        .arg(&nixfile_path)
        .current_dir(repo_root)
        .output();

    let derivation_path = match instantiate_result {
        Ok(output) => {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                eprintln!("[{}] Instantiated derivation: {}", 
                         Utc::now().format("%Y-%m-%d %H:%M:%S"), 
                         path);
                
                tasks.update_commit_derivation(commit_task.uuid(), path.clone())?;
                path
            } else {
                let error_content = format!(
                    "nix-instantiate failed for commit {}\n\nSTDOUT:\n{}\n\nSTDERR:\n{}",
                    commit_task.commit_id(),
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
                save_error_to_file(repo_root, commit_task.commit_id(), "instantiate", &error_content)?;
                return Ok(false);
            }
        }
        Err(e) => {
            let error_content = format!("Failed to run nix-instantiate for commit {}: {}", 
                                       commit_task.commit_id(), e);
            save_error_to_file(repo_root, commit_task.commit_id(), "instantiate", &error_content)?;
            return Ok(false);
        }
    };

    // Build the derivation
    eprintln!("[{}] Building derivation for commit {}", 
             Utc::now().format("%Y-%m-%d %H:%M:%S"), 
             commit_task.commit_id());

    let build_result = Command::new("nix-build")
        .arg("--builders-use-substitutes")
        .arg("--no-build-output")
        .arg("--no-out-link")
        .arg("--keep-failed")
        .arg("--keep-derivations")
        .arg("--keep-outputs")
        .arg("--log-lines").arg("100")
        .arg(&derivation_path)
        .arg("--log-format").arg("internal-json")
        .arg("-v")
        .current_dir(repo_root)
        .output();

    match build_result {
        Ok(output) => {
            if output.status.success() {
                Ok(true)
            } else {
                let error_content = format!(
                    "nix-build failed for commit {}\n\nSTDOUT:\n{}\n\nSTDERR:\n{}",
                    commit_task.commit_id(),
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
                save_error_to_file(repo_root, commit_task.commit_id(), "build", &error_content)?;
                Ok(false)
            }
        }
        Err(e) => {
            let error_content = format!("Failed to run nix-build for commit {}: {}", 
                                       commit_task.commit_id(), e);
            save_error_to_file(repo_root, commit_task.commit_id(), "build", &error_content)?;
            Ok(false)
        }
    }
}

fn find_cargo_lockfiles(sh: &Shell, commit_id: &GitCommit) -> Result<Vec<String>> {
    let output = cmd!(sh, "git ls-tree -r --name-only {commit_id}")
        .read()
        .context("Failed to list files in commit")?;

    let lockfiles: Vec<String> = output
        .lines()
        .filter(|line| line.contains("Cargo") && line.ends_with(".lock"))
        .map(|s| s.to_string())
        .collect();

    for lockfile in &lockfiles {
        eprintln!("[{}] Found Cargo.lock at {}", 
                 Utc::now().format("%Y-%m-%d %H:%M:%S"), 
                 lockfile);
    }

    Ok(lockfiles)
}

fn save_error_to_file(repo_root: &Path, commit_id: &GitCommit, operation: &str, content: &str) -> Result<()> {
    let error_dir = repo_root.parent().unwrap_or(repo_root);
    let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
    let filename = format!("nix-error-{}-{}-{}.log", commit_id, operation, timestamp);
    let error_path = error_dir.join(filename);

    fs::write(&error_path, content)
        .with_context(|| format!("Failed to write error file: {}", error_path.display()))?;

    eprintln!("[{}] Error details saved to: {}", 
             Utc::now().format("%Y-%m-%d %H:%M:%S"), 
             error_path.display());

    Ok(())
}

fn check_for_pushable_merges(tasks: &mut TaskCollection, _commit_uuid: &uuid::Uuid) -> Result<()> {
    let mut to_update = vec![];
    // Check all PRs to see if they're ready for approval or merge status update
    for (_, pr_task) in tasks.pulls() {
        let all_commits_approved_and_successful = pr_task.commits(tasks)
            .all(|commit| {
                *commit.review_status() == ReviewStatus::Approved 
                && *commit.ci_status() == CiStatus::Success
            });

        let merge_commit = pr_task.merge_commit(tasks);
        let merge_approved_and_successful = *merge_commit.review_status() == ReviewStatus::Approved 
            && *merge_commit.ci_status() == CiStatus::Success;

        // If all commits succeeded and PR approved, post approval
        if all_commits_approved_and_successful && *pr_task.review_status() == ReviewStatus::Approved {
            let sh = Shell::new()?;
            let repo_root = pr_task.repo_root();
            let repo = crate::repo::Repository {
                project_name: pr_task.project().to_string(),
                repo_root: repo_root.to_owned(),
            };
            
            if let Err(e) = crate::post_github_approval_if_ready(&sh, tasks, &repo, pr_task) {
                eprintln!("[{}] Failed to post GitHub approval for PR #{}: {}", 
                         Utc::now().format("%Y-%m-%d %H:%M:%S"), 
                         pr_task.number(), 
                         e);
            }
        }

        // If all commits succeeded but PR not approved, alert user
        if all_commits_approved_and_successful && *pr_task.review_status() != ReviewStatus::Approved {
            eprintln!("[{}] PR #{} has all commits approved and successful, but PR itself is not approved. Please approve it.", 
                     Utc::now().format("%Y-%m-%d %H:%M:%S"), 
                     pr_task.number());
        }

        // If everything is approved and successful, bump to needsig
        if all_commits_approved_and_successful 
            && merge_approved_and_successful 
            && *pr_task.review_status() == ReviewStatus::Approved 
            && *pr_task.merge_status() != MergeStatus::NeedSig 
            && *pr_task.merge_status() != MergeStatus::Pushed {
            
            eprintln!("[{}] PR #{} is ready for signing, updating merge_status to needsig", 
                     Utc::now().format("%Y-%m-%d %H:%M:%S"), 
                     pr_task.number());

            to_update.push(*pr_task.uuid());
        }
    }

    // For borrowck reasons we have to do this outside of the above loop
    for uuid in to_update {
        tasks.update_pr_merge_status(&uuid, MergeStatus::NeedSig)
            .context("update merge status")?;
    }
    Ok(())
}

fn check_and_push_ready_prs(tasks: &mut TaskCollection) -> Result<()> {
    let sh = Shell::new()?;
    
    // Find PRs with merge_status:needsig. Collect into a Vec to avoid keepin
    // a borrow of `tasks`.
    let needsig_prs: Vec<_> = tasks.pulls()
        .filter(|(_, pr)| *pr.merge_status() == MergeStatus::NeedSig)
        .map(|(uuid, pr)| (*uuid, pr.number(), pr.project().to_string()))
        .collect();

    for (pr_uuid, pr_number, project) in needsig_prs {
        eprintln!("[{}] Checking PR #{} for push readiness", 
                 Utc::now().format("%Y-%m-%d %H:%M:%S"), 
                 pr_number);

        // Get repository info
        let pr_task = tasks.pulls().find(|(uuid, _)| **uuid == pr_uuid).unwrap().1;
        let repo = crate::repo::Repository {
            project_name: project.clone(),
            repo_root: pr_task.repo_root().to_owned(),
        };

        // Refresh the PR to check if merge commit is still up to date
        let refreshed_pr = match tasks.insert_or_refresh_pr(&sh, &repo, pr_number) {
            Ok(pr) => pr,
            Err(e) => {
                eprintln!("[{}] Failed to refresh PR #{}: {}", 
                         Utc::now().format("%Y-%m-%d %H:%M:%S"), 
                         pr_number, e);
                continue;
            }
        };

        // If merge status is no longer NeedSig after refresh, skip
        if *refreshed_pr.merge_status() != MergeStatus::NeedSig {
            eprintln!("[{}] PR #{} merge status changed during refresh, skipping", 
                     Utc::now().format("%Y-%m-%d %H:%M:%S"), 
                     pr_number);
            continue;
        }

        let _push_dir = sh.push_dir(refreshed_pr.repo_root());
        let merge_change_id = refreshed_pr.merge_change_id().to_owned();

        // Check if JJ change has GPG signature
        let has_signature = match crate::jj::jj_log(&sh, "if(signature, \"true\", \"false\")", &merge_change_id) {
            Ok(result) => result.trim() == "true",
            Err(e) => {
                eprintln!("[{}] Failed to check signature for PR #{}: {}", 
                         Utc::now().format("%Y-%m-%d %H:%M:%S"), 
                         pr_number, e);
                continue;
            }
        };

        // Update merge commit description with latest ACKs
        let refreshed_pr = refreshed_pr.clone();
        let description = merge_description::compute_merge_description(&sh, tasks, &refreshed_pr, &merge_change_id)?;
        if let Err(e) = cmd!(sh, "jj describe --quiet -r {merge_change_id} -m {description}").run() {
            eprintln!("[{}] Failed to update description for PR #{}: {}", 
                     Utc::now().format("%Y-%m-%d %H:%M:%S"), 
                     pr_number, e);
            continue;
        }

        if has_signature {
            // Get the merge commit ID from JJ change
            let merge_commit_id = match crate::jj::jj_log(&sh, "commit_id", merge_change_id) {
                Ok(id) => id.trim().to_string(),
                Err(e) => {
                    eprintln!("[{}] Failed to get commit ID for PR #{}: {}", 
                             Utc::now().format("%Y-%m-%d %H:%M:%S"), 
                             pr_number, e);
                    continue;
                }
            };

            // Get base branch name from stored data
            let base_ref = refreshed_pr.base_ref();

            eprintln!("[{}] {} PR #{} has GPG signature, pushing to {}", 
                     Utc::now().format("%Y-%m-%d %H:%M:%S"), 
                     project, pr_number, base_ref);

            // Push to base branch
            match cmd!(sh, "git push origin {merge_commit_id}:{base_ref}").run() {
                Ok(_) => {
                    eprintln!("[{}] Successfully pushed PR #{} to {}", 
                             Utc::now().format("%Y-%m-%d %H:%M:%S"), 
                             pr_number, base_ref);
                    
                    // Update merge status to pushed
                    if let Err(e) = tasks.update_pr_merge_status(&pr_uuid, MergeStatus::Pushed) {
                        eprintln!("[{}] Failed to update merge status for PR #{}: {}", 
                                 Utc::now().format("%Y-%m-%d %H:%M:%S"), 
                                 pr_number, e);
                    }
                }
                Err(e) => {
                    eprintln!("[{}] Failed to push PR #{} to {}: {}", 
                             Utc::now().format("%Y-%m-%d %H:%M:%S"), 
                             pr_number, base_ref, e);
                }
            }
        } else {
            eprintln!("[{}] {} PR #{} JJ change {} does not have GPG signature yet", 
                     Utc::now().format("%Y-%m-%d %H:%M:%S"), 
                     project, pr_number, merge_change_id);
        }
    }

    Ok(())
}

fn check_needsig_prs(tasks: &TaskCollection, pr_alert_timers: &mut HashMap<usize, Instant>) -> Result<()> {
    for (_, pr_task) in tasks.pulls() {
        if *pr_task.merge_status() == MergeStatus::NeedSig {
            let pr_number = pr_task.number();
            let now = Instant::now();
            
            // Check if we should alert (first time or 15 minutes since last alert)
            let should_alert = pr_alert_timers
                .get(&pr_number)
                .map(|last_alert| now.duration_since(*last_alert) >= Duration::from_secs(15 * 60))
                .unwrap_or(true);

            if should_alert {
                // Count ACKs by parsing review notes or comments
                let ack_count = count_acks_in_pr(pr_task)?;
                
                eprintln!("[{}] PR #{} needs GPG signature ({} ACKs). Please sign it.", 
                         Utc::now().format("%Y-%m-%d %H:%M:%S"), 
                         pr_number, 
                         ack_count);
                
                pr_alert_timers.insert(pr_number, now);
            }
        }
    }
    Ok(())
}

fn count_acks_in_pr(pr_task: &crate::tw::PrTask) -> Result<usize> {
    // Simple ACK counting - look for "ACK" in review notes
    // This is a simplified version; the full implementation would need to
    // parse GitHub comments and reviews like the Python script does
    let review_notes = pr_task.review_notes();
    let ack_count = review_notes.lines()
        .filter(|line| line.contains("ACK"))
        .count();
    
    Ok(ack_count)
}
