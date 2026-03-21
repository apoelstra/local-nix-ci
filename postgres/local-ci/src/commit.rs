// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Context as _;
use lcilib::{
    Db,
    db::{models::{Repository, Commit, NewCommit, NewRepository, ReviewStatus, CiStatus, PrCommit, PullRequest, CommitType, UpdateCommit}, EntityType, Log},
    git,
    repo,
};
use std::{env, fs, io::{self, Write}};
use xshell::{Shell, cmd};

/// Show information about a commit
/// 
/// # Errors
/// 
/// Returns an error if:
/// - Failed to get current repository information
/// - Git reference resolution fails
/// - Database transaction fails
/// - Repository lookup fails
pub async fn info(commit_ref: &str, db: &mut Db) -> anyhow::Result<()> {
    let shell = Shell::new()?;
    let current_repo = repo::current_repo(&shell)
        .context("failed to get current repository")?;

    // Resolve the reference to a commit hash
    let commit_hash = git::resolve_ref(&shell, commit_ref)
        .with_context(|| format!("failed to resolve reference '{}'. Try 'git fetch' if this is a remote reference.", commit_ref))?;

    let tx = db.transaction().await
        .context("failed to start database transaction")?;

    // Find the repository in the database
    let Some(repo_record) = Repository::find_by_path(&tx, current_repo.repo_root.to_str().unwrap()).await
        .context("failed to query repository")?
    else {
        println!("Repository not found in database. Please run 'refresh' first to initialize it.");
        return Ok(());
    };

    // Look up the commit in the database
    if let Some(commit) = Commit::find_by_git_id(&tx, repo_record.id, &commit_hash).await
        .context("failed to query commit")?
    {
        // Get commit details from git
        let commit_info = git::get_commit_info(&shell, &commit_hash)
            .context("failed to get commit info from git")?;

        println!("{} {}", current_repo.project_name, commit_hash);
        println!("Author: {}", commit_info.author);
        println!("Date: {}", commit_info.date);
        println!();
        println!("{}", commit_info.message);
        println!();
        println!("Diffstat: {}", commit_info.diffstat);
        println!();
        println!("Review Status: {:?}", commit.review_status);
        println!("Should Run CI: {}", commit.should_run_ci);
        println!("CI Status: {:?}", commit.ci_status);
        if let Some(ref derivation) = commit.nix_derivation {
            println!("Nix Derivation: {}", derivation);
        }
        if commit.jj_change_id != format!("unknown-{}", commit_hash) {
            println!("JJ Change ID: {}", commit.jj_change_id);
        }
        println!("Created: {}", commit.created_at);

        // Find PRs containing this commit
        let pr_commits = PrCommit::find_by_commit(&tx, commit.id).await
            .context("failed to find PRs containing this commit")?;

        if !pr_commits.is_empty() {
            println!("\nPull Requests:");
            for pr_commit in pr_commits {
                let pr = PullRequest::find_by_id(&tx, pr_commit.pull_request_id).await
                    .context("failed to get PR details")?
                    .context("PR not found")?;

                let status = match (pr_commit.is_current, pr_commit.commit_type) {
                    (true, CommitType::Normal) => "current".to_string(),
                    (true, commit_type) => format!("{:?}", commit_type).to_lowercase(),
                    (false, CommitType::Normal) => "old".to_string(),
                    (false, commit_type) => format!("old {:?}", commit_type).to_lowercase(),
                };

                println!("  PR #{}: {} ({})", pr.pr_number, pr.title, status);
            }
        }

        // Show recent logs
        let logs = Log::query_for_entities(&tx, &[(EntityType::Commit, commit.id)], None, None).await
            .context("failed to query logs")?;

        if !logs.is_empty() {
            println!("\nRecent Activity (showing last 5, use 'commit log {}' for more):", commit_ref);
            for log in logs.iter().take(5) {
                println!("  {}", log.format_for_display());
            }
        }

        // Show next action
        let next_action = determine_next_action_for_commit(&commit);
        println!("\nNext action: {}", next_action);
    } else {
        println!("Commit {} not found in database.", commit_hash);
        println!("Use 'local-ci refresh commit {}' to add it to the database.", commit_ref);
    }

    tx.commit().await
        .context("failed to commit transaction")?;

    Ok(())
}

