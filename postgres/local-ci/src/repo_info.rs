// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Context as _;
use lcilib::{
    Db,
    db::models::{Repository, PullRequest, Commit, Ack, AckStatus, ReviewStatus, CiStatus},
    repo,
};
use xshell::Shell;

/// Show overview of all PRs and commits in the current repository
/// 
/// # Errors
/// 
/// Returns an error if:
/// - Failed to get current repository information
/// - Database transaction fails
/// - Repository lookup fails
pub async fn overview(db: &mut Db) -> anyhow::Result<()> {
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

    println!("Repository: {} ({})", repo_record.name, repo_record.path);
    println!("Created: {}", repo_record.created_at);
    println!();

    // Get all PRs for this repository
    let all_prs = get_all_prs_for_repo(&tx, repo_record.id).await
        .context("failed to get PRs for repository")?;

    // Get all commits for this repository
    let all_commits = Commit::find_by_repository(&tx, repo_record.id).await
        .context("failed to get commits for repository")?;

    // Get all ACKs for PRs in this repository
    let mut all_acks = Vec::new();
    for pr in &all_prs {
        let pr_acks = Ack::find_by_pull_request(&tx, pr.id).await
            .context("failed to get ACKs for PR")?;
        all_acks.extend(pr_acks);
    }

    // Display repository summary
    show_repository_summary(&all_prs, &all_commits, &all_acks);

    // Display PRs by status
    show_prs_by_status(&all_prs);

    // Display pending actions
    show_pending_actions(&all_prs, &all_commits, &all_acks);

    // Display CI status
    show_ci_status(&all_commits);

    tx.commit().await
        .context("failed to commit transaction")?;

    Ok(())
}

