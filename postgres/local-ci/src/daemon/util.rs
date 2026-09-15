// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Context as _;
use lcilib::db::models::{CiStatus, CommitToTest, Repository};
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

    let mut position = 0;
    for commit in commits {
        let commit_priority = calculate_commit_priority(commit, tx)
            .await
            .context("calculating commit priority")?;

        // Apply position weighting: (1/2)^position
        let weight = 0.5_f64.powi(position);
        total_priority += commit_priority * weight;

        // Only increment "position" for untested commits. Otherwise we get perverse stuff
        // where a stack with 5 finished merges and a high-priority PR on top is ignored
        // in favor of a brand-new stack with the high-priority PR on the bottom.
        if commit.ci_status != CiStatus::Passed {
            position += 1;
        }
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
    use lcilib::db::models::{PrCommit, PullRequest, UserPriorityOffset};

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

    // Add user priority offset: a fixed offset keyed on the PR author,
    // distinct from (and independent of) the ACK-counting logic below.
    let user_offset = UserPriorityOffset::get_offset_by_username(tx, &oldest_pr.author_login)
        .await
        .context("getting user priority offset")?;
    priority += f64::from(user_offset);

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

    // Add weighted ACK contribution. `get_ack_weight` sums maintainer review
    // scores across pending/posted/external ACKs on the tip commit, and
    // internally scales by 0.25 if the PR author is the configured user.
    // Multiply by 3 so a strong reviewer weighting typically dominates
    // everything except the operator-set user priority offset.
    let ack_weight = oldest_pr
        .get_ack_weight(tx, repo.repo_shell.upstream())
        .await
        .context("getting ACK weight")?;
    priority += 3.0 * ack_weight;

    // Add: +0.1 per day based on creation time of its PR
    let age_days = (Utc::now() - oldest_pr.created_at).num_days();
    priority += 0.1 * age_days as f64;

    Ok(priority)
}
