// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Context as _;
use lcilib::{
    Db,
    db::CiStatus,
    db::models::{Commit, PullRequest, Stack, Repository, UpdateCommit},
};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use chrono::Utc;
use tokio::{fs, io::{BufReader, AsyncReadExt as _}, time};
use tokio::process::Command;

use super::{get_repository_for_commit, mark_commit_failed, mark_commit_passed};
use super::{log, util};

/// Find the next commit that needs testing, following the priority rules
/// 
/// # Errors
/// 
/// Returns an error if database operations fail.
async fn find_next_commit_to_test(db: &mut Db) -> anyhow::Result<Option<Commit>> {
    let tx = db.transaction().await
        .context("starting transaction")?;

    // Get repository info for GPG checking
    let repos = Repository::list_all(&tx).await
        .context("getting repository list")?;
    let repo_map: HashMap<i32, &Repository> = repos.iter().map(|r| (r.id, r)).collect();

     // Compute lists of available work and print summary.
    let high_priority_stacks = Stack::find_highest_priority_by_repo_branch(&tx).await
        .context("finding high-priority stacks")?;
    let prs_needing_testing = PullRequest::find_needing_testing_prioritized(&tx).await
        .context("finding PRs needing testing")?;
    let low_priority_stacks = Stack::find_low_priority_stacks(&tx).await
        .context("finding low-priority stacks")?;
    
    print_work_summary(&tx, &high_priority_stacks, &prs_needing_testing, &low_priority_stacks).await
        .context("printing work summary")?;
    
    // 1. Check high-priority stacks first (with positive priority)
    let mut prioritized_stacks = Vec::new();
    for (stack, commits) in &high_priority_stacks {
        let priority = util::calculate_stack_priority(commits, &tx, &repo_map).await
            .context("calculating stack priority")?;
        if priority > 0.0 {
            prioritized_stacks.push((stack, commits, priority));
        }
    }
    
    // Sort by priority (highest first)
    prioritized_stacks.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    
    for (stack, _commits, _priority) in &prioritized_stacks {
        if let Some(commit) = stack.get_next_untested_commit(&tx).await
            .context("getting next untested commit from stack")? {
            log::info("Found commit from high-priority stack");
            tx.commit().await.context("committing transaction")?;
            return Ok(Some(commit));
        }
    }

    // 2. Check PRs by priority (user priority, then all approved, then fewer untested, then age)
    let mut prioritized_prs = Vec::new();
    for pr in &prs_needing_testing {
        let (total_commits, approved_commits, untested_commits) = pr.get_commit_counts(&tx).await
            .context("getting PR commit counts")?;
        
        let all_approved = approved_commits == total_commits;
        let age_days = (Utc::now() - pr.created_at).num_days();
        
        prioritized_prs.push((pr, all_approved, -untested_commits, age_days));
    }
    
    // Sort by: priority DESC, all_approved DESC, fewer untested DESC (negative untested), age DESC (older first)
    prioritized_prs.sort_by(|a, b| {
        a.0.priority.cmp(&b.0.priority).reverse()
            .then(a.1.cmp(&b.1).reverse())
            .then(a.2.cmp(&b.2).reverse())
            .then(a.3.cmp(&b.3).reverse())
    });
    
    for (pr, _all_approved, _neg_untested, _age) in &prioritized_prs {
        if let Some(commit) = pr.get_next_untested_commit(&tx).await
            .context("getting next untested commit from PR")? {
            log::info("Found commit from PR");
            tx.commit().await.context("committing transaction")?;
            return Ok(Some(commit));
        }
    }

    // 3. Check low-priority stacks (negative priority or conflicting)
    let mut low_priority_with_scores = Vec::new();
    for (stack, commits) in &low_priority_stacks {
        let priority = util::calculate_stack_priority(commits, &tx, &repo_map).await
            .context("calculating low-priority stack priority")?;
        low_priority_with_scores.push((stack, commits, priority));
    }
    
    // Sort by priority (highest first, even if negative)
    low_priority_with_scores.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    
    for (stack, _commits, _priority) in &low_priority_with_scores {
        if let Some(commit) = stack.get_next_untested_commit(&tx).await
            .context("getting next untested commit from low-priority stack")? {
            log::info("Found commit from low-priority stack");
            tx.commit().await.context("committing transaction")?;
            return Ok(Some(commit));
        }
    }

    tx.commit().await.context("committing transaction")?;
    Ok(None)
}

