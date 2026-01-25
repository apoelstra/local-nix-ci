// SPDX-License-Identifier: GPL-3.0-or-later

mod log;
mod state;

use crate::tw::TaskCollection;
use crate::tw::serde_types::{CiStatus, ReviewStatus, MergeStatus};
use crate::git::GitCommit;
use anyhow::Context as _;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};
use xshell::{Shell, cmd};

const INITIAL_BACKOFF: Duration = Duration::from_secs(30);
const MAXIMUM_BACKOFF: Duration = Duration::from_hours(1);
const IDLE_SLEEP: Duration = Duration::from_secs(3);

pub fn run(task_shell: &Shell) -> Result<(), anyhow::Error> {
    let mut logger = log::Logger::new();
    let mut state = state::CiState::new()?;
    let mut backoff = INITIAL_BACKOFF;
    let mut last_check = Instant::now();
    let mut busy = false;

    logger.info(format_args!("Starting CI loop."));
    loop {
        // Check CI repo status periodically
        if let Err(e) = state.check_ci_repo_status() {
            logger.error(format_args!("Failed to check CI repo status: {e}"));
        }

        // Find next approved commit that needs CI. Call `check_and_push_ready_prs`
        // except during idle times. Reload the task database on every loop iteration,
        // because until we do, we'll miss any updates the user did via CLI. (Later,
        // when we daemonize this, we can have the user send a reload signal somehow.)
        let mut tasks = TaskCollection::new(task_shell)
            .context("reloading task database")?;
        let commit_uuid = match find_next_commit_for_ci(&tasks) {
            Some(uuid) => {
                check_and_push_ready_prs(&logger, &mut tasks)?;
                uuid
            }
            None => {
                if busy {
                    check_and_push_ready_prs(&logger, &mut tasks)?;
                    backoff = INITIAL_BACKOFF;
                    last_check = Instant::now();
                    busy = false;
                } else {
                    std::thread::sleep(IDLE_SLEEP);
                    if last_check.elapsed() >= backoff {
                        if backoff < MAXIMUM_BACKOFF {
                            backoff *= 2;
                        }

                        check_and_push_ready_prs(&logger, &mut tasks)?;

                        logger.info(format_args!(
                            "Nothing to do. Reloading task database. (Next message in {} minutes.)",
                            backoff.as_secs() / 60,
                        )); 
                        last_check = Instant::now();
                    }
                }
                continue;
            }
        };

        busy = true;
        let commit_task = tasks.commit(&commit_uuid).unwrap();

        logger.newline();
        logger.set_task(Some(commit_task.clone()));
        logger.info("Starting.");
        // Update commit status to started
        tasks.update_commit_ci_status(&commit_uuid, CiStatus::Started)?;

        // Process the commit
        let commit_task = tasks.commit(&commit_uuid).unwrap(); // Re-get after update
        let commit_task = commit_task.clone(); // un-borrow `tasks`
        match process_commit(&logger, &mut tasks, &commit_task, &mut state) {
            Ok(success) => {
                if success {
                    logger.info("SUCCESS.");
                    tasks.update_commit_ci_status(&commit_uuid, CiStatus::Success)?;
                    
                    // Check all PRs containing this commit for merge readiness
                    for pr_number in commit_task.prs() {
                        let pr_task = tasks.pull_by_number(commit_task.project(), *pr_number)
                            .expect("PR in task collection")
                            .clone(); // clone to unborrow `tasks`
                        match tasks.check_and_update_pr_merge_readiness(pr_task.uuid()) {
                            Ok(true) => {
                                logger.info(format_args!(
                                    "PR #{} change ID {} now needs signature ({} ACKs).",
                                    pr_number,
                                    pr_task.merge_change_id(),
                                    pr_task.github_acks().lines().count(),
                                ));
                            },
                            Ok(false) => {}, // No change needed
                            Err(e) => logger.warn(format_args!(
                                "Failed to check PR #{} merge readiness: {}", 
                                pr_number, e
                            )),
                        }
                    }
                    
                    post_approvals(&logger, &mut tasks, &commit_uuid)?;
                } else {
                    logger.error("FAILED");
                    tasks.update_commit_ci_status(&commit_uuid, CiStatus::Failed)?;
                }
            }
            Err(e) => {
                logger.error(format_args!("Unable to process commit: {e}"));
                tasks.update_commit_ci_status(&commit_uuid, CiStatus::Failed)?;
            }
        }
        logger.set_task(None);
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
    logger: &log::Logger,
    tasks: &mut TaskCollection, 
    commit_task: &crate::tw::CommitTask, 
    state: &mut state::CiState
) -> anyhow::Result<bool> {
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
    logger.info("Instantiating derivation.");
    let instantiate_result = Command::new("nix-instantiate")
        .arg("--show-trace")
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
                logger.info(format_args!("Instantiated derivation: {path}"));
                
                tasks.update_commit_derivation(commit_task.uuid(), path.clone())?;
                path
            } else {
                let error_content = format!(
                    "nix-instantiate failed for commit {}\n\nSTDOUT:\n{}\n\nSTDERR:\n{}",
                    commit_task.commit_id(),
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
                save_error_to_file(logger, repo_root, commit_task.commit_id(), "instantiate", &error_content)?;
                return Ok(false);
            }
        }
        Err(e) => {
            let error_content = format!("Failed to run nix-instantiate for commit {}: {}", 
                                       commit_task.commit_id(), e);
            save_error_to_file(logger, repo_root, commit_task.commit_id(), "instantiate", &error_content)?;
            return Ok(false);
        }
    };

    // Build the derivation
    logger.info("Building derivation.");

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
                save_error_to_file(logger, repo_root, commit_task.commit_id(), "build", &error_content)?;
                Ok(false)
            }
        }
        Err(e) => {
            logger.error("Failed to run nix-build: {e}.");
            let error_content = format!("Failed to run nix-build for commit {}: {}", 
                                       commit_task.commit_id(), e);
            save_error_to_file(logger, repo_root, commit_task.commit_id(), "build", &error_content)?;
            Ok(false)
        }
    }
}

