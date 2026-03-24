// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Context as _;
use chrono::Utc;
use lcilib::{
    Db,
    db::CiStatus,
    db::models::{
        CommitToTest, DbRepositoryId, PullRequest, Repository, ReviewStatus,
        Stack,
    },
    jj::is_commit_gpg_signed,
};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time;

use super::{get_repository_for_commit, mark_commit_status};
use super::{build_derivation, log, util};

/// Returns all the high-priority and low-priority stacks across all repos.
async fn find_stacks(
    tx: &lcilib::Transaction<'_>,
) -> anyhow::Result<(
    Vec<(Stack, Vec<CommitToTest>)>,
    Vec<(Stack, Vec<CommitToTest>)>,
)> {
    // Find all stacks grouped by DB and branch
    let mut branch_map = HashMap::<(DbRepositoryId, String), Vec<(f64, Stack, Vec<CommitToTest>)>>::new();
    for stack in Stack::get_all(tx).await? {
        let commits = stack.id.get_commits(tx).await?;
        let priority = util::calculate_stack_priority(&commits, tx)
            .await
            .context("calculating stack priority")?;

        branch_map
            .entry((stack.repository_id, stack.target_branch.clone()))
            .or_default()
            .push((priority, stack, commits));
    }

    let mut high_priority = vec![];
    let mut low_priority = vec![];

    for mut stacks in branch_map.into_values() {
        // priority order
        stacks.sort_by(|a, b| a.0.total_cmp(&b.0));
        // pop() takes the highest one
        if let Some(stack) = stacks.pop() {
            // We check the untested count right before pushing -- this means that
            // the high-priority stack might be empty, if we're done testing all the
            // high priority commits. Then we'll move on to PR commits.
            let (_, untested) = stack.1.get_commit_counts(tx).await?;
            if untested > 0 {
                high_priority.push((stack.1, stack.2));
            }
        }
        for stack in stacks {
            let (_, untested) = stack.1.get_commit_counts(tx).await?;
            if untested > 0 {
                low_priority.push((stack.1, stack.2));
            }
        }
    }

    Ok((high_priority, low_priority))
}

/// Find the next commit that needs testing, following the priority rules
///
/// # Errors
///
/// Returns an error if database operations fail.
async fn find_next_commit_to_test(db: &mut Db) -> anyhow::Result<Option<CommitToTest>> {
    let tx = db.transaction().await.context("starting transaction")?;

    // Compute lists of available work and print summary.
    let (high_priority_stacks, low_priority_stacks) =
        find_stacks(&tx).await.context("finding stacks")?;
    let prs_needing_testing = PullRequest::find_needing_testing_prioritized(&tx)
        .await
        .context("finding PRs needing testing")?;

    print_work_summary(
        &tx,
        &high_priority_stacks,
        &prs_needing_testing,
        &low_priority_stacks,
    )
    .await
    .context("printing work summary")?;

    // 1. Check high-priority stacks first (with positive priority)
    for (stack, commits) in &high_priority_stacks {
        for commit in commits {
            if commit.should_run_ci && commit.ci_status == CiStatus::Unstarted {
                log::info(format_args!("Found commit from high-priority stack {}", stack.id));
                tx.commit().await.context("committing transaction")?;
                return Ok(Some(commit.clone()));
            }
        }
    }

    // 2. Check PRs by priority (user priority, then all approved, then fewer untested, then age)
    let mut prioritized_prs = Vec::new();
    for pr in &prs_needing_testing {
        let counts = pr
            .id
            .get_commit_counts(&tx)
            .await
            .context("getting PR commit counts")?;

        let all_approved = counts.approved == counts.total;
        let age_days = (Utc::now() - pr.created_at).num_days();

        prioritized_prs.push((pr, all_approved, counts.untested, age_days));
    }

    // Sort by: priority DESC, all_approved DESC, untested ASC ("closer to done" first), age DESC (older first)
    prioritized_prs.sort_by(|a, b| {
        a.0.priority
            .cmp(&b.0.priority)
            .reverse()
            .then(a.1.cmp(&b.1).reverse())
            .then(a.2.cmp(&b.2))
            .then(a.3.cmp(&b.3).reverse())
    });

    for (pr, _all_approved, _neg_untested, _age) in &prioritized_prs {
        if let Some(commit) = pr
            .get_next_untested_commit(&tx)
            .await
            .context("getting next untested commit from PR")?
        {
            log::info("Found commit from PR");
            tx.commit().await.context("committing transaction")?;
            return Ok(Some(commit));
        }
    }

    // 3. Check low-priority stacks (negative priority or conflicting)
    for (_stack, commits) in &low_priority_stacks {
        for commit in commits {
            if commit.should_run_ci && commit.ci_status == CiStatus::Unstarted {
                log::info("Found commit from low-priority stack");
                tx.commit().await.context("committing transaction")?;
                return Ok(Some(commit.clone()));
            }
        }
    }

    tx.commit().await.context("committing transaction")?;
    Ok(None)
}