/// Print a summary of all remaining work
async fn print_work_summary(
    tx: &lcilib::Transaction<'_>,
    high_priority_stacks: &[(Stack, Vec<Commit>)],
    prs_needing_testing: &[PullRequest],
    low_priority_stacks: &[(Stack, Vec<Commit>)],
) -> anyhow::Result<()> {
    // Get repository info for display names
    let repos = Repository::list_all(tx).await
        .context("getting repository list")?;
    let repo_map: HashMap<i32, &Repository> = repos.iter().map(|r| (r.id, r)).collect();

    if prs_needing_testing.is_empty() && high_priority_stacks.is_empty() && low_priority_stacks.is_empty() {
        // If there is nothing to do, print no summary. We will use backoff logic
        // to print "nothing to do" messages without spamming the user at a higher
        // layer.
        return Ok(());
    }

    log::info("=== Available Work Summary ===");

    // Print PR summary with individual commits
    for pr in prs_needing_testing {
        let repo_name = repo_map.get(&pr.repository_id)
            .map_or("unknown", |r| r.name.as_str());
        
        let (total_commits, approved_commits, untested_commits) = pr.get_commit_counts(tx).await
            .context("getting PR commit counts")?;
        
        let unapproved = total_commits - approved_commits;
        let review_status_text = match pr.review_status {
            lcilib::db::models::ReviewStatus::Approved => "approved".to_string(),
            lcilib::db::models::ReviewStatus::Unreviewed => "unreviewed".to_string(),
            lcilib::db::models::ReviewStatus::Rejected => "rejected".to_string(),
        };
        
        let pr_line = if unapproved > 0 {
            format!(
                "{} PR#{} {} commits left to test ({} unapproved) (PR {})",
                repo_name, pr.pr_number, untested_commits, unapproved, review_status_text
            )
        } else {
            format!(
                "{} PR#{} {} commits left to test (PR {})",
                repo_name, pr.pr_number, untested_commits, review_status_text
            )
        };
        
        log::info(format_args!("{}", pr_line));
        
        // Get commits that need testing for this PR
        let commits_to_test = get_commits_needing_testing_for_pr(tx, pr).await
            .context("getting commits needing testing for PR")?;
        
        for commit in commits_to_test {
            let jj_change_id_short = if commit.jj_change_id.len() > 12 {
                &commit.jj_change_id[..12]
            } else {
                &commit.jj_change_id
            };
            
            log::info(format_args!(
                "  - {} ({})",
                commit.git_commit_id, jj_change_id_short
            ));
        }
    }

    // Print high-priority stack summary
    for (stack, _commits) in high_priority_stacks {
        let repo_name = repo_map.get(&stack.repository_id)
            .map_or("unknown", |r| r.name.as_str());
        
        let prs = stack.get_associated_prs(tx).await
            .context("getting associated PRs for stack")?;
        let pr_numbers: Vec<String> = prs.iter().map(|pr| format!("#{}", pr.pr_number)).collect();
        
        let (_total, signed, untested) = stack.get_commit_counts(tx).await
            .context("getting stack commit counts")?;
        
        log::info(format_args!(
            "{} {} PRs {} ({} signed, {} left to test)",
            repo_name, stack.target_branch, pr_numbers.join(", "), signed, untested
        ));
    }

    // Print low-priority stack summary if any
    if !low_priority_stacks.is_empty() {
        log::info("=== Low Priority Stacks ===");
        for (stack, _commits) in low_priority_stacks {
            let repo_name = repo_map.get(&stack.repository_id)
                .map_or("unknown", |r| r.name.as_str());
            
            let prs = stack.get_associated_prs(tx).await
                .context("getting associated PRs for low-priority stack")?;
            let pr_numbers: Vec<String> = prs.iter().map(|pr| format!("#{}", pr.pr_number)).collect();
            
            let (_total, signed, untested) = stack.get_commit_counts(tx).await
                .context("getting low-priority stack commit counts")?;
            
            log::info(format_args!(
                "{} {} PRs {} ({} signed, {} left to test)",
                repo_name, stack.target_branch, pr_numbers.join(", "), signed, untested
            ));
        }
    }
    log::info("");

    Ok(())
}