fn find_cargo_lockfiles(sh: &Shell, commit_id: &GitCommit) -> anyhow::Result<Vec<String>> {
    let output = cmd!(sh, "git ls-tree -r --name-only {commit_id}")
        .read()
        .context("Failed to list files in commit")?;

    let mut lockfiles: Vec<String> = output
        .lines()
        .filter(|line| line.contains("Cargo") && line.ends_with(".lock"))
        .map(|s| s.to_string())
        .collect();

    // If no lockfiles found in commit, search the ".." directory for auxiliary lockfiles
    if lockfiles.is_empty() {
        if let Ok(aux_output) = cmd!(sh, "ls ../*.lock").ignore_status().read() {
            for line in aux_output.lines() {
                let filename = line.trim();
                if !filename.is_empty() && filename.contains("Cargo") {
                    // Convert to absolute path using realpath
                    if let Ok(absolute_path) = cmd!(sh, "realpath ../{filename}").read() {
                        lockfiles.push(absolute_path.trim().to_string());
                    }
                }
            }
        }
    }

    Ok(lockfiles)
}

fn save_error_to_file(logger: &log::Logger, repo_root: &Path, commit_id: &GitCommit, operation: &str, content: &str) -> anyhow::Result<()> {
    use chrono::Utc;
    
    let error_dir = repo_root.parent().unwrap_or(repo_root);
    let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
    let filename = format!("nix-error-{}-{}-{}.log", commit_id, operation, timestamp);
    let error_path = error_dir.join(filename);

    fs::write(&error_path, content)
        .with_context(|| format!("Failed to write error file: {}", error_path.display()))?;

    logger.info(format_args!("Error details saved to: {}", error_path.display()));

    Ok(())
}

