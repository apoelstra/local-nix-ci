// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Context as _;
use lcilib::{
    Db,
    db::{models::{Repository, Commit, NewCommit, NewRepository, ReviewStatus, CiStatus, PrCommit, PullRequest, CommitType}, EntityType, Log},
    git,
    repo,
};
use xshell::Shell;

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
    } else {
        println!("Commit {} not found in database.", commit_hash);
        println!("Use 'local-ci refresh commit {}' to add it to the database.", commit_ref);
    }

    tx.commit().await
        .context("failed to commit transaction")?;

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
