// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Context as _;
use lcilib::{
    Db,
    db::{models::{Repository, PullRequest, NewRepository, Commit, CommitType, NewCommit, ReviewStatus, CiStatus, UpdatePullRequest, NewPullRequest, PrCommit}, EntityType, Log},
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
        println!("{} PR #{}: {}", current_repo.project_name, pr.pr_number, pr.title);
        println!();
        println!("{}", if pr.body.is_empty() { "(empty)" } else { &pr.body });
        println!();
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
            for (i, (commit, commit_type)) in commits.iter().enumerate() {
                println!("  {}. {} ({:?})", i + 1, commit.git_commit_id, commit_type);
            }
        }

        // Show previous tips
        let previous_tips = pr.get_previous_tips(&tx).await
            .context("failed to get previous tip commits")?;
        
        if !previous_tips.is_empty() {
            println!("\nPrevious tip commits:");
            for tip in previous_tips.iter() {
                println!("  {}", tip.git_commit_id);
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

/// Show logs for a PR and its commits
/// 
/// # Errors
/// 
/// Returns an error if:
/// - Failed to get current repository information
/// - Database transaction fails
/// - Repository or PR lookup fails
/// - Date parsing fails
pub async fn log(pr_number: usize, since: Option<&str>, until: Option<&str>, db: &mut Db) -> anyhow::Result<()> {
    let shell = Shell::new()?;
    let current_repo = repo::current_repo(&shell)
        .context("failed to get current repository")?;

    let tx = db.transaction().await
        .context("failed to start database transaction")?;

    // Find the repository in the database
    let Some(repo_record) = Repository::find_by_path(&tx, current_repo.repo_root.to_str().unwrap()).await
        .context("failed to query repository")?
    else {
        anyhow::bail!("Repository not found in database. Please run 'refresh' first to initialize it.");
    };

    // Look up the PR in the database
    let Some(pr) = PullRequest::find_by_number(&tx, repo_record.id, pr_number.try_into()?).await
        .context("failed to query pull request")?
    else {
        anyhow::bail!("PR #{} not found in database. Use 'local-ci refresh pr {}' to download it from GitHub.", pr_number, pr_number);
    };

    // Get all commits associated with this PR
    let commits = pr.get_commits(&tx).await
        .context("failed to get PR commits")?;

    // Build list of entities to query logs for (PR + all its commits)
    let mut entities = vec![(EntityType::PullRequest, pr.id)];
    for (commit, _) in &commits {
        entities.push((EntityType::Commit, commit.id));
    }

    // Query logs for all entities
    let logs = Log::query_for_entities(&tx, &entities, since, until).await
        .context("failed to query logs")?;

    // Display the logs
    for log in logs {
        println!("{}", log.format_for_display());
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
    for commit_oid in pr_info.commit_ids() {
        let commit_record = if let Some(commit) = Commit::find_by_git_id(&tx, repo_record.id, commit_oid).await
            .context("failed to query commit")? 
        {
            commit
        } else {
            // Create new commit record
            let new_commit = NewCommit {
                repository_id: repo_record.id,
                git_commit_id: commit_oid.clone(),
                jj_change_id: format!("unknown-{}", commit_oid), // Will be updated later when we have jj integration
                review_status: ReviewStatus::Unreviewed,
                should_run_ci: false,
                ci_status: CiStatus::Unstarted,
                nix_derivation: None,
                review_text: None,
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

    // Implement the new refresh logic
    use std::collections::{HashMap, HashSet};

    // Get existing pr_commits for this PR
    let existing_pr_commits = PrCommit::find_by_pr(&tx, pr_record.id).await
        .context("failed to get existing PR commits")?;

    // Build maps for efficient lookup
    let mut existing_by_commit_id: HashMap<i32, PrCommit> = HashMap::new();
    for pr_commit in existing_pr_commits {
        existing_by_commit_id.insert(pr_commit.commit_id, pr_commit);
    }

    // Build the new state we want
    let mut new_commit_ids = HashSet::new();
    let mut updates_needed = Vec::new();
    
    for (i, commit) in commit_records.iter().enumerate() {
        new_commit_ids.insert(commit.id);
        
        let new_sequence = (i + 1) as i32;
        let new_commit_type = if i == commit_records.len() - 1 {
            CommitType::Tip
        } else {
            CommitType::Normal
        };
        
        if let Some(existing) = existing_by_commit_id.get(&commit.id) {
            // Check if we need to update this record
            let needs_update = existing.sequence_order != new_sequence
                || existing.commit_type != new_commit_type
                || !existing.is_current;
                
            if needs_update {
                updates_needed.push((
                    existing.id,
                    Some(new_sequence),
                    Some(new_commit_type),
                    Some(true),
                ));
            }
        } else {
            // This is a new commit, insert it
            PrCommit::create(&tx, pr_record.id, commit.id, new_sequence, new_commit_type).await
                .context("failed to create new pr_commit record")?;
        }
    }

    // Mark commits that are no longer in the PR as not current
    for (commit_id, existing) in &existing_by_commit_id {
        if !new_commit_ids.contains(commit_id) && existing.is_current {
            updates_needed.push((existing.id, None, None, Some(false)));
        }
    }

    // Apply all updates
    for (id, sequence_order, commit_type, is_current) in updates_needed {
        PrCommit::update_status(&tx, id, sequence_order, commit_type, is_current).await
            .context("failed to update pr_commit record")?;
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
