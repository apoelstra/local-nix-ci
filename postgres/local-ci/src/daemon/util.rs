// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Context as _;
use lcilib::db::models::{CommitToTest, Repository};
use lcilib::jj::is_commit_gpg_signed;

/// Calculate the priority of a stack using the formula from the documentation
///
/// # Errors
///
/// Returns an error if database operations fail.
pub async fn calculate_stack_priority(
    commits: &[CommitToTest],
    tx: &lcilib::Transaction<'_>,
) -> anyhow::Result<f64> {
    let mut total_priority = 0.0;

    for (position, commit) in commits.iter().enumerate() {
        let commit_priority = calculate_commit_priority(commit, tx)
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
    commit: &CommitToTest,
    tx: &lcilib::Transaction<'_>,
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
    let repo = Repository::get_by_id(tx, commit.repository_id).await?;
    match is_commit_gpg_signed(&repo.repo_shell, &commit.jj_change_id).await {
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

    // Add: +0.1 per day based on creation time of its PR
    let age_days = (Utc::now() - oldest_pr.created_at).num_days();
    priority += 0.1 * age_days as f64;

    Ok(priority)
}