/// Determine the next action for a commit
fn determine_next_action_for_commit(commit: &Commit) -> String {
    if commit.review_status == ReviewStatus::Unreviewed {
        format!("local-ci review commit {}", commit.git_commit_id)
    } else {
        "none".to_string()
    }
}

/// Execute the next action for a commit
/// 
/// # Errors
/// 
/// Returns an error if:
/// - Failed to get current repository information
/// - Git reference resolution fails
/// - Database transaction fails
/// - Repository or commit lookup fails
/// - No next action available
pub async fn next(commit_ref: &str, db: &mut Db) -> anyhow::Result<()> {
    let shell = Shell::new()?;
    let current_repo = repo::current_repo(&shell)
        .context("failed to get current repository")?;

    // Resolve the reference to a commit hash
    let commit_hash = git::resolve_ref(&shell, commit_ref)
        .with_context(|| format!("failed to resolve reference '{}'. Try 'git fetch' if this is a remote reference.", commit_ref))?;

    let tx = db.transaction().await
        .context("failed to start database transaction")?;

    // Find the repository in the database
    let Some(repo_record) = Repository::find_by_path(&tx, current_repo.repo_root.to_str().unwrap()).await
        .context("failed to query repository")?
    else {
        anyhow::bail!("Repository not found in database. Please run 'refresh' first to initialize it.");
    };

    // Look up the commit in the database
    let Some(commit) = Commit::find_by_git_id(&tx, repo_record.id, &commit_hash).await
        .context("failed to query commit")?
    else {
        anyhow::bail!("Commit {} not found in database. Use 'local-ci refresh commit {}' to add it to the database.", commit_hash, commit_ref);
    };

    // Check if commit needs review
    if commit.review_status == ReviewStatus::Unreviewed {
        // Drop the transaction since we're going to call review which will start its own
        tx.commit().await
            .context("failed to commit transaction")?;
        
        // Review this commit
        return review(commit_ref, db).await
            .context("failed to review commit");
    }

    // Nothing to do
    tx.commit().await
        .context("failed to commit transaction")?;
    
    println!("Nothing to do");
    Ok(())
}

