// SPDX-License-Identifier: GPL-3.0-or-later

mod log;
mod stacks;

use anyhow::Context as _;
use lcilib::Db;
use lcilib::db::models::{Commit, Repository, UpdateCommit, CiStatus};
use std::path::Path;
use std::time::Duration;
use tokio::{fs, time, task};
use tokio::process::Command;
use chrono::Utc;
use xshell::Shell;

pub async fn run(_db: &mut Db) -> anyhow::Result<()> {
    log::info(format_args!("Starting local-ci daemon..."));
    
    // Start all cycles concurrently, each with its own database connection
    let tasks = vec![
        tokio::spawn(run_db_maintenance_cycle()),
        tokio::spawn(run_ci_cycle_loop()),
        tokio::spawn(run_pr_sync_cycle()),
    ];
    
    // Wait for all tasks to complete (which should never happen)
    for task in tasks {
        task.await
            .context("daemon task failed")??;
    }
    
    Ok(())
}

async fn run_db_maintenance_cycle() -> anyhow::Result<()> {
    let mut db = Db::connect().await
        .context("connecting to database for maintenance cycle")?;
    
    loop {
        let mut had_work = false;

        // Check for pending ACKs that need to be posted
        if check_pending_acks(&mut db).await? {
            had_work = true;
        }

        // Check for approved PRs that need merge commits created
        if check_approved_prs(&mut db).await? {
            had_work = true;
        }

        // Check for signed merge commits that need to be pushed
        if check_signed_merges(&mut db).await? {
            had_work = true;
        }

        if !had_work {
            time::sleep(Duration::from_secs(5)).await;
        }
    }
}

async fn run_pr_sync_cycle() -> anyhow::Result<()> {
    let mut db = Db::connect().await
        .context("connecting to database for PR sync cycle")?;
    
    // Run every 15 minutes
    let mut interval = time::interval(Duration::from_secs(15 * 60));
    
    loop {
        interval.tick().await;
        
        match sync_all_repositories(&mut db).await {
            Ok(()) => {
                log::info(format_args!("PR sync cycle completed successfully"));
            }
            Err(e) => {
                log::warn(format_args!("Error in PR sync cycle: {}", e));
            }
        }
    }
}

async fn run_ci_cycle_loop() -> anyhow::Result<()> {
    let mut db = Db::connect().await
        .context("connecting to database for CI cycle")?;
    
    let mut no_work_count = 0u32;
    let mut last_no_work_log = std::time::Instant::now();
    
    loop {
        if let Some(commit) = stacks::find_next_commit_to_test(&mut db).await
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
                log::info(format_args!("Nothing to do"));
                no_work_count += 1;
                last_no_work_log = now;
            }
            
            time::sleep(Duration::from_secs(1)).await;
        }
    }
}

async fn check_pending_acks(_db: &mut Db) -> anyhow::Result<bool> {
    // TODO: Query for PRs with pending ACKs
    // TODO: Check if all non-merge commits are approved and passed CI
    // TODO: Post ACK and update status to 'posted'
    // Return true if work was done, false if nothing to do
    time::sleep(Duration::from_secs(15)).await;
    Ok(false)
}

async fn check_approved_prs(_db: &mut Db) -> anyhow::Result<bool> {
    // TODO: Query for approved but unmerged PRs
    // TODO: Create merge commit on highest-priority non-conflicting stack
    // TODO: Create individual stack for the merge
    // TODO: Handle rebase warnings
    // Return true if work was done, false if nothing to do
    time::sleep(Duration::from_secs(15)).await;
    Ok(false)
}

async fn check_signed_merges(_db: &mut Db) -> anyhow::Result<bool> {
    // TODO: Query for signed merge commits that passed CI
    // TODO: Push them
    // Return true if work was done, false if nothing to do
    time::sleep(Duration::from_secs(15)).await;
    Ok(false)
}

async fn get_repository_for_commit(db: &mut Db, commit: &Commit) -> anyhow::Result<Repository> {
    let tx = db.transaction().await.context("starting transaction")?;
    let repo = Repository::find_by_id(&tx, commit.repository_id).await
        .map_err(|e| anyhow::anyhow!("Database error: {}", e))?
        .ok_or_else(|| anyhow::anyhow!("Repository not found for ID: {}", commit.repository_id))?;
    tx.commit().await.context("committing transaction")?;
    Ok(repo)
}

async fn mark_commit_failed(db: &mut Db, commit: &Commit, reason: &str) -> anyhow::Result<()> {
    let tx = db.transaction().await.context("starting transaction")?;
    let updates = UpdateCommit {
        ci_status: Some(CiStatus::Failed),
        review_text: Some(Some(reason.to_string())),
        ..Default::default()
    };
    commit.update(&tx, updates).await
        .map_err(|e| anyhow::anyhow!("Failed to update commit: {}", e))?;
    tx.commit().await.context("committing transaction")?;
    Ok(())
}