/// Get commits that need testing for a specific PR
/// 
/// # Errors
/// 
/// Returns an error if database operations fail.
async fn get_commits_needing_testing_for_pr(
    tx: &lcilib::Transaction<'_>,
    pr: &PullRequest,
) -> anyhow::Result<Vec<Commit>> {
    let rows = tx
        .query(
            r#"
            SELECT c.id, c.repository_id, c.git_commit_id, c.jj_change_id, c.review_status,
                   c.should_run_ci, c.ci_status, c.nix_derivation, c.review_text, c.created_at
            FROM commits c
            JOIN pr_commits pc ON c.id = pc.commit_id
            WHERE pc.pull_request_id = $1 
            AND pc.is_current = true
            AND c.review_status = 'approved'
            AND c.ci_status = 'unstarted'
            AND c.should_run_ci = true
            ORDER BY pc.sequence_order ASC
            "#,
            &[&pr.id],
        )
        .await
        .context("querying commits needing testing for PR")?;

    let commits = rows.iter().map(|row| Commit {
        id: row.get("id"),
        repository_id: row.get("repository_id"),
        git_commit_id: row.get("git_commit_id"),
        jj_change_id: row.get("jj_change_id"),
        review_status: row.get("review_status"),
        should_run_ci: row.get("should_run_ci"),
        ci_status: row.get("ci_status"),
        nix_derivation: row.get("nix_derivation"),
        review_text: row.get("review_text"),
        created_at: row.get("created_at"),
    }).collect();

    Ok(commits)
}


async fn process_commit_ci(db: &mut Db, commit: &Commit, repo: &Repository) -> anyhow::Result<bool> {
    let repo_path = Path::new(&repo.path);
    let nixfile_path = Path::new(&repo.nixfile_path);

    // Check if nixfile exists
    if !nixfile_path.exists() {
        let error_msg = format!("Nixfile not found: {}", nixfile_path.display());
        log::warn(format_args!("{}", error_msg));
        mark_commit_failed(db, commit, &error_msg).await?;
        return Ok(false);
    }

    // Find Cargo.lock files
    let lockfiles = match find_cargo_lockfiles(repo_path, &commit.git_commit_id).await {
        Ok(files) => files,
        Err(e) => {
            let error_msg = format!("Failed to find Cargo.lock files: {}", e);
            log::warn(format_args!("{}", error_msg));
            mark_commit_failed(db, commit, &error_msg).await?;
            return Ok(false);
        }
    };

    // Check for Cargo.toml without Cargo.lock
    let has_cargo_toml = match check_has_cargo_toml(repo_path, &commit.git_commit_id).await {
        Ok(has_toml) => has_toml,
        Err(e) => {
            let error_msg = format!("Failed to check for Cargo.toml files: {}", e);
            log::warn(format_args!("{}", error_msg));
            mark_commit_failed(db, commit, &error_msg).await?;
            return Ok(false);
        }
    };

    if has_cargo_toml && lockfiles.is_empty() {
        let error_msg = "Found Cargo.toml files but no Cargo.lock files";
        log::warn(format_args!("{}", error_msg));
        mark_commit_failed(db, commit, error_msg).await?;
        return Ok(false);
    }

    // Build cargo nixes JSON
    let cargo_nixes = if lockfiles.is_empty() {
        "{}".to_string()
    } else {
        let entries: Vec<String> = lockfiles
            .iter()
            .map(|lockfile| format!("\"{}\" = null", lockfile))
            .collect();
        format!("{{ {}; }}", entries.join("; "))
    };

    // Get derivation path with cancellation checking
    let derivation_path = match get_or_create_derivation_with_cancellation(db, commit, repo, &cargo_nixes).await {
        Ok(path) => path,
        Err(e) => {
            let error_msg = format!("Failed to get derivation: {}", e);
            log::warn(format_args!("{}", error_msg));
            mark_commit_failed(db, commit, &error_msg).await?;
            return Ok(false);
        }
    };

    // Build the derivation with cancellation checking
    match build_derivation_with_cancellation(db, commit, repo_path, &derivation_path).await {
        Ok(success) => {
            if !success {
                // Error already logged and commit marked as failed
            }
            Ok(success)
        }
        Err(e) => {
            let error_msg = format!("Build process error: {}", e);
            log::warn(format_args!("{}", error_msg));
            mark_commit_failed(db, commit, &error_msg).await?;
            Ok(false)
        }
    }
}