/// Interactive review of a commit
/// 
/// # Errors
/// 
/// Returns an error if:
/// - Failed to get current repository information
/// - Git reference resolution fails
/// - Database transaction fails
/// - Repository or commit lookup fails
/// - Editor invocation fails
pub async fn review(commit_ref: &str, db: &mut Db) -> anyhow::Result<()> {
    let shell = Shell::new()?;
    let current_repo = repo::current_repo(&shell)
        .context("failed to get current repository")?;

    // Resolve the reference to a commit hash
    let commit_hash = git::resolve_ref(&shell, commit_ref)
        .with_context(|| format!("failed to resolve reference '{}'. Try 'git fetch' if this is a remote reference.", commit_ref))?;

    let tx = db.transaction().await
        .context("failed to start database transaction")?;

    // Find the repository in the database
    let Some(repo_record) = Repository::find_by_path(&tx, current_repo.repo_root.to_str().unwrap()).await
        .context("failed to query repository")?
    else {
        anyhow::bail!("Repository not found in database. Please run 'refresh' first to initialize it.");
    };

    // Look up the commit in the database
    let Some(mut commit) = Commit::find_by_git_id(&tx, repo_record.id, &commit_hash).await
        .context("failed to query commit")?
    else {
        anyhow::bail!("Commit {} not found in database. Use 'local-ci refresh commit {}' to add it to the database.", commit_hash, commit_ref);
    };

    // Show commit info first
    show_commit_info(&shell, &tx, &current_repo, &commit_hash, &commit).await?;

    loop {
        // Show menu
        println!("\nWhat would you like to do?");
        println!("1a) Review and Approve");
        println!("1b) Review and Reject");
        println!();
        println!("2a) View existing review");
        println!("2b) Erase review (mark unreviewed)");
        println!();
        println!("3a) View diff (50-line context)");
        println!("3b) View diff (3-line context)");
        println!("3c) View diff (5000-line context)");
        println!("3d) View diff (stat)");
        println!();
        println!("4) Cancel");
        print!("Choice (1a-4): ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let choice = input.trim();

        match choice {
            "1a" => {
                if let Some(update) = handle_review_with_editor(&commit, ReviewStatus::Approved).await? {
                    Commit::apply_update_by_id(&tx, commit.id, update).await
                        .context("failed to update commit with review")?;
                    println!("Commit review updated and approved.");
                    break;
                }
            }
            "1b" => {
                if let Some(update) = handle_review_with_editor(&commit, ReviewStatus::Rejected).await? {
                    Commit::apply_update_by_id(&tx, commit.id, update).await
                        .context("failed to update commit with review")?;
                    println!("Commit review updated and rejected.");
                    break;
                }
            }
            "2a" => {
                if let Some(ref review_text) = commit.review_text {
                    println!("\nExisting review:");
                    println!("{}", review_text);
                } else {
                    println!("No existing review.");
                }
            }
            "2b" => {
                let update = UpdateCommit {
                    review_text: Some(None),
                    review_status: Some(ReviewStatus::Unreviewed),
                    ..Default::default()
                };
                commit = Commit::apply_update_by_id(&tx, commit.id, update).await
                    .context("failed to erase review")?;
                println!("Review erased and commit marked as unreviewed.");
            }
            "3a" => {
                show_diff(&shell, &commit_hash, Some(50))?;
            }
            "3b" => {
                show_diff(&shell, &commit_hash, Some(3))?;
            }
            "3c" => {
                show_diff(&shell, &commit_hash, Some(5000))?;
            }
            "3d" => {
                show_diff_stat(&shell, &commit_hash)?;
            }
            "4" => {
                println!("Cancelled.");
                break;
            }
            _ => {
                println!("Invalid choice. Please enter 1a, 1b, 2a, 2b, 3a, 3b, 3c, 3d, or 3.");
                continue;
            }
        }
    }

    tx.commit().await
        .context("failed to commit transaction")?;

    Ok(())
}

/// Show commit information (extracted from info function for reuse)
async fn show_commit_info(
    shell: &Shell,
    tx: &lcilib::Transaction<'_>,
    current_repo: &repo::Repository,
    commit_hash: &git::GitCommit,
    commit: &Commit,
) -> anyhow::Result<()> {
    // Get commit details from git
    let commit_info = git::get_commit_info(shell, commit_hash)
        .context("failed to get commit info from git")?;

    println!("{} {}", current_repo.project_name, commit_hash);
    println!("Author: {}", commit_info.author);
    println!("Date: {}", commit_info.date);
    println!();
    println!("{}", commit_info.message);
    println!();
    println!("Diffstat: {}", commit_info.diffstat);
    println!();
    println!("Review Status: {:?}", commit.review_status);
    println!("Should Run CI: {}", commit.should_run_ci);
    println!("CI Status: {:?}", commit.ci_status);
    if let Some(ref derivation) = commit.nix_derivation {
        println!("Nix Derivation: {}", derivation);
    }
    if commit.jj_change_id != format!("unknown-{}", commit_hash) {
        println!("JJ Change ID: {}", commit.jj_change_id);
    }
    println!("Created: {}", commit.created_at);

    // Find PRs containing this commit
    let pr_commits = PrCommit::find_by_commit(tx, commit.id).await
        .context("failed to find PRs containing this commit")?;

    if !pr_commits.is_empty() {
        println!("\nPull Requests:");
        for pr_commit in pr_commits {
            let pr = PullRequest::find_by_id(tx, pr_commit.pull_request_id).await
                .context("failed to get PR details")?
                .context("PR not found")?;

            let status = match (pr_commit.is_current, pr_commit.commit_type) {
                (true, CommitType::Normal) => "current".to_string(),
                (true, commit_type) => format!("{:?}", commit_type).to_lowercase(),
                (false, CommitType::Normal) => "old".to_string(),
                (false, commit_type) => format!("old {:?}", commit_type).to_lowercase(),
            };

            println!("  PR #{}: {} ({})", pr.pr_number, pr.title, status);
        }
    }

    Ok(())
}

/// Handle review with text editor
async fn handle_review_with_editor(
    commit: &Commit,
    new_status: ReviewStatus,
) -> anyhow::Result<Option<UpdateCommit>> {
    let commit_git_id = commit.git_commit_id.clone();
    let existing_review = commit.review_text.clone();
    
    // Move all blocking operations into spawn_blocking
    let result = tokio::task::spawn_blocking(move || -> anyhow::Result<Option<String>> {
        // Create temporary directory and file
        let shell = Shell::new()?;
        let temp_dir = shell.create_temp_dir()
            .context("failed to create temporary directory")?;
        
        let temp_file_path = temp_dir.path().join("review.txt");

        let status_text = match new_status {
            ReviewStatus::Approved => "approved",
            ReviewStatus::Rejected => "rejected",
            ReviewStatus::Unreviewed => "unreviewed",
        };

        let prefill_content = format!(
            "# Enter your review here. Updated commit {} review status: {}\n# Edit the review message above. Lines starting with # will be removed.\n{}",
            commit_git_id,
            status_text,
            existing_review.as_deref().unwrap_or("")
        );

        fs::write(&temp_file_path, prefill_content.as_bytes())
            .context("failed to write to temporary file")?;

        // Get editor from environment or default to vim
        let editor = env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());

        // Launch editor
        let result = cmd!(shell, "{editor} {temp_file_path}")
            .run()
            .with_context(|| format!("failed to launch editor: {}", editor));

        if result.is_err() {
            println!("Editor exited with error, review update cancelled.");
            return Ok(None);
        }

        // Read the edited content
        let content = fs::read_to_string(&temp_file_path)
            .context("failed to read edited file")?;

        // Remove lines starting with #
        let review_text: String = content
            .lines()
            .filter(|line| !line.trim_start().starts_with('#'))
            .collect::<Vec<_>>()
            .join("\n");

        Ok(Some(review_text))
    }).await
    .context("failed to execute blocking editor operation")?;

    match result? {
        Some(review_text) => {
            // Create update struct
            let update = UpdateCommit {
                review_text: Some(Some(review_text)),
                review_status: Some(new_status),
                ..Default::default()
            };
            Ok(Some(update))
        }
        None => Ok(None),
    }
}