/// Print a summary of all remaining work
async fn print_work_summary(
    tx: &lcilib::Transaction<'_>,
    high_priority_stacks: &[(Stack, Vec<CommitToTest>)],
    prs_needing_testing: &[PullRequest],
    low_priority_stacks: &[(Stack, Vec<CommitToTest>)],
) -> anyhow::Result<()> {
    if prs_needing_testing.is_empty()
        && high_priority_stacks.is_empty()
        && low_priority_stacks.is_empty()
    {
        // If there is nothing to do, print no summary. We will use backoff logic
        // to print "nothing to do" messages without spamming the user at a higher
        // layer.
        return Ok(());
    }

    log::info("=== Available Work Summary ===");

    // Print PR summary with individual commits
    if !prs_needing_testing.is_empty() {
        log::info("\n");
        log::info("=== PRs needing testing ===");
    }
    for pr in prs_needing_testing {
        let repo = Repository::get_by_id(tx, pr.repository_id).await?;

        let counts = pr
            .id
            .get_commit_counts(tx)
            .await
            .context("getting PR commit counts")?;

        if counts.unapproved > 0 {
            log::info(format_args!(
                "{} PR#{} {} commits left to test ({} unapproved) (PR {})",
                repo.name, pr.pr_number, counts.untested, counts.unapproved, pr.review_status
            ));
        } else {
            log::info(format_args!(
                "{} PR#{} {} commits left to test (PR {})",
                repo.name, pr.pr_number, counts.untested, pr.review_status
            ));
        };

        // Get commits that need testing for this PR
        let commits_to_test = pr
            .id
            .get_current_non_merge_commits(tx)
            .await
            .context("getting commits needing testing for PR")?;

        for commit in commits_to_test {
            if commit.review_status == ReviewStatus::Approved
                && commit.ci_status == CiStatus::Unstarted
                && commit.should_run_ci
            {
                let prs: Vec<_> = commit.prs.iter().map(|(pr, commit_type)| format!("PR #{}, {}", pr.pr_number, commit_type)).collect();
                let prs_str = prs.join(", ");
                log::info(format_args!(
                    "  - {} ({}) ({})",
                    commit.git_commit_id,
                    commit.jj_change_id.prefix8(),
                    prs_str,
                ));
            }
        }
    }

    // Print high-priority stack summary
    if !high_priority_stacks.is_empty() {
        log::info("\n");
        log::info("=== High Priority Stacks ===");
    }
    for (stack, _commits) in high_priority_stacks {
        let repo = Repository::get_by_id(tx, stack.repository_id).await?;
        let prs = stack
            .get_associated_prs(tx)
            .await
            .context("getting associated PRs for stack")?;
        let pr_numbers: Vec<String> = prs.iter().map(|pr| format!("#{}", pr.pr_number)).collect();

        let (_total, untested) = stack
            .get_commit_counts(tx)
            .await
            .context("getting stack commit counts")?;

        // Count signed commits in the stack
        let signed = count_signed_commits_in_stack(tx, stack)
            .await
            .context("counting signed commits in stack")?;

        log::info(format_args!(
            "{} {} {} PRs {} ({} signed, {} left to test)",
            stack.id,
            repo.name,
            stack.target_branch,
            pr_numbers.join(", "),
            signed,
            untested
        ));
    }

    // Print low-priority stack summary if any
    if !low_priority_stacks.is_empty() {
        log::info("");
        log::info("=== Low Priority Stacks ===");
    }
    for (stack, _commits) in low_priority_stacks {
        let repo = Repository::get_by_id(tx, stack.repository_id).await?;

        let prs = stack
            .get_associated_prs(tx)
            .await
            .context("getting associated PRs for low-priority stack")?;
        let pr_numbers: Vec<String> =
            prs.iter().map(|pr| format!("#{}", pr.pr_number)).collect();

        let (_total, untested) = stack
            .get_commit_counts(tx)
            .await
            .context("getting low-priority stack commit counts")?;

        // Count signed commits in the stack
        let signed = count_signed_commits_in_stack(tx, stack)
            .await
            .context("counting signed commits in low-priority stack")?;

        log::info(format_args!(
            "{} {} {} PRs {} ({} signed, {} left to test)",
            stack.id,
            repo.name,
            stack.target_branch,
            pr_numbers.join(", "),
            signed,
            untested
        ));
    }

    Ok(())
}