#[expect(clippy::case_sensitive_file_extension_comparisons)] // neat lint. complains about looking for .lock files. deliberately violating for now.
async fn find_cargo_lockfiles(repo_path: &Path, commit_id: &str) -> anyhow::Result<Vec<String>> {
    let output = Command::new("git")
        .args(["ls-tree", "-r", "--name-only", commit_id])
        .current_dir(repo_path)
        .output()
        .await
        .context("Failed to run git ls-tree")?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "git ls-tree failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut lockfiles: Vec<String> = stdout
        .lines()
        .filter(|line| line.starts_with("Cargo") && line.ends_with(".lock"))
        .map(str::to_owned)
        .collect();

    // If no lockfiles found in commit, search the ".." directory for auxiliary lockfiles
    if lockfiles.is_empty() {
        let find_output = Command::new("find")
            .args(["../", "-maxdepth", "1", "-name", "Cargo*.lock"])
            .current_dir(repo_path)
            .output()
            .await;

        if let Ok(aux_output) = find_output
            && aux_output.status.success()
        {
            let aux_stdout = String::from_utf8_lossy(&aux_output.stdout);
            for line in aux_stdout.lines() {
                let filename = line.trim();
                if !filename.is_empty() && filename.contains("Cargo") {
                    // Convert to absolute path using realpath
                    let realpath_output = Command::new("realpath")
                        .arg(filename)
                        .current_dir(repo_path)
                        .output()
                        .await;

                    if let Ok(realpath_result) = realpath_output
                        && realpath_result.status.success()
                    {
                        let absolute_path = String::from_utf8_lossy(&realpath_result.stdout);
                        lockfiles.push(absolute_path.trim().to_string());
                    }
                }
            }
        }
    }

    Ok(lockfiles)
}

async fn check_has_cargo_toml(repo_path: &Path, commit_id: &str) -> anyhow::Result<bool> {
    let output = Command::new("git")
        .args(["ls-tree", "-r", "--name-only", commit_id])
        .current_dir(repo_path)
        .output()
        .await
        .context("Failed to run git ls-tree")?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "git ls-tree failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let has_cargo_toml = stdout
        .lines()
        .any(|line| line.ends_with("Cargo.toml"));

    Ok(has_cargo_toml)
}