/// Show git diff with specified context
fn show_diff(shell: &Shell, commit_hash: &git::GitCommit, context: Option<u32>) -> anyhow::Result<()> {
    if let Some(lines) = context {
        let lines = lines.to_string();
        cmd!(shell, "git show --unified={lines} {commit_hash}")
            .run()
            .context("failed to run git show")?;
    } else {
        cmd!(shell, "git show {commit_hash}")
            .run()
            .context("failed to run git show")?;
    }

    Ok(())
}

/// Show git diff stat
fn show_diff_stat(shell: &Shell, commit_hash: &git::GitCommit) -> anyhow::Result<()> {
    cmd!(shell, "git show --stat {commit_hash}")
        .run()
        .context("failed to run git show --stat")?;

    Ok(())
}

/// Show logs for a commit
/// 
/// # Errors
/// 
/// Returns an error if:
/// - Failed to get current repository information
/// - Git reference resolution fails
/// - Database transaction fails
/// - Repository or commit lookup fails
/// - Date parsing fails
pub async fn log(commit_ref: &str, since: Option<&str>, until: Option<&str>, db: &mut Db) -> anyhow::Result<()> {
    let shell = Shell::new()?;
    let current_repo = repo::current_repo(&shell)
        .context("failed to get current repository")?;

    // Resolve the reference to a commit hash
    let commit_hash = git::resolve_ref(&shell, commit_ref)
        .with_context(|| format!("failed to resolve reference '{}'. Try 'git fetch' if this is a remote reference.", commit_ref))?;

    let tx = db.transaction().await
        .context("failed to start database transaction")?;

    // Find the repository in the database
    let Some(repo_record) = Repository::find_by_path(&tx, current_repo.repo_root.to_str().unwrap()).await
        .context("failed to query repository")?
    else {
        anyhow::bail!("Repository not found in database. Please run 'refresh' first to initialize it.");
    };

    // Look up the commit in the database
    let Some(commit) = Commit::find_by_git_id(&tx, repo_record.id, &commit_hash).await
        .context("failed to query commit")?
    else {
        anyhow::bail!("Commit {} not found in database. Use 'local-ci refresh commit {}' to add it to the database.", commit_hash, commit_ref);
    };

    // Query logs for this commit
    let logs = Log::query_for_entities(&tx, &[(EntityType::Commit, commit.id)], since, until).await
        .context("failed to query logs")?;

    // Display the logs
    for log in logs {
        println!("{}", log.format_for_display());
    }

    tx.commit().await
        .context("failed to commit transaction")?;

    Ok(())
}

