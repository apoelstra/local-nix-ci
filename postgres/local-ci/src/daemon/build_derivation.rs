// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Context as _;
use chrono::Utc;
use lcilib::{
    Db,
    db::CiStatus,
    db::models::{
        Commit, CommitToTest, CommitType, Repository, UpdateCommit,
    },
    git::CommitId,
};
use std::path::Path;
use std::time::Duration;
use tokio::{
    fs,
    process::Command,
    io::{AsyncReadExt as _, BufReader},
    time,
};

use super::mark_commit_status;
use super::log;

pub async fn process_commit_ci(
    db: &mut Db,
    commit: &CommitToTest,
    repo: &Repository,
) -> anyhow::Result<bool> {
    let repo_path = Path::new(&repo.path);

    // Find Cargo.lock files
    let lockfiles = match find_cargo_lockfiles(repo_path, &commit.git_commit_id).await {
        Ok(files) => files,
        Err(e) => {
            log::warn(
                &*e.into_boxed_dyn_error(),
                "Failed to find Cargo.lock files",
            );
            mark_commit_status(db, commit.id, CiStatus::Failed).await?;
            return Ok(false);
        }
    };

    // Check for Cargo.toml without Cargo.lock
    let has_cargo_toml = match check_has_cargo_toml(repo_path, &commit.git_commit_id).await {
        Ok(has_toml) => has_toml,
        Err(e) => {
            log::warn(
                &*e.into_boxed_dyn_error(),
                "Failed to check for Cargo.toml files",
            );
            mark_commit_status(db, commit.id, CiStatus::Failed).await?;
            return Ok(false);
        }
    };

    if has_cargo_toml && lockfiles.is_empty() {
        // FIXME promote these error checks to an error type, make this a warning
        log::info("Found Cargo.toml files but no Cargo.lock files");
        mark_commit_status(db, commit.id, CiStatus::Failed).await?;
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
    let derivation_path =
        match get_or_create_derivation_with_cancellation(db, commit, repo, &cargo_nixes).await {
            Ok(path) => path,
            Err(e) => {
                log::warn(&*e.into_boxed_dyn_error(), "Failed to get derivation");
                mark_commit_status(db, commit.id, CiStatus::Failed).await?;
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
            log::warn(&*e.into_boxed_dyn_error(), "Build failed");
            mark_commit_status(db, commit.id, CiStatus::Failed).await?;
            Ok(false)
        }
    }
}

async fn get_or_create_derivation_with_cancellation(
    db: &mut Db,
    commit: &CommitToTest,
    repo: &Repository,
    cargo_nixes: &str,
) -> anyhow::Result<String> {
    // Check if derivation already exists and is valid
    if let Some(existing_derivation) = &commit.nix_derivation {
        if Path::new(existing_derivation).exists() {
            log::info(format_args!(
                "Using existing derivation: {}",
                existing_derivation
            ));
            return Ok(existing_derivation.clone());
        }
        log::info(format_args!(
            "Existing derivation {} not found, will recreate",
            existing_derivation
        ));
    }

    // Build commit JSON for nix-instantiate
    let is_tip = if commit.prs.iter().all(|(_, commit_type)| *commit_type == CommitType::Normal) {
        "false"
    } else {
        "true"
    };
    let commit_str = format!(
        "{{ commit = \"{}\"; isTip = {}; gitUrl = \"{}\"; cargoNixes = {}; }}",
        commit.git_commit_id, is_tip, repo.path, cargo_nixes
    );

    // Instantiate derivation with cancellation checking
    log::info(format_args!(
        "Instantiating derivation for commit {}",
        commit.git_commit_id
    ));

    let mut child = Command::new("nix-instantiate")
        .args([
            "--show-trace",
            "--arg",
            "inlineJsonConfig",
            &format!(
                "{{ gitDir = \"{}\"; projectName = \"{}\"; }}",
                repo.path, repo.name
            ),
            "--arg",
            "inlineCommitList",
            &format!("[ {} ]", commit_str),
            "--arg",
            "prNum",
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
                            mark_commit_status(db, commit.id, CiStatus::Failed).await?;
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
                        commit.id.apply_update(&tx, &updates).await
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
                    log::warn(
                        &*e.into_boxed_dyn_error(),
                        "Failed to check for cancellation"
                    );
                    continue;
                }

                // Check if we should cancel
                if should_cancel_ci(db, commit).await? {
                    log::info(format_args!("CI cancellation requested for commit {}", commit.git_commit_id));

                    // Kill the child process
                    if let Err(e) = child.kill().await {
                        log::warn(
                            &e,
                            "Failed to kill nix-instantiate process",
                        );
                    }

                    // Wait for it to actually exit
                    let _ = child.wait().await;
                    log::info("Nix instantiation cancelled.");
                    mark_commit_status(db, commit.id, CiStatus::Failed).await?;
                    return Err(anyhow::anyhow!("instantiation cancelled"));
                }
            }
        }
    }
}