async fn get_or_create_derivation_with_cancellation(
    db: &mut Db,
    commit: &Commit,
    repo: &Repository,
    cargo_nixes: &str,
) -> anyhow::Result<String> {
    // Check if derivation already exists and is valid
    if let Some(existing_derivation) = &commit.nix_derivation {
        if Path::new(existing_derivation).exists() {
            log::info(format_args!("Using existing derivation: {}", existing_derivation));
            return Ok(existing_derivation.clone());
        }
        log::info(format_args!("Existing derivation {} not found, will recreate", existing_derivation));
    }

    // Build commit JSON for nix-instantiate
    let commit_str = format!(
        "{{ commit = \"{}\"; isTip = true; gitUrl = \"{}\"; cargoNixes = {}; }}",
        commit.git_commit_id,
        repo.path,
        cargo_nixes
    );

    // Instantiate derivation with cancellation checking
    log::info(format_args!("Instantiating derivation for commit {}", commit.git_commit_id));
    
    let mut child = Command::new("nix-instantiate")
        .args([
            "--show-trace",
            "--arg", "inlineJsonConfig",
            &format!("{{ gitDir = \"{}\"; projectName = \"{}\"; }}", repo.path, repo.name),
            "--arg", "inlineCommitList",
            &format!("[ {} ]", commit_str),
            "--arg", "prNum",
            "\"\"",
            &repo.nixfile_path,
        ])
        .current_dir(&repo.path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to spawn nix-instantiate")?;

    // Collect stdout and stderr into a vec (in principle there is a memory-DoS vector here, but
    // nix-instantiate should never output more than a few hundred kb, even on error, so okay).
    let stdout = child.stdout.take().expect("stdout piped");
    let stderr = child.stderr.take().expect("stderr piped");

    let stdout_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stdout);
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).await?;
        Ok::<Vec<u8>, std::io::Error>(buf)
    });

    let stderr_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stderr);
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).await?;
        Ok::<Vec<u8>, std::io::Error>(buf)
    });

    // Check for cancellation every 10 seconds during instantiation
    let mut interval = time::interval(Duration::from_secs(10));
    
    loop {
        tokio::select! {
            status_result = child.wait() => {
                match status_result {
                    Ok(status) => {
                        // Process finished, get the output
                        let stdout = stdout_task.await??;
                        let stderr = stderr_task.await??;
                        
                        if !status.success() {
                            save_error_to_file(&repo.path, &commit.git_commit_id, "instantiate", &stdout, &stderr).await?;
                            mark_commit_failed(db, commit, "nix-instantiate failed").await?;
                            return Err(anyhow::anyhow!("nix-instantiate failed"));
                        }

                        let stdout_str = String::from_utf8_lossy(&stdout);
                        let lines: Vec<&str> = stdout_str.lines().collect();

                        let derivation_path = if lines.len() > 1 {
                            log::info(format_args!(
                                "nix-instantiate returned {} lines, taking only the first line",
                                lines.len()
                            ));
                            lines[0].trim().to_string()
                        } else {
                            stdout_str.trim().to_string()
                        };

                        // Check if derivation exists on filesystem
                        if !Path::new(&derivation_path).exists() {
                            return Err(anyhow::anyhow!(
                                "Instantiated derivation does not exist: {}",
                                derivation_path
                            ));
                        }

                        log::info(format_args!("Instantiated derivation: {}", derivation_path));

                        // Update commit with derivation path
                        let tx = db.transaction().await.context("starting transaction")?;
                        let updates = UpdateCommit {
                            nix_derivation: Some(Some(derivation_path.clone())),
                            ..Default::default()
                        };
                        commit.update(&tx, updates).await
                            .map_err(|e| anyhow::anyhow!("Failed to update commit with derivation: {}", e))?;
                        tx.commit().await.context("committing transaction")?;

                        return Ok(derivation_path);
                    }
                    Err(e) => {
                        return Err(anyhow::anyhow!("Failed to check nix-instantiate status: {}", e));
                    }
                }
            }
            
            // Check for cancellation every 10 seconds
            _ = interval.tick() => {
                if let Err(e) = check_for_cancellation(db, commit).await {
                    log::warn(format_args!("Error checking for cancellation: {}", e));
                    continue;
                }
                
                // Check if we should cancel
                if should_cancel_ci(db, commit).await? {
                    log::info(format_args!("CI cancellation requested for commit {}", commit.git_commit_id));
                    
                    // Kill the child process
                    if let Err(e) = child.kill().await {
                        log::warn(format_args!("Failed to kill nix-instantiate process: {}", e));
                    }
                    
                    // Wait for it to actually exit
                    let _ = child.wait().await;
                    
                    let error_msg = "CI cancelled by user request";
                    log::warn(format_args!("{}", error_msg));
                    mark_commit_failed(db, commit, error_msg).await?;
                    return Err(anyhow::anyhow!("{}", error_msg));
                }
            }
        }
    }
}