async fn mark_commit_passed(db: &mut Db, commit: &Commit) -> anyhow::Result<()> {
    let tx = db.transaction().await.context("starting transaction")?;
    let updates = UpdateCommit {
        ci_status: Some(CiStatus::Passed),
        ..Default::default()
    };
    commit.update(&tx, updates).await
        .map_err(|e| anyhow::anyhow!("Failed to update commit: {}", e))?;
    tx.commit().await.context("committing transaction")?;
    Ok(())
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

    // Check for cancellation every 10 seconds during instantiation
    let mut interval = time::interval(Duration::from_secs(10));
    
    loop {
        tokio::select! {
            status_result = child.wait() => {
                match status_result {
                    Ok(status) => {
                        // Process finished, get the output
                        let output = child.wait_with_output().await
                            .context("Failed to get output from nix-instantiate")?;
                        
                        if !status.success() {
                            let error_content = format!(
                                "nix-instantiate failed for commit {}\n\nSTDOUT:\n{}\nSTDERR:\n{}",
                                commit.git_commit_id,
                                String::from_utf8_lossy(&output.stdout),
                                String::from_utf8_lossy(&output.stderr)
                            );
                            
                            save_error_to_file(&repo.path, &commit.git_commit_id, "instantiate", &error_content).await?;
                            mark_commit_failed(db, commit, "nix-instantiate failed").await?;
                            return Err(anyhow::anyhow!("nix-instantiate failed"));
                        }

                        let stdout_str = String::from_utf8_lossy(&output.stdout);
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
            "--builders-use-substitutes",
            "--no-build-output",
            "--no-out-link",
            "--keep-failed",
            "--keep-derivations",
            "--keep-outputs",
            "--log-lines", "100",
            derivation_path,
            "-v",
        ])
        .current_dir(repo_path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to spawn nix-build")?;

    // Check for cancellation every 30 seconds
    let mut interval = time::interval(Duration::from_secs(30));
    
    loop {
        tokio::select! {
            status_result = child.wait() => {
                match status_result {
                    Ok(status) => {
                        // Process finished, get the output
                        let output = child.wait_with_output().await
                            .context("Failed to get output from nix-build")?;
                        
                        if status.success() {
                            log::info(format_args!("nix-build completed successfully"));
                            return Ok(true);
                        }

                        let error_content = format!(
                            "nix-build failed for commit {}\n\nSTDOUT:\n{}\nSTDERR:\n{}",
                            commit.git_commit_id,
                            String::from_utf8_lossy(&output.stdout),
                            String::from_utf8_lossy(&output.stderr)
                        );
                        
                        save_error_to_file(&repo_path.to_string_lossy(), &commit.git_commit_id, "build", &error_content).await?;
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

async fn sync_all_repositories(db: &mut Db) -> anyhow::Result<()> {
    let tx = db.transaction().await
        .context("starting transaction to get repositories")?;
    
    let repositories = Repository::list_all(&tx).await
        .map_err(|e| anyhow::anyhow!("Failed to get repositories: {}", e))?;
    
    tx.commit().await
        .context("committing transaction")?;
    
    for repo in repositories {
        if let Err(e) = sync_repository_prs(db, &repo).await {
            log::warn(format_args!("Failed to sync PRs for repository {}: {}", repo.name, e));
        }
    }
    
    Ok(())
}

async fn sync_repository_prs(db: &mut Db, repo: &Repository) -> anyhow::Result<()> {
    struct Data {
        current_repo: lcilib::repo::Repository,
        pr_infos: Vec<lcilib::gh::PrInfo>,
    }

    // Use spawn_blocking for the GitHub API calls and shell operations
    let repo_path = repo.path.clone();
    let last_synced = repo.last_synced_at;
    
    let data = task::spawn_blocking(move || -> Option<Data> {
        // Check if repository path exists
        let shell = Shell::new().ok()?; // just eat shell creation error; this basically cannot happen
        if !Path::new(&repo_path).exists() {
            log::warn(format_args!("Warning: Repository path does not exist: {}", repo_path));
            return None;
        }
        shell.change_dir(&repo_path);
        
        // Get updated PRs from GitHub
        let pr_infos = match lcilib::gh::list_updated_prs(&shell, last_synced)  {
            Ok(prs) => prs,
            Err(e) => {
                log::warn(format_args!("Warning: Failed to get updated PRs for repository {}: {}", repo_path, e));
                return None;
            }
        };

        let current_repo = match lcilib::repo::current_repo(&shell) {
            Ok(repo) => repo,
            Err(e) => {
                log::warn(format_args!("Warning: failed to get current repo for repository path {}: {}", repo_path, e));
                return None;
            }
        };

        Some(Data { current_repo, pr_infos })
    }).await
    .context("spawning blocking task for GitHub API calls")?;

    let Some(data) = data else { return Ok(()) };
    if !data.pr_infos.is_empty() {
        log::info(format_args!("Found {} updated PRs for repository {}", data.pr_infos.len(), repo.name));
    }
    for pr_info in &data.pr_infos {
        // A shell cannot live across await points so we have to recreate it on every iteration
        // and hand ownership to the refresh function.
        let shell = Shell::new().expect("this just worked above..");
        shell.change_dir(&repo.path);
        if let Err(e) = crate::pr::refresh(shell, &data.current_repo, pr_info, db).await
            .with_context(|| format!("failed to refresh PR #{}", pr_info.number))
        {
            log::warn(format_args!("Warning: Failed to process PR #{} in repository {}: {}", 
                pr_info.number, repo.name, e));
        } else {
            log::info(format_args!("Successfully synced PR #{} in repository {}", pr_info.number, repo.name));
        }
    }
    
    // Update last synced time
    let tx = db.transaction().await
        .context("starting transaction to update last synced time")?;
    
    repo.update_last_synced(&tx).await
        .map_err(|e| anyhow::anyhow!("Failed to update last synced time: {}", e))?;
    
    tx.commit().await
        .context("committing transaction")?;
    
    Ok(())
}

async fn save_error_to_file(
    repo_path: &str,
    commit_id: &str,
    operation: &str,
    content: &str,
) -> anyhow::Result<()> {
    let repo_path = Path::new(repo_path);
    let error_dir = repo_path.parent().unwrap_or(repo_path);
    let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
    let filename = format!("local-ci-error-{}-{}-{}.log", commit_id, operation, timestamp);
    let error_path = error_dir.join(filename);

    fs::write(&error_path, content).await
        .with_context(|| format!("Failed to write error file: {}", error_path.display()))?;

    log::info(format_args!("Error details saved to: {}", error_path.display()));

    Ok(())
}
