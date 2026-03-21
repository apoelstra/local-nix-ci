use anyhow::Context as _;
use core::fmt;
use lcilib::{Db, db::models::{Commit, PullRequest, Stack, Repository}};
use std::collections::HashMap;
use chrono::Utc;
use tokio::task;

/// Find the next commit that needs testing, following the priority rules
/// 
/// # Errors
/// 
/// Returns an error if database operations fail.
pub async fn find_next_commit_to_test(db: &mut Db) -> anyhow::Result<Option<Commit>> {
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
        let priority = calculate_stack_priority(commits, &tx, &repo_map).await
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
            log_info("Found commit from high-priority stack");
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
            log_info("Found commit from PR");
            tx.commit().await.context("committing transaction")?;
            return Ok(Some(commit));
        }
    }

    // 3. Check low-priority stacks (negative priority or conflicting)
    let mut low_priority_with_scores = Vec::new();
    for (stack, commits) in &low_priority_stacks {
        let priority = calculate_stack_priority(commits, &tx, &repo_map).await
            .context("calculating low-priority stack priority")?;
        low_priority_with_scores.push((stack, commits, priority));
    }
    
    // Sort by priority (highest first, even if negative)
    low_priority_with_scores.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    
    for (stack, _commits, _priority) in &low_priority_with_scores {
        if let Some(commit) = stack.get_next_untested_commit(&tx).await
            .context("getting next untested commit from low-priority stack")? {
            log_info("Found commit from low-priority stack");
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

    log_info("=== Work Summary ===");

    // Print PR summary
    for pr in prs_needing_testing {
        let repo_name = repo_map.get(&pr.repository_id)
            .map(|r| r.name.as_str())
            .unwrap_or("unknown");
        
        let (total_commits, approved_commits, untested_commits) = pr.get_commit_counts(tx).await
            .context("getting PR commit counts")?;
        
        let unapproved = total_commits - approved_commits;
        log_info(format_args!(
            "{} PR#{} {} commits left to test ({} unapproved)",
            repo_name, pr.pr_number, untested_commits, unapproved
        ));
    }

    // Print high-priority stack summary
    for (stack, _commits) in high_priority_stacks {
        let repo_name = repo_map.get(&stack.repository_id)
            .map(|r| r.name.as_str())
            .unwrap_or("unknown");
        
        let prs = stack.get_associated_prs(tx).await
            .context("getting associated PRs for stack")?;
        let pr_numbers: Vec<String> = prs.iter().map(|pr| format!("#{}", pr.pr_number)).collect();
        
        let (_total, signed, untested) = stack.get_commit_counts(tx).await
            .context("getting stack commit counts")?;
        
        log_info(format_args!(
            "{} {} PRs {} ({} signed, {} left to test)",
            repo_name, stack.target_branch, pr_numbers.join(", "), signed, untested
        ));
    }

    // Print low-priority stack summary if any
    if !low_priority_stacks.is_empty() {
        log_info("=== Low Priority Stacks ===");
        for (stack, _commits) in low_priority_stacks {
            let repo_name = repo_map.get(&stack.repository_id)
                .map(|r| r.name.as_str())
                .unwrap_or("unknown");
            
            let prs = stack.get_associated_prs(tx).await
                .context("getting associated PRs for low-priority stack")?;
            let pr_numbers: Vec<String> = prs.iter().map(|pr| format!("#{}", pr.pr_number)).collect();
            
            let (_total, signed, untested) = stack.get_commit_counts(tx).await
                .context("getting low-priority stack commit counts")?;
            
            log_info(&format_args!(
                "{} {} PRs {} ({} signed, {} left to test)",
                repo_name, stack.target_branch, pr_numbers.join(", "), signed, untested
            ));
        }
    }
    log_info("");

    Ok(())
}

/// Calculate the priority of a stack using the formula from the documentation
/// 
/// # Errors
/// 
/// Returns an error if database operations fail.
async fn calculate_stack_priority(
    commits: &[Commit],
    tx: &lcilib::Transaction<'_>,
    repo_map: &HashMap<i32, &Repository>,
) -> anyhow::Result<f64> {
    let mut total_priority = 0.0;
    
    for (position, commit) in commits.iter().enumerate() {
        let commit_priority = calculate_commit_priority(commit, tx, repo_map).await
            .context("calculating commit priority")?;
        
        // Apply position weighting: (1/2)^position
        let weight = 0.5_f64.powi(position as i32);
        total_priority += commit_priority * weight;
    }
    
    Ok(total_priority)
}

/// Calculate the priority of an individual commit
/// 
/// # Errors
/// 
/// Returns an error if database operations fail.
async fn calculate_commit_priority(
    commit: &Commit,
    tx: &lcilib::Transaction<'_>,
    repo_map: &HashMap<i32, &Repository>,
) -> anyhow::Result<f64> {
    // Find the PR(s) this commit belongs to
    let pr_commits = lcilib::db::models::PrCommit::find_by_commit(tx, commit.id).await
        .context("finding PR commits for commit")?;
    
    if pr_commits.is_empty() {
        // Commit not in any PR, use default priority
        return Ok(0.0);
    }
    
    // Get the oldest PR this commit belongs to
    let mut oldest_pr: Option<PullRequest> = None;
    let mut base_priority = 0;
    
    for pr_commit in &pr_commits {
        if let Some(pr) = PullRequest::find_by_id(tx, pr_commit.pull_request_id).await
            .context("finding pull request")? {
            
            if oldest_pr.is_none() || pr.created_at < oldest_pr.as_ref().unwrap().created_at {
                oldest_pr = Some(pr.clone());
                base_priority = pr.priority;
            }
        }
    }
    
    let Some(oldest_pr) = oldest_pr else {
        return Ok(0.0);
    };
    
    // Start with: 10 × PR priority
    let mut priority = 10.0 * base_priority as f64;
    
    // Add: +1 for every ACK
    let ack_count = oldest_pr.get_ack_count(tx).await
        .context("getting ACK count")?;
    priority += ack_count as f64;
    
    // Add: +0.5 if GPG-signed already
    if let Some(repo) = repo_map.get(&commit.repository_id) {
        match is_commit_gpg_signed(commit, &repo.path).await {
            Ok(true) => priority += 0.5,
            Ok(false) => {}, // No bonus
            Err(e) => {
                log_info(format_args!("Warning: Failed to check GPG signature for commit {}: {}", commit.jj_change_id, e));
                // Assume unsigned
            }
        }
    }
    
    // Add: +0.1 per day based on creation time of its PR
    let age_days = (Utc::now() - oldest_pr.created_at).num_days();
    priority += 0.1 * age_days as f64;
    
    Ok(priority)
}

/// Check if a commit is GPG signed using jj
/// 
/// # Errors
/// 
/// Returns an error if the jj command fails or if we can't determine the repository path.
async fn is_commit_gpg_signed(commit: &Commit, repo_path: &str) -> anyhow::Result<bool> {
    let change_id = commit.jj_change_id.clone();
    let repo_path = repo_path.to_string();
    
    let result = task::spawn_blocking(move || {
        use xshell::{Shell, cmd};
        
        let sh = Shell::new()?;
        sh.change_dir(&repo_path);
        
        let output = cmd!(sh, "jj log -r {change_id} -T if(signature, \"true\", \"false\")")
            .read()
            .context("running jj log command")?;
        
        Ok::<bool, anyhow::Error>(output.trim() == "true")
    }).await
    .context("spawning blocking task for jj command")??;
    
    Ok(result)
}

/// Log a message with timestamp prefix
fn log_info<D: fmt::Display>(message: D) {
    let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
    println!("[{}] {}", timestamp, message);
}