async fn build_derivation_with_cancellation(
    db: &mut Db,
    commit: &Commit,
    repo_path: &Path,
    derivation_path: &str,
) -> anyhow::Result<bool> {
    log::info(format_args!("Building derivation: {}", derivation_path));

    // Start the nix-build process
    let mut child = Command::new("nix-build")
        .args([
            "--no-build-output",
            "--no-out-link",
            "--keep-failed",
            derivation_path,
            "-v",
        ])
        .current_dir(repo_path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to spawn nix-build")?;

    // Collect stdout and stderr into a vec (in principle there is a memory-DoS vector here but in practice
    // it should be fine for even the most crazy nix derivations have only a few megs of output.
    let stdout = child.stdout.take().expect("stdout piped");
    let stderr = child.stderr.take().expect("stderr piped");

    let stdout_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stdout);
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).await?;
        Ok::<Vec<u8>, std::io::Error>(buf)
    });

    let stderr_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stderr);
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).await?;
        Ok::<Vec<u8>, std::io::Error>(buf)
    });

    // Check for cancellation every 30 seconds
    let mut interval = time::interval(Duration::from_secs(30));
    
    loop {
        tokio::select! {
            status_result = child.wait() => {
                match status_result {
                    Ok(status) => {
                        // Process finished, get the output
                        let stdout = stdout_task.await??;
                        let stderr = stderr_task.await??;
                        
                        if status.success() {
                            log::info(format_args!("nix-build completed successfully"));
                            return Ok(true);
                        }
                        
                        save_error_to_file(&repo_path.to_string_lossy(), &commit.git_commit_id, "build", &stdout, &stderr).await?;
                        mark_commit_failed(db, commit, "nix-build failed").await?;
                        return Ok(false);
                    }
                    Err(e) => {
                        let error_msg = format!("Failed to check nix-build status: {}", e);
                        log::warn(format_args!("{}", error_msg));
                        mark_commit_failed(db, commit, &error_msg).await?;
                        return Ok(false);
                    }
                }
            }
            
            // Check for cancellation every 30 seconds
            _ = interval.tick() => {
                if let Err(e) = check_for_cancellation(db, commit).await {
                    log::warn(format_args!("Error checking for cancellation: {}", e));
                    continue;
                }
                
                // Check if we should cancel
                if should_cancel_ci(db, commit).await? {
                    log::info(format_args!("CI cancellation requested for commit {}", commit.git_commit_id));
                    
                    // Kill the child process
                    if let Err(e) = child.kill().await {
                        log::warn(format_args!("Failed to kill nix-build process: {}", e));
                    }
                    
                    // Wait for it to actually exit
                    let _ = child.wait().await;
                    
                    let error_msg = "CI cancelled by user request";
                    log::warn(format_args!("{}", error_msg));
                    return Ok(false);
                }
            }
        }
    }
}

async fn check_for_cancellation(db: &mut Db, _commit: &Commit) -> anyhow::Result<()> {
    // Just ensure we can still connect to the database
    let tx = db.transaction().await.context("starting transaction for cancellation check")?;
    tx.commit().await.context("committing cancellation check transaction")?;
    Ok(())
}

