// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Context as _;
use lcilib::db::models::{Commit, DbRepositoryId, Repository};
use std::collections::HashMap;
use tokio::task;
use xshell::Shell;

/// Check if a commit is GPG signed using jj
///
/// # Errors
///
/// Returns an error if the jj command fails or if we can't determine the repository path.
pub async fn is_commit_gpg_signed(commit: &Commit, repo_path: &str) -> anyhow::Result<bool> {
    let change_id = commit.jj_change_id.clone();
    let repo_path = repo_path.to_string();

    let result = task::spawn_blocking(move || {
        let sh = Shell::new()?;
        sh.change_dir(&repo_path);

        lcilib::jj::is_commit_gpg_signed(&sh, &change_id).map_err(anyhow::Error::from)
    })
    .await
    .context("spawning blocking task for jj command")??;

    Ok(result)
}

/// Calculate the priority of a stack using the formula from the documentation
///
/// # Errors
///
/// Returns an error if database operations fail.
pub async fn calculate_stack_priority(
    commits: &[Commit],
    tx: &lcilib::Transaction<'_>,
    repo_map: &HashMap<DbRepositoryId, &Repository>,
) -> anyhow::Result<f64> {
    let mut total_priority = 0.0;

    for (position, commit) in commits.iter().enumerate() {
        let commit_priority = calculate_commit_priority(commit, tx, repo_map)
            .await
            .context("calculating commit priority")?;

        // Apply position weighting: (1/2)^position
        let weight = 0.5_f64.powi(i32::try_from(position)?);
        total_priority += commit_priority * weight;
    }

    Ok(total_priority)
}

/// Calculate the priority of an individual commit
///
/// # Errors
///
/// Returns an error if database operations fail.
#[expect(clippy::cast_precision_loss)] // fine; we are computing priorities which don't need to be precise
async fn calculate_commit_priority(
    commit: &Commit,
    tx: &lcilib::Transaction<'_>,
    repo_map: &HashMap<DbRepositoryId, &Repository>,
) -> anyhow::Result<f64> {
    use chrono::Utc;
    use lcilib::db::models::{PrCommit, PullRequest};

    // Find the PR(s) this commit belongs to
    let pr_commits = PrCommit::find_by_commit(tx, commit.id)
        .await
        .context("finding PR commits for commit")?;

    if pr_commits.is_empty() {
        // Commit not in any PR, use default priority
        return Ok(0.0);
    }

    // Get the oldest PR this commit belongs to
    let mut oldest_pr: Option<PullRequest> = None;
    let mut base_priority = 0;

    for pr_commit in &pr_commits {
        if let Some(pr) = PullRequest::find_by_id(tx, pr_commit.pull_request_id)
            .await
            .context("finding pull request")?
            && (oldest_pr.is_none() || pr.created_at < oldest_pr.as_ref().unwrap().created_at)
        {
            oldest_pr = Some(pr.clone());
            base_priority = pr.priority;
        }
    }

    let Some(oldest_pr) = oldest_pr else {
        return Ok(0.0);
    };

    // Start with: 10 × PR priority
    let mut priority = 10.0 * f64::from(base_priority);

    // Add: +1 for every ACK
    let ack_count = oldest_pr
        .get_ack_count(tx)
        .await
        .context("getting ACK count")?;
    priority += ack_count as f64;

    // Add: +0.5 if GPG-signed already
    if let Some(repo) = repo_map.get(&commit.repository_id) {
        match is_commit_gpg_signed(commit, &repo.path).await {
            Ok(true) => priority += 0.5,
            Ok(false) => {} // No bonus
            Err(e) => {
                super::log::info(format_args!(
                    "Warning: Failed to check GPG signature for commit {}: {}",
                    commit.jj_change_id, e
                ));
                // Assume unsigned
            }
        }
    }

    // Add: +0.1 per day based on creation time of its PR
    let age_days = (Utc::now() - oldest_pr.created_at).num_days();
    priority += 0.1 * age_days as f64;

    Ok(priority)
}
