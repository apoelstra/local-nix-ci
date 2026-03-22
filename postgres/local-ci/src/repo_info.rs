// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Context as _;
use lcilib::{
    Db,
    db::models::{Ack, AckStatus, CiStatus, Commit, PullRequest, Repository, ReviewStatus, Stack},
    repo,
};
use xshell::{Shell, cmd};

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
    let current_repo = repo::current_repo(&shell).context("failed to get current repository")?;

    let tx = db
        .transaction()
        .await
        .context("failed to start database transaction")?;

    // Find the repository in the database
    let Some(repo_record) = Repository::find_by_path(&tx, current_repo.repo_root.to_str().unwrap())
        .await
        .context("failed to query repository")?
    else {
        println!("Repository not found in database. Please run 'refresh' first to initialize it.");
        return Ok(());
    };

    println!("Repository: {} ({})", repo_record.name, repo_record.path);
    println!("Nixfile: {}", repo_record.nixfile_path);
    println!("Created: {}", repo_record.created_at);
    println!();

    // Get all PRs for this repository
    let all_prs = repo_record
        .id
        .get_current_pull_requests(&tx)
        .await
        .context("failed to get PRs for repository")?;

    // Get all commits for this repository
    let all_commits = Commit::find_by_repository(&tx, repo_record.id)
        .await
        .context("failed to get commits for repository")?;

    // Get all ACKs for PRs in this repository
    let mut all_acks = Vec::new();
    for pr in &all_prs {
        let pr_acks = Ack::find_by_pull_request(&tx, pr.id)
            .await
            .context("failed to get ACKs for PR")?;
        all_acks.extend(pr_acks);
    }

    // Get all stacks for this repository
    let all_stacks = repo_record
        .id
        .get_stacks(&tx)
        .await
        .context("failed to get stacks for repository")?;

    show_prs(&all_prs);
    show_stacks(&tx, &all_stacks).await?;
    show_pending_actions(&all_prs, &all_commits, &all_acks);

    // Display CI status
    show_ci_status(&all_commits);

    tx.commit().await.context("failed to commit transaction")?;

    Ok(())
}

/// Display PRs organized by status
fn show_prs(prs: &[PullRequest]) {
    println!("=== Pull Requests by Status ===");

    // Ready to merge
    let ready_to_merge: Vec<_> = prs
        .iter()
        .filter(|pr| pr.review_status == ReviewStatus::Approved && pr.ok_to_merge)
        .collect();

    if !ready_to_merge.is_empty() {
        println!("Ready to Merge ({}):", ready_to_merge.len());
        for pr in ready_to_merge {
            println!(
                "  PR #{}: {} (priority: {})",
                pr.pr_number, pr.title, pr.priority
            );
        }
        println!();
    }

    // .await review
    let needs_review: Vec<_> = prs
        .iter()
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
    let rejected: Vec<_> = prs
        .iter()
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
    let approved_not_ready: Vec<_> = prs
        .iter()
        .filter(|pr| pr.review_status == ReviewStatus::Approved && !pr.ok_to_merge)
        .collect();

    if !approved_not_ready.is_empty() {
        println!(
            "Approved but Not Ready to Merge ({}):",
            approved_not_ready.len()
        );
        for pr in approved_not_ready {
            println!("  PR #{}: {}", pr.pr_number, pr.title);
        }
    }
}

/// Display PRs organized by status
async fn show_stacks(tx: &lcilib::Transaction<'_>, stacks: &[Stack]) -> anyhow::Result<()> {
    if stacks.is_empty() {
        println!("\nNo stacks.");
        return Ok(());
    }
    println!("\n=== Merge Stacks ===");

    for stack in stacks {
        let repo = Repository::find_by_id(tx, stack.repository_id)
            .await?
            .expect("repo exists");

        let commits = stack.id.get_commits(tx).await?;
        let ids: Vec<_> = commits
            .iter()
            .map(|commit| commit.git_commit_id.as_str())
            .collect();
        let revset = ids.join("|");

        println!("Stack {}: {} commits", stack.id, commits.len());
        tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let shell = Shell::new()?;
            let _guard = shell.push_dir(&repo.path);

            cmd!(shell, "jj log --no-pager -r {revset}").quiet().run()?;
            Ok(())
        })
        .await??;
    }

    Ok(())
}

/// Display pending actions that need attention
fn show_pending_actions(prs: &[PullRequest], commits: &[Commit], acks: &[Ack]) {
    println!("\n=== Pending Actions ===");

    let mut has_pending = false;

    // Pending ACKs
    let pending_acks: Vec<_> = acks
        .iter()
        .filter(|ack| ack.status == AckStatus::Pending)
        .collect();

    if !pending_acks.is_empty() {
        has_pending = true;
        println!("ACKs Pending ({}):", pending_acks.len());
        for ack in pending_acks {
            // Find the PR for this ACK
            if let Some(pr) = prs.iter().find(|pr| pr.id == ack.pull_request_id) {
                println!(
                    "  PR #{}: {} by {}",
                    pr.pr_number, ack.message, ack.reviewer_name
                );
            }
        }
        println!();
    }

    // Failed ACKs
    let failed_acks: Vec<_> = acks
        .iter()
        .filter(|ack| ack.status == AckStatus::Failed)
        .collect();

    if !failed_acks.is_empty() {
        has_pending = true;
        println!("ACKs Failed ({}):", failed_acks.len());
        for ack in failed_acks {
            if let Some(pr) = prs.iter().find(|pr| pr.id == ack.pull_request_id) {
                println!(
                    "  PR #{}: {} by {}",
                    pr.pr_number, ack.message, ack.reviewer_name
                );
            }
        }
        println!();
    }

    // Commits needing CI
    let ci_needed: Vec<_> = commits
        .iter()
        .filter(|c| c.should_run_ci && c.ci_status == CiStatus::Unstarted)
        .collect();

    if !ci_needed.is_empty() {
        has_pending = true;
        println!("Commits Needing CI ({}):", ci_needed.len());
        for commit in ci_needed.iter().take(10) {
            // Limit to first 10
            println!(
                "  {}: {}",
                commit.git_commit_id.prefix8(),
                commit
                    .review_text
                    .as_deref()
                    .unwrap_or("(no review text)")
                    .lines()
                    .next()
                    .unwrap_or("")
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
    println!("\n=== CI Status ===");

    // CI failures
    let ci_failed: Vec<_> = commits
        .iter()
        .filter(|c| c.ci_status == CiStatus::Failed)
        .collect();

    if !ci_failed.is_empty() {
        println!("CI Failures ({}):", ci_failed.len());
        for commit in ci_failed.iter().take(10) {
            // Limit to first 10
            println!(
                "  {}: {}",
                commit.git_commit_id.prefix8(),
                commit
                    .review_text
                    .as_deref()
                    .unwrap_or("(no review text)")
                    .lines()
                    .next()
                    .unwrap_or("")
            );
        }
        if ci_failed.len() > 10 {
            println!("  ... and {} more", ci_failed.len() - 10);
        }
        println!();
    }

    // Recent CI passes (last 5)
    let ci_passed: Vec<_> = commits
        .iter()
        .filter(|c| c.ci_status == CiStatus::Passed)
        .collect();

    if !ci_passed.is_empty() {
        println!("Recent CI Passes (showing last 5):");
        for commit in ci_passed.iter().take(5) {
            println!(
                "  {}: {}",
                commit.git_commit_id.prefix8(),
                commit
                    .review_text
                    .as_deref()
                    .unwrap_or("(no review text)")
                    .lines()
                    .next()
                    .unwrap_or("")
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