async fn should_cancel_ci(db: &mut Db, commit: &Commit) -> anyhow::Result<bool> {
    let tx = db.transaction().await.context("starting transaction")?;
    
    // Reload the commit to check current status
    let current_commit = Commit::find_by_id(&tx, commit.id).await
        .map_err(|e| anyhow::anyhow!("Failed to reload commit: {}", e))?
        .ok_or_else(|| anyhow::anyhow!("Commit not found: {}", commit.id))?;
    
    tx.commit().await.context("committing transaction")?;
    
    // Check if CI should be cancelled
    let should_cancel = current_commit.ci_status == CiStatus::Skipped || !current_commit.should_run_ci;
    
    Ok(should_cancel)
}

async fn save_error_to_file(
    repo_path: &str,
    commit_id: &str,
    operation: &str,
    stdout: &[u8],
    stderr: &[u8],
) -> anyhow::Result<()> {
    let repo_path = Path::new(repo_path);
    let error_dir = repo_path.parent().unwrap_or(repo_path);
    let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
    let filename = format!("local-ci-error-{}-{}-{}.log", commit_id, operation, timestamp);
    let error_path = error_dir.join(filename);

    fs::write(&error_path, format!("nix-build failed for commit {}\n\nSTDOUT:\n", commit_id)).await
        .with_context(|| format!("Failed to write error file (preamble): {}", error_path.display()))?;
    fs::write(&error_path, stdout).await
        .with_context(|| format!("Failed to write error file (stdout): {}", error_path.display()))?;
    fs::write(&error_path, "\nSTDERR:\n").await
        .with_context(|| format!("Failed to write error file (stderr preamble): {}", error_path.display()))?;
    fs::write(&error_path, stderr).await
        .with_context(|| format!("Failed to write error file (stderr): {}", error_path.display()))?;

    log::info(format_args!("Error details saved to: {}", error_path.display()));

    Ok(())
}

pub async fn run_ci_cycle_loop() -> anyhow::Result<()> {
    let mut db = Db::connect().await
        .context("connecting to database for CI cycle")?;
    
    let mut no_work_count = 0u32;
    let mut last_no_work_log = std::time::Instant::now();
    
    loop {
        if let Some(commit) = find_next_commit_to_test(&mut db).await
            .context("finding next commit to test")?
        {
            log::info(format_args!("Starting CI for commit: {} ({})", commit.git_commit_id, commit.jj_change_id));
        
            // Get repository info
            let repo = match get_repository_for_commit(&mut db, &commit).await {
                Ok(repo) => repo,
                Err(e) => {
                    log::warn_backoff(format!("Failed to get repository for commit {}: {}", commit.git_commit_id, e)).await;
                    continue;
                }
            };

            // Process the commit
            match process_commit_ci(&mut db, &commit, &repo).await {
                Ok(success) => {
                    log::reset_error_sleep();
                    if success {
                        log::info(format_args!("CI SUCCESS for commit: {}", commit.git_commit_id));
                        mark_commit_passed(&mut db, &commit).await?;
                        // After a commit succeeds, re-scan the database to see if we should make merge commits or something
                        super::real_run_db_maintenance_cycle(&mut db).await?;
                    } else {
                        log::warn(format_args!("CI FAILED for commit: {}", commit.git_commit_id));
                        // Error details already logged and commit marked as failed in process_commit_ci
                    }
                }
                Err(e) => {
                    log::warn_backoff(format!("Error processing commit {}: {}", commit.git_commit_id, e)).await;
                    mark_commit_failed(&mut db, &commit, &format!("Processing error: {}", e)).await?;
                }
            }
        
        } else {
            // Nothing to do -- just sleep for a second and maybe post a 'nothing to do' message so the
            // user knows we're still alive.
            let delay_secs = match no_work_count {
                0 => 5,
                1 => 60,
                2 => 300,
                n => 300 * (1u64 << (n - 2)).min(18000), // Cap at 5 hours
            };
            
            let now = std::time::Instant::now();
            if now.duration_since(last_no_work_log).as_secs() >= delay_secs {
                log::info(format_args!("Nothing to do."));
                no_work_count += 1;
                last_no_work_log = now;
            }
            
            time::sleep(Duration::from_secs(1)).await;
        }
    }
}