/// Get all PRs for a repository (helper function since it's not in the operations)
async fn get_all_prs_for_repo(
    tx: &lcilib::Transaction<'_>,
    repository_id: i32,
) -> anyhow::Result<Vec<PullRequest>> {
    let rows = tx
        .query(
            r#"
            SELECT id, repository_id, pr_number, title, body, author_login, target_branch, tip_commit_id, merge_status, review_status, 
                   priority, ok_to_merge, required_reviewers, created_at, updated_at, synced_at
            FROM pull_requests WHERE repository_id = $1 ORDER BY pr_number DESC
            "#,
            &[&repository_id],
        )
        .await
        .context("failed to query pull requests")?;

    Ok(rows.iter().map(|row| PullRequest {
        id: row.get("id"),
        repository_id: row.get("repository_id"),
        pr_number: row.get("pr_number"),
        title: row.get("title"),
        body: row.get("body"),
        author_login: row.get("author_login"),
        target_branch: row.get("target_branch"),
        tip_commit_id: row.get("tip_commit_id"),
        merge_status: row.get("merge_status"),
        review_status: row.get("review_status"),
        priority: row.get("priority"),
        ok_to_merge: row.get("ok_to_merge"),
        required_reviewers: row.get("required_reviewers"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        synced_at: row.get("synced_at"),
    }).collect())
}

/// Display repository summary statistics
fn show_repository_summary(prs: &[PullRequest], commits: &[Commit], acks: &[Ack]) {
    println!("=== Repository Summary ===");
    println!("Pull Requests: {}", prs.len());
    println!("Commits: {}", commits.len());
    println!("ACKs: {}", acks.len());
    
    // Count PRs by status
    let ready_to_merge = prs.iter().filter(|pr| pr.review_status == ReviewStatus::Approved && pr.ok_to_merge).count();
    let needs_review = prs.iter().filter(|pr| pr.review_status == ReviewStatus::Unreviewed).count();
    let rejected = prs.iter().filter(|pr| pr.review_status == ReviewStatus::Rejected).count();
    
    println!("  - Ready to merge: {}", ready_to_merge);
    println!("  - Needs review: {}", needs_review);
    println!("  - Rejected: {}", rejected);
    
    // Count commits by CI status
    let ci_needed = commits.iter().filter(|c| c.should_run_ci && c.ci_status == CiStatus::Unstarted).count();
    let ci_failed = commits.iter().filter(|c| c.ci_status == CiStatus::Failed).count();
    let ci_passed = commits.iter().filter(|c| c.ci_status == CiStatus::Passed).count();
    
    println!("  - Commits needing CI: {}", ci_needed);
    println!("  - CI failures: {}", ci_failed);
    println!("  - CI passed: {}", ci_passed);
    
    println!();
}

/// Display PRs organized by status
fn show_prs_by_status(prs: &[PullRequest]) {
    println!("=== Pull Requests by Status ===");
    
    // Ready to merge
    let ready_to_merge: Vec<_> = prs.iter()
        .filter(|pr| pr.review_status == ReviewStatus::Approved && pr.ok_to_merge)
        .collect();
    
    if !ready_to_merge.is_empty() {
        println!("Ready to Merge ({}):", ready_to_merge.len());
        for pr in ready_to_merge {
            println!("  PR #{}: {} (priority: {})", pr.pr_number, pr.title, pr.priority);
        }
        println!();
    }
    
    // Needs review
    let needs_review: Vec<_> = prs.iter()
        .filter(|pr| pr.review_status == ReviewStatus::Unreviewed)
        .collect();
    
    if !needs_review.is_empty() {
        println!("Needs Review ({}):", needs_review.len());
        for pr in needs_review {
            println!("  PR #{}: {}", pr.pr_number, pr.title);
        }
        println!();
    }
    
    // Rejected
    let rejected: Vec<_> = prs.iter()
        .filter(|pr| pr.review_status == ReviewStatus::Rejected)
        .collect();
    
    if !rejected.is_empty() {
        println!("Rejected/Needs Changes ({}):", rejected.len());
        for pr in rejected {
            println!("  PR #{}: {}", pr.pr_number, pr.title);
        }
        println!();
    }
    
    // Approved but not ready to merge
    let approved_not_ready: Vec<_> = prs.iter()
        .filter(|pr| pr.review_status == ReviewStatus::Approved && !pr.ok_to_merge)
        .collect();
    
    if !approved_not_ready.is_empty() {
        println!("Approved but Not Ready to Merge ({}):", approved_not_ready.len());
        for pr in approved_not_ready {
            println!("  PR #{}: {}", pr.pr_number, pr.title);
        }
        println!();
    }
}

/// Display pending actions that need attention
fn show_pending_actions(prs: &[PullRequest], commits: &[Commit], acks: &[Ack]) {
    println!("=== Pending Actions ===");
    
    let mut has_pending = false;
    
    // Pending ACKs
    let pending_acks: Vec<_> = acks.iter()
        .filter(|ack| ack.status == AckStatus::Pending)
        .collect();
    
    if !pending_acks.is_empty() {
        has_pending = true;
        println!("ACKs Pending ({}):", pending_acks.len());
        for ack in pending_acks {
            // Find the PR for this ACK
            if let Some(pr) = prs.iter().find(|pr| pr.id == ack.pull_request_id) {
                println!("  PR #{}: {} by {}", pr.pr_number, ack.message, ack.reviewer_name);
            }
        }
        println!();
    }
    
    // Failed ACKs
    let failed_acks: Vec<_> = acks.iter()
        .filter(|ack| ack.status == AckStatus::Failed)
        .collect();
    
    if !failed_acks.is_empty() {
        has_pending = true;
        println!("ACKs Failed ({}):", failed_acks.len());
        for ack in failed_acks {
            if let Some(pr) = prs.iter().find(|pr| pr.id == ack.pull_request_id) {
                println!("  PR #{}: {} by {}", pr.pr_number, ack.message, ack.reviewer_name);
            }
        }
        println!();
    }
    
    // Commits needing CI
    let ci_needed: Vec<_> = commits.iter()
        .filter(|c| c.should_run_ci && c.ci_status == CiStatus::Unstarted)
        .collect();
    
    if !ci_needed.is_empty() {
        has_pending = true;
        println!("Commits Needing CI ({}):", ci_needed.len());
        for commit in ci_needed.iter().take(10) { // Limit to first 10
            println!("  {}: {}", 
                commit.git_commit_id.prefix8(), 
                commit.review_text.as_deref().unwrap_or("(no review text)").lines().next().unwrap_or("")
            );
        }
        if ci_needed.len() > 10 {
            println!("  ... and {} more", ci_needed.len() - 10);
        }
        println!();
    }
    
    if !has_pending {
        println!("No pending actions! 🎉");
        println!();
    }
}

/// Display CI status overview
fn show_ci_status(commits: &[Commit]) {
    println!("=== CI Status ===");
    
    // CI failures
    let ci_failed: Vec<_> = commits.iter()
        .filter(|c| c.ci_status == CiStatus::Failed)
        .collect();
    
    if !ci_failed.is_empty() {
        println!("CI Failures ({}):", ci_failed.len());
        for commit in ci_failed.iter().take(10) { // Limit to first 10
            println!("  {}: {}", 
                commit.git_commit_id.prefix8(), 
                commit.review_text.as_deref().unwrap_or("(no review text)").lines().next().unwrap_or("")
            );
        }
        if ci_failed.len() > 10 {
            println!("  ... and {} more", ci_failed.len() - 10);
        }
        println!();
    }
    
    // Recent CI passes (last 5)
    let ci_passed: Vec<_> = commits.iter()
        .filter(|c| c.ci_status == CiStatus::Passed)
        .collect();
    
    if !ci_passed.is_empty() {
        println!("Recent CI Passes (showing last 5):");
        for commit in ci_passed.iter().take(5) {
            println!("  {}: {}", 
                commit.git_commit_id.prefix8(), 
                commit.review_text.as_deref().unwrap_or("(no review text)").lines().next().unwrap_or("")
            );
        }
        if ci_passed.len() > 5 {
            println!("  ... and {} more passed", ci_passed.len() - 5);
        }
        println!();
    }
    
    if ci_failed.is_empty() && ci_passed.is_empty() {
        println!("No CI results yet.");
        println!();
    }
}
