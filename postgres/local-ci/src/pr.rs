// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Context as _;
use lcilib::{
    Db,
    db::models::{Repository, PullRequest, NewRepository, Commit, CommitType, NewCommit, ReviewStatus, CiStatus, UpdatePullRequest, NewPullRequest},
    gh,
    git,
    repo,
};
use xshell::Shell;

/// Show information about a PR
/// 
/// # Errors
/// 
/// Returns an error if:
/// - Failed to get current repository information
/// - Database transaction fails
/// - Repository or PR lookup fails
pub async fn info(pr_number: usize, db: &mut Db) -> anyhow::Result<()> {
    let shell = Shell::new()?;
    let current_repo = repo::current_repo(&shell)
        .context("failed to get current repository")?;

    let tx = db.transaction().await
        .context("failed to start database transaction")?;

    // Find the repository in the database
    let Some(repo_record) = Repository::find_by_path(&tx, current_repo.repo_root.to_str().unwrap()).await
        .context("failed to query repository")?
    else {
        println!("Repository not found in database. Please run 'refresh' first to initialize it.");
        return Ok(());
    };

    // Look up the PR in the database
    if let Some(pr) = PullRequest::find_by_number(&tx, repo_record.id, pr_number.try_into()?).await
        .context("failed to query pull request")?
    {
        println!("PR #{}", pr.pr_number);
        println!("Title: {}", pr.title);
        println!("Body: {}", if pr.body.is_empty() { "(empty)" } else { &pr.body });
        println!("Review Status: {:?}", pr.review_status);
        println!("Priority: {}", pr.priority);
        println!("OK to Merge: {}", pr.ok_to_merge);
        println!("Required Reviewers: {}", pr.required_reviewers);
        println!("Created: {}", pr.created_at);
        println!("Updated: {}", pr.updated_at);
        println!("Last Synced: {}", pr.synced_at);

        // Show commits
        let commits = pr.get_commits(&tx).await
            .context("failed to get PR commits")?;
        
        if !commits.is_empty() {
            println!("\nCommits:");
            for (i, commit) in commits.iter().enumerate() {
                println!("  {}. {} ({})", i + 1, commit.git_commit_id, commit.commit_type.as_str());
            }
        }
    } else {
        println!("PR #{} not found in database.", pr_number);
        println!("Use 'local-ci refresh pr {}' to download it from GitHub.", pr_number);
    }

    tx.commit().await
        .context("failed to commit transaction")?;

    Ok(())
}

/// Refresh a PR from GitHub
/// 
/// # Errors
/// 
/// Returns an error if:
/// - Failed to get current repository information
/// - GitHub API call fails or PR not found
/// - Git fetch of head commit fails
/// - Database transaction or operations fail
/// - PR has no commits
pub async fn refresh(pr_number: usize, db: &mut Db) -> anyhow::Result<()> {
    let shell = Shell::new()?;
    let current_repo = repo::current_repo(&shell)
        .context("failed to get current repository")?;

    // Fetch PR info from GitHub
    let pr_info = gh::get_pr_info(&shell, pr_number)
        .context("failed to fetch PR from GitHub")?;

    // Fetch the head commit to ensure it's available locally
    git::fetch_commit(&shell, &pr_info.head_commit)
        .context("failed to fetch head commit")?;

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

    // Create or find commits for all commits in the PR
    let mut commit_records = Vec::new();
    for (i, commit_oid) in pr_info.commit_ids().enumerate() {
        let commit_record = if let Some(commit) = Commit::find_by_git_id(&tx, repo_record.id, &commit_oid.to_string()).await
            .context("failed to query commit")? 
        {
            commit
        } else {
            // Create new commit record
            let commit_type = if i == pr_info.commits.len() - 1 {
                CommitType::Tip
            } else {
                CommitType::Normal
            };

            let new_commit = NewCommit {
                repository_id: repo_record.id,
                git_commit_id: commit_oid.to_string(),
                jj_change_id: format!("unknown-{}", commit_oid), // Will be updated later when we have jj integration
                review_status: ReviewStatus::Unreviewed,
                should_run_ci: false,
                ci_status: CiStatus::Unstarted,
                commit_type,
                nix_derivation: None,
            };

            Commit::create(&tx, new_commit).await
                .context("failed to create commit record")?
        };
        commit_records.push(commit_record);
    }

    // Find the tip commit (last commit in the list)
    let tip_commit = commit_records.last()
        .context("PR has no commits")?;

    // Create or update the PR record
    let pr_record = if let Some(pr) = PullRequest::find_by_number(&tx, repo_record.id, pr_number.try_into()?).await
        .context("failed to query pull request")?
    {
        // Update existing PR
        let updates = UpdatePullRequest {
            title: Some(pr_info.title.clone()),
            body: Some(pr_info.body.clone()),
            tip_commit_id: Some(tip_commit.id),
            ..Default::default()
        };
        pr.update(&tx, updates).await
            .context("failed to update pull request")?
    } else {
        // Create new PR
        let new_pr = NewPullRequest {
            repository_id: repo_record.id,
            pr_number: pr_number.try_into()?,
            title: pr_info.title.clone(),
            body: pr_info.body.clone(),
            tip_commit_id: tip_commit.id,
            review_status: ReviewStatus::Unreviewed,
            priority: 0,
            ok_to_merge: true,
            required_reviewers: 1,
        };
        PullRequest::create(&tx, new_pr).await
            .context("failed to create pull request")?
    };

    // Clear existing PR-commit relationships and recreate them
    tx.execute(
        "DELETE FROM pr_commits WHERE pull_request_id = $1",
        &[&pr_record.id],
    ).await
        .context("failed to clear existing PR commits")?;

    // Add all commits to the PR in order
    for (i, commit) in commit_records.iter().enumerate() {
        pr_record.add_commit(&tx, commit.id, i.try_into()?).await
            .context("failed to add commit to PR")?;
    }

    // Commit the transaction
    tx.commit().await
        .context("failed to commit transaction")?;

    println!("Successfully refreshed PR #{}", pr_number);
    println!("Title: {}", pr_info.title);
    println!("Commits: {}", commit_records.len());
    println!("Head commit: {}", pr_info.head_commit);

    Ok(())
}