/// Refresh a commit from the local git repository
/// 
/// # Errors
/// 
/// Returns an error if:
/// - Failed to get current repository information
/// - Git reference resolution fails
/// - Database transaction or operations fail
pub async fn refresh(commit_ref: &str, db: &mut Db) -> anyhow::Result<()> {
    let shell = Shell::new()?;
    let current_repo = repo::current_repo(&shell)
        .context("failed to get current repository")?;

    // Resolve the reference to a commit hash
    let commit_hash = git::resolve_ref(&shell, commit_ref)
        .with_context(|| format!("failed to resolve reference '{}'. Try 'git fetch' if this is a remote reference.", commit_ref))?;

    // Get commit details from git
    let commit_info = git::get_commit_info(&shell, &commit_hash)
        .context("failed to get commit info from git")?;

    // Start database transaction
    let tx = db.transaction().await
        .context("failed to start database transaction")?;

    // Find or create the repository record
    let repo_record = if let Some(repo) = Repository::find_by_path(&tx, current_repo.repo_root.to_str().unwrap()).await
        .context("failed to query repository")? 
    {
        repo
    } else {
        // Create repository record
        let new_repo = NewRepository {
            name: current_repo.project_name.clone(),
            path: current_repo.repo_root.to_str().unwrap().to_string(),
            nixfile_path: "default.nix".to_string(), // Default, can be configured later
        };
        Repository::create(&tx, new_repo).await
            .context("failed to create repository record")?
    };

    // Check if commit already exists
    if let Some(existing_commit) = Commit::find_by_git_id(&tx, repo_record.id, &commit_hash).await
        .context("failed to query commit")?
    {
        println!("Commit {} already exists in database.", commit_hash);
        println!("Created: {}", existing_commit.created_at);
        println!("Review Status: {:?}", existing_commit.review_status);
        println!("CI Status: {:?}", existing_commit.ci_status);
    } else {
        // Create new commit record
        let new_commit = NewCommit {
            repository_id: repo_record.id,
            git_commit_id: commit_hash.clone(),
            jj_change_id: format!("unknown-{}", commit_hash), // Will be updated later when we have jj integration
            review_status: ReviewStatus::Unreviewed,
            should_run_ci: true,
            ci_status: CiStatus::Unstarted,
            nix_derivation: None,
            review_text: None,
        };

        let commit_record = Commit::create(&tx, new_commit).await
            .context("failed to create commit record")?;

        println!("Successfully added commit {} to database.", commit_hash);
        println!("Author: {}", commit_info.author);
        println!("Date: {}", commit_info.date);
        println!("Message: {}", commit_info.message.lines().next().unwrap_or("(no message)"));
        println!("Database ID: {}", commit_record.id);
        println!();
        println!("Note: If this commit is part of a PR, use 'local-ci refresh pr <number>' instead");
        println!("to properly associate it with the pull request.");
    }

    // Commit the transaction
    tx.commit().await
        .context("failed to commit transaction")?;

    Ok(())
}