async fn build_derivation_with_cancellation(
    db: &mut Db,
    commit: &CommitToTest,
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
                        mark_commit_status(db, commit.id, CiStatus::Failed).await?;
                        return Ok(false);
                    }
                    Err(e) => {
                        log::warn(&e, "Failed to check nix-build status");
                        mark_commit_status(db, commit.id, CiStatus::Failed).await?;
                        return Ok(false);
                    }
                }
            }

            // Check for cancellation every 30 seconds
            _ = interval.tick() => {
                if let Err(e) = check_for_cancellation(db, commit).await {
                    log::warn(&*e.into_boxed_dyn_error(), "Error checking for cancellation");
                    continue;
                }

                // Check if we should cancel
                if should_cancel_ci(db, commit).await? {
                    log::info(format_args!("CI cancellation requested for commit {}", commit.git_commit_id));

                    // Kill the child process
                    if let Err(e) = child.kill().await {
                        log::warn(&e, "Failed to kill nix-build process");
                    }

                    // Wait for it to actually exit
                    let _ = child.wait().await;
                    log::info("nix-build of derivation cancelled.");
                    return Ok(false);
                }
            }
        }
    }
}

#[expect(clippy::case_sensitive_file_extension_comparisons)] // neat lint. complains about looking for .lock files. deliberately violating for now.
async fn find_cargo_lockfiles(
    repo_path: &Path,
    commit_id: &CommitId,
) -> anyhow::Result<Vec<String>> {
    let output = Command::new("git")
        .args(["ls-tree", "-r", "--name-only", commit_id.as_str()])
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

async fn check_has_cargo_toml(repo_path: &Path, commit_id: &CommitId) -> anyhow::Result<bool> {
    let output = Command::new("git")
        .args(["ls-tree", "-r", "--name-only", commit_id.as_str()])
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
    let has_cargo_toml = stdout.lines().any(|line| line.ends_with("Cargo.toml"));

    Ok(has_cargo_toml)
}

async fn check_for_cancellation(db: &mut Db, _commit: &CommitToTest) -> anyhow::Result<()> {
    // Just ensure we can still connect to the database
    let tx = db
        .transaction()
        .await
        .context("starting transaction for cancellation check")?;
    tx.commit()
        .await
        .context("committing cancellation check transaction")?;
    Ok(())
}

async fn should_cancel_ci(db: &mut Db, commit: &CommitToTest) -> anyhow::Result<bool> {
    let tx = db.transaction().await.context("starting transaction")?;

    // Reload the commit to check current status
    let current_commit = Commit::find_by_id(&tx, commit.id)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to reload commit: {}", e))?
        .ok_or_else(|| anyhow::anyhow!("Commit not found: {}", commit.id))?;

    tx.commit().await.context("committing transaction")?;

    // Check if CI should be cancelled
    let should_cancel =
        current_commit.ci_status == CiStatus::Skipped || !current_commit.should_run_ci;

    Ok(should_cancel)
}

async fn save_error_to_file(
    repo_path: &str,
    commit_id: &CommitId,
    operation: &str,
    stdout: &[u8],
    stderr: &[u8],
) -> anyhow::Result<()> {
    let repo_path = Path::new(repo_path);
    let error_dir = repo_path.parent().unwrap_or(repo_path);
    let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
    let filename = format!(
        "local-ci-error-{}-{}-{}.log",
        commit_id, operation, timestamp
    );
    let error_path = error_dir.join(filename);

    fs::write(
        &error_path,
        format!("nix-build failed for commit {}\n\nSTDOUT:\n", commit_id),
    )
    .await
    .with_context(|| {
        format!(
            "Failed to write error file (preamble): {}",
            error_path.display()
        )
    })?;
    fs::write(&error_path, stdout).await.with_context(|| {
        format!(
            "Failed to write error file (stdout): {}",
            error_path.display()
        )
    })?;
    fs::write(&error_path, "\nSTDERR:\n")
        .await
        .with_context(|| {
            format!(
                "Failed to write error file (stderr preamble): {}",
                error_path.display()
            )
        })?;
    fs::write(&error_path, stderr).await.with_context(|| {
        format!(
            "Failed to write error file (stderr): {}",
            error_path.display()
        )
    })?;

    log::info(format_args!(
        "Error details saved to: {}",
        error_path.display()
    ));

    Ok(())
}