fn post_approvals(logger: &log::Logger, tasks: &mut TaskCollection, _commit_uuid: &uuid::Uuid) -> anyhow::Result<()> {
    // Check all PRs to see if they're ready for approval or merge status update
    for (_, pr_task) in tasks.pulls() {
        // Skip already-pushed things.
        if *pr_task.merge_status() == MergeStatus::Pushed {
            continue;
        }

        let all_commits_approved_and_successful = pr_task.commits(tasks)
            .all(|commit| {
                *commit.review_status() == ReviewStatus::Approved 
                && *commit.ci_status() == CiStatus::Success
            });

        // If all commits succeeded and PR approved, post approval
        if all_commits_approved_and_successful && *pr_task.review_status() == ReviewStatus::Approved {
            let sh = Shell::new()?;
            let repo_root = pr_task.repo_root();
            let repo = crate::repo::Repository {
                project_name: pr_task.project().to_string(),
                repo_root: repo_root.to_owned(),
            };
            
            if let Err(e) = crate::post_github_approval_if_ready(&sh, tasks, &repo, pr_task) {
                logger.error(format_args!(
                    "Failed to post GitHub approval for PR #{}: {}", 
                     pr_task.number(), 
                     e,
                ));
            }
        }

        // If all commits succeeded but PR not approved, alert user
        if all_commits_approved_and_successful && *pr_task.review_status() != ReviewStatus::Approved {
            logger.warn(format_args!(
                "PR #{} has all commits approved and successful, but PR itself is not approved. Please approve it.", 
                pr_task.number(),
            ));
        }
    }
    Ok(())
}

fn check_and_push_ready_prs(
    logger: &log::Logger,
    tasks: &mut TaskCollection,
) -> anyhow::Result<()> {
    let sh = Shell::new()?;
    // Find PRs with merge_status:needsig. Collect into a Vec to avoid keeping
    // a borrow of `tasks`.
    let needsig_prs: Vec<_> = tasks.pulls()
        .filter(|(_, pr)| *pr.merge_status() == MergeStatus::NeedSig)
        .map(|(uuid, pr)| (*uuid, pr.number(), pr.project().to_string()))
        .collect();

    for (pr_uuid, pr_number, project) in needsig_prs {
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
                logger.error(format_args!(
                    "Failed to refresh PR #{}: {}", 
                    pr_number, e,
                ));
                continue;
            }
        };

        // If merge status is no longer NeedSig after refresh, skip
        if *refreshed_pr.merge_status() != MergeStatus::NeedSig {
            continue;
        }

        let _push_dir = sh.push_dir(refreshed_pr.repo_root());
        let merge_change_id = refreshed_pr.merge_change_id().to_owned();

        // Check if JJ change has GPG signature
        let has_signature = match crate::jj::jj_log(&sh, "if(signature, \"true\", \"false\")", &merge_change_id) {
            Ok(result) => result.trim() == "true",
            Err(e) => {
                logger.error(format_args!(
                    "Failed to check signature for PR #{}: {}", 
                    pr_number, e,
                ));
                continue;
            }
        };

        // Update merge commit description with latest ACKs
        let refreshed_pr = refreshed_pr.clone();

        if has_signature {
            // Get the merge commit ID from JJ change
            let merge_commit_id = match crate::jj::jj_log(&sh, "commit_id", merge_change_id) {
                Ok(id) => id.trim().to_string(),
                Err(e) => {
                    logger.error(format_args!(
                        "Failed to get commit ID for PR #{}: {}", 
                        pr_number, e,
                    ));
                    continue;
                }
            };

            // Get base branch name from stored data
            let base_ref = refreshed_pr.base_ref();

            logger.info(format_args!(
                "{} PR #{} has GPG signature, pushing to {}", 
                project, pr_number, base_ref,
            ));

            // Push to base branch
            match cmd!(sh, "git push origin {merge_commit_id}:{base_ref}").run() {
                Ok(_) => {
                    logger.info("Successfully pushed.");
                    
                    // Update merge status to pushed
                    if let Err(e) = tasks.update_pr_merge_status(&pr_uuid, MergeStatus::Pushed) {
                        logger.error(format_args!(
                            "Failed to update merge status for PR #{}: {}", 
                            pr_number, e,
                        ));
                    }
                }
                Err(e) => {
                    logger.error(format_args!(
                        "Failed to push PR #{}: {}", 
                        pr_number, e,
                    ));
                }
            }
        } else {
            // Query GitHub for ACKs on this PR
            let ack_count = refreshed_pr.github_acks().lines().count();

            logger.info(format_args!(
                "*** {} PR #{} JJ change {} does not have GPG signature yet. Please sign it. ({} ACK(s))", 
                project, pr_number, merge_change_id, ack_count,
            ));
        }
    }

    Ok(())
}