/// Count the number of signed commits in a stack
async fn count_signed_commits_in_stack(
    tx: &lcilib::Transaction<'_>,
    stack: &Stack,
) -> anyhow::Result<usize> {
    let commits = stack
        .id
        .get_commits(tx)
        .await
        .context("getting stack commits")?;

    let repo = Repository::get_by_id(tx, stack.repository_id).await?;

    let mut signed_count = 0;
    for commit in &commits {
        if is_commit_gpg_signed(&repo.repo_shell, &commit.jj_change_id).await? {
            signed_count += 1;
        }
    }

    Ok(signed_count)
}

pub async fn run_ci_cycle_loop() -> anyhow::Result<()> {
    let mut db = Db::connect()
        .await
        .context("connecting to database for CI cycle")?;

    let mut no_work_count = 0u32;
    let mut last_no_work_log = std::time::Instant::now();
    let mut error_limit = log::BackoffSleepToken::new();

    loop {
        let commit = match find_next_commit_to_test(&mut db).await {
            Ok(Some(commit)) => commit,
            Ok(None) => {
                // Nothing to do -- just sleep for a second and maybe post a 'nothing to do' message so the
                // user knows we're still alive.
                let delay_secs = match no_work_count {
                    0 => 5,
                    1 => 60,
                    2 => 300,
                    n => 300 * (1u64 << (n - 2)).min(18000), // Cap at 5 hours
                };

                let now = std::time::Instant::now();
                if now.duration_since(last_no_work_log).as_secs() >= delay_secs {
                    log::info(format_args!("Nothing to do."));
                    no_work_count += 1;
                    last_no_work_log = now;
                }

                time::sleep(Duration::from_secs(1)).await;
                continue;
            },
            Err(e) => {
                log::warn_backoff(
                    &mut error_limit,
                    &*e.into_boxed_dyn_error(),
                    "Failed to get next commit to test.",
                ).await;
                continue;
            }
        };

        // Get repository information
        let repo = match get_repository_for_commit(&mut db, &commit).await {
            Ok(repo) => repo,
            Err(e) => {
                log::warn_backoff(
                    &mut error_limit,
                    &*e.into_boxed_dyn_error(),
                    format!(
                        "Failed to get repository for commit {}",
                        commit.git_commit_id
                    ),
                )
                .await;
                continue;
            }
        };

        // If we got a commit, we can reset the error backoff.
        error_limit.reset();

        log::info("");
        log::info(format_args!(
            "Starting CI for commit: {} ({})",
            commit.git_commit_id, commit.jj_change_id
        ));
        for (pr, commit_type) in &commit.prs {
            log::info(format_args!("    {} PR #{} ({}): {}", repo.name, pr.pr_number, commit_type, pr.title));
        }
        log::info("");

        // Process the commit
        match build_derivation::process_commit_ci(&mut db, &commit, &repo).await {
            Ok(success) => {
                if success {
                    log::info(format_args!(
                        "CI SUCCESS for commit: {}",
                        commit.git_commit_id
                    ));
                    if let Err(e) = mark_commit_status(&mut db, commit.id, CiStatus::Passed).await {
                        log::warn(&*e.into_boxed_dyn_error(), "Failed to mark commit passed.");
                    }
                    // After a commit succeeds, re-scan the database to see if we should make merge commits or something
                    // FIXME seems like even with the `RepoShell` abstraction we are still getting
                    //  concurrent describes with this.
                    /*
                    super::real_run_db_maintenance_cycle(
                        &mut db,
                        &mut log::RateLimitToken::ok_to_run(),
                    )
                    .await?;
                    */
                } else {
                    // FIXME shouldn't this be an error case?
                    log::info(format_args!(
                        "CI FAILED for commit: {}",
                        commit.git_commit_id
                    ));
                    // Error details already logged and commit marked as failed in process_commit_ci
                    if let Err(e) = mark_commit_status(&mut db, commit.id, CiStatus::Failed).await {
                        log::warn(&*e.into_boxed_dyn_error(), "Failed to mark commit failed.");
                    }
                }
            }
            Err(e) => {
                log::warn(
                    &*e.into_boxed_dyn_error(),
                    format!("Error processing commit {}", commit.git_commit_id),
                );
                if let Err(e) = mark_commit_status(&mut db, commit.id, CiStatus::Failed).await {
                    log::warn(&*e.into_boxed_dyn_error(), "Failed to mark commit failed.");
                }
            }
        }
    }
}
