// SPDX-License-Identifier: GPL-3.0-or-later

mod ci_cycle;
mod log;
mod util;

use anyhow::Context as _;
use lcilib::Db;
use lcilib::db::models::{
    CiStatus, Commit, CommitToTest, DbAckId, DbCommitId, DbPullRequestId, DbRepositoryId, DbStackId, PullRequest,
    NewCommit, NewStack, Repository, ReviewStatus, Stack, UpdateCommit,
};
use lcilib::{gh, git, jj};
use lcilib::jj::is_commit_gpg_signed;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time;

pub async fn run(_db: &mut Db) -> anyhow::Result<()> {
    log::info(format_args!("Starting local-ci daemon..."));

    // Start all cycles concurrently, each with its own database connection
    let tasks = vec![
        tokio::spawn(run_db_maintenance_cycle()),
        tokio::spawn(ci_cycle::run_ci_cycle_loop()),
        tokio::spawn(run_pr_sync_cycle()),
    ];

    // Wait for all tasks to complete (which should never happen)
    for task in tasks {
        task.await.context("daemon task failed")??;
    }

    Ok(())
}

async fn run_db_maintenance_cycle() -> anyhow::Result<()> {
    let mut db = Db::connect()
        .await
        .context("connecting to database for maintenance cycle")?;

    let mut error_limit = log::BackoffSleepToken::new();
    let mut info_limit = log::RateLimiter::new(Duration::from_mins(5));

    let idle_time = Duration::from_secs(30);
    loop {
        match real_run_db_maintenance_cycle(&mut db, &mut info_limit.token()).await {
            Ok(true) => {
                error_limit.reset();
            }
            Ok(false) => {
                time::sleep(idle_time).await;
                error_limit.reset();
            }
            Err(e) => {
                log::warn_backoff(
                    &mut error_limit,
                    &*e.into_boxed_dyn_error(),
                    "Failed to run DB maintenance cycle.",
                )
                .await;
            }
        };
    }
}

async fn real_run_db_maintenance_cycle(
    db: &mut Db,
    log_limit: &mut log::RateLimitToken,
) -> anyhow::Result<bool> {
    let mut had_work = false;

    // Check for pending ACKs that need to be posted
    if check_pending_acks(db, log_limit).await
        .context("checking pending ACKs")?
    {
        had_work = true;
    }

    // Check for approved PRs that need merge commits created
    if check_approved_prs(db).await
        .context("checking approved PRs")?
    {
        had_work = true;
    }

    // Check for signed merge commits that need to be pushed
    if check_signed_merges(db, log_limit).await
        .context("checking signed merges")?
    {
        had_work = true;
    }

    Ok(had_work)
}

async fn run_pr_sync_cycle() -> anyhow::Result<()> {
    let mut db = Db::connect()
        .await
        .context("connecting to database for PR sync cycle")?;

    // Run every 15 minutes
    let mut interval = time::interval(Duration::from_secs(15 * 60));

    loop {
        interval.tick().await;

        match sync_all_repositories(&mut db).await {
            Ok(()) => {
                log::info(format_args!("PR sync cycle completed successfully"));
            }
            Err(e) => {
                log::warn(
                    &*e.into_boxed_dyn_error(),
                    "Failed PR sync cycle.",
                );
            }
        }
    }
}

async fn check_pending_acks(
    db: &mut Db,
    log_limit: &mut log::RateLimitToken,
) -> anyhow::Result<bool> {
    let tx = db.transaction().await.context("starting transaction")?;

    // Query for ACKs that are pending or failed and might be ready to post
    let rows = tx
        .query(
            r#"
            SELECT
                a.id as ack_id,
                a.pull_request_id,
                a.commit_id,
                a.reviewer_name,
                a.message,
                a.status as ack_status,
                pr.repository_id
                pr.pr_number,
                pr.review_status as pr_review_status,
                pr.author_login,
            FROM acks a
            JOIN pull_requests pr ON a.pull_request_id = pr.id
            WHERE a.status IN ('pending', 'failed')
            AND pr.review_status = 'approved'
            ORDER BY a.created_at ASC
            "#,
            &[],
        )
        .await
        .context("querying for pending ACKs")?;

    let mut work_done = false;

    for row in rows {
        let ack_id: DbAckId = row.get("ack_id");
        let pull_request_id: DbPullRequestId = row.get("pull_request_id");
        let pr_number: i32 = row.get("pr_number");
        let reviewer_name: String = row.get("reviewer_name");
        let message: String = row.get("message");
        let repo_id: DbRepositoryId = row.get("repository_id");
        let author_login: String = row.get("author_login");

        let repo = Repository::get_by_id(&tx, repo_id)
            .await
            .context("looking up repository for ACK")?;

        // Check if all non-merge commits in this PR are approved and passed CI
        let commit_check_rows = tx
            .query(
                r#"
                SELECT COUNT(*) as total_commits,
                       COUNT(CASE WHEN c.review_status = 'approved' AND c.ci_status = 'passed' THEN 1 END) as ready_commits
                FROM commits c
                JOIN pr_commits pc ON c.id = pc.commit_id
                WHERE pc.pull_request_id = $1
                AND pc.is_current = true
                AND pc.commit_type != 'merge'
                "#,
                &[&pull_request_id],
            )
            .await
            .context("checking commit status for PR")?;

        if let Some(commit_row) = commit_check_rows.first() {
            let total_commits: i64 = commit_row.get("total_commits");
            let ready_commits: i64 = commit_row.get("ready_commits");

            if total_commits == 0 {
                log_limit.run(|| {
                    log::info(format_args!(
                        "ACK for PR #{} not posted: no non-merge commits found",
                        pr_number
                    ));
                });
                continue;
            }

            if ready_commits != total_commits {
                log_limit.run(|| log::info(format_args!(
                    "ACK for PR #{} not posted: {}/{} non-merge commits are approved and passed CI",
                    pr_number, ready_commits, total_commits
                )));
                continue;
            }

            // All conditions met, post the ACK
            let post_result = if author_login == "apoelstra" {
                // Post comment instead of approval for own PRs
                gh::post_pr_comment(&repo.repo_shell, pr_number, &message).await
            } else {
                // Post approval review
                gh::post_pr_approval(&repo.repo_shell, pr_number, &message).await
            };

            match post_result {
                Ok(()) => {
                    // Update ACK status to 'posted'
                    tx.execute(
                        "UPDATE acks SET status = 'posted' WHERE id = $1",
                        &[&ack_id],
                    )
                    .await
                    .context("updating ACK status to posted")?;

                    log::info(format_args!(
                        "Posted ACK for PR #{} from reviewer {}",
                        pr_number, reviewer_name
                    ));
                    work_done = true;
                }
                Err(e) => {
                    log::warn(&e,
                        format_args!(
                        "Failed to post ACK for PR #{} from reviewer {}: {}",
                        pr_number, reviewer_name, e
                    ));

                    // Update ACK status to 'failed'
                    tx.execute(
                        "UPDATE acks SET status = 'failed' WHERE id = $1",
                        &[&ack_id],
                    )
                    .await
                    .context("updating ACK status to failed")?;
                }
            }
        }
    }

    tx.commit().await.context("committing transaction")?;
    Ok(work_done)
}

async fn check_approved_prs(db: &mut Db) -> anyhow::Result<bool> {
    let mut work_done = false;

    // Step 1 & 2: For each approved PR, create merge commits and try to extend stacks
    let tx = db
        .transaction()
        .await
        .context("starting transaction for approved PRs")?;
    let approved_prs = tx.query(
        r#"
        SELECT pr.id, pr.repository_id, pr.pr_number, pr.title, pr.body, pr.author_login, pr.target_branch,
               pr.tip_commit_id, pr.merge_status, pr.review_status, pr.priority, pr.ok_to_merge,
               pr.required_reviewers, pr.created_at, pr.updated_at, pr.synced_at
        FROM pull_requests pr
        WHERE pr.review_status = 'approved'
        AND pr.merge_status = 'pending'
        AND NOT EXISTS (
            SELECT 1 FROM pr_commits pc
            JOIN commits c ON pc.commit_id = c.id
            WHERE pc.pull_request_id = pr.id
            AND pc.is_current = true
            AND pc.commit_type != 'merge'
            AND (c.review_status != 'approved' OR c.ci_status != 'passed')
        )
        ORDER BY pr.priority DESC, pr.created_at ASC
        "#,
        &[],
    ).await.context("querying approved PRs")?;
    tx.commit().await.context("committing approved PRs query")?;

    for pr_row in approved_prs {
        let pr = PullRequest {
            id: pr_row.get("id"),
            repository_id: pr_row.get("repository_id"),
            pr_number: pr_row.get("pr_number"),
            title: pr_row.get("title"),
            body: pr_row.get("body"),
            author_login: pr_row.get("author_login"),
            target_branch: pr_row.get("target_branch"),
            tip_commit_id: pr_row.get("tip_commit_id"),
            merge_status: pr_row.get("merge_status"),
            review_status: pr_row.get("review_status"),
            priority: pr_row.get("priority"),
            ok_to_merge: pr_row.get("ok_to_merge"),
            required_reviewers: pr_row.get("required_reviewers"),
            created_at: pr_row.get("created_at"),
            updated_at: pr_row.get("updated_at"),
            synced_at: pr_row.get("synced_at"),
        };

        if process_approved_pr(db, &pr).await
            .with_context(|| format!("processing approved PR {}", pr.pr_number))?
        {
            work_done = true;
        }
    }

    // Step 3: Process existing stacks for rebasing and updates
    if process_existing_stacks(db).await
        .context("processing existing stacks")?
    {
        work_done = true;
    }

    Ok(work_done)
}

async fn process_approved_pr(db: &mut Db, pr: &PullRequest) -> anyhow::Result<bool> {
    let tx = db.transaction().await.context("starting PR transaction")?;
    let repo = Repository::get_by_id(&tx, pr.repository_id).await?;

    // Step 1: Get all stacks for this repo/target, sorted by priority
    let all_stacks = Stack::get_all_for_target_branch(&tx, pr.repository_id, &pr.target_branch)
        .await
        .context("failed to find stacks")?;
    let mut annotated_stacks = Vec::with_capacity(all_stacks.len());
    for stack in all_stacks {
        let commits = stack
            .id
            .get_commits(&tx)
            .await
            .with_context(|| format!("failed to get commits for stack {}", stack.id))?;
        let priority = util::calculate_stack_priority(&commits, &tx)
            .await
            .context("calculating stack priority")?;
        annotated_stacks.push((
            priority,
            stack,
            commits,
        ));
    }
    // Sort by reverse priority.
    annotated_stacks.sort_by(|(a, _, _), (b, _, _)| b.total_cmp(a));
    let annotated_stacks = annotated_stacks; // remove mut

    // Step 2: Go through all stacks, checking whether this PR is present and/or first.
    let mut added_to_stack = false;
    let mut pr_is_first_in_some_stack = false;
    for (_, stack, commits) in &annotated_stacks {
        let index = commits
            .iter()
            .position(|commit| commit.prs.iter().any(|(pr1, _)| pr1.id == pr.id));

        // If the PR is already in a stack, great.
        if let Some(index) = index {
            pr_is_first_in_some_stack |= index == 0;
            added_to_stack = true;
        } else if !added_to_stack {
            // Otherwise, try to add it.
            if try_extend_stack(&tx, Some(stack.id), &pr.target_branch, commits, pr, &repo).await? {
                added_to_stack = true;
            }
        }
    }

    // Step 4: If not first in any stack, create direct merge and new stack
    if !pr_is_first_in_some_stack {
        try_extend_stack(&tx, None, &pr.target_branch, &[], pr, &repo).await?;
    }

    tx.commit().await.context("committing PR transaction")?;
    Ok(true)
}

async fn try_extend_stack(
    tx: &lcilib::Transaction<'_>,
    stack_id: Option<DbStackId>,
    target_branch: &str,
    commits: &[CommitToTest],
    pr: &PullRequest,
    repo: &Repository,
) -> anyhow::Result<bool> {
    // Get the tip commit of the PR
    let tip_commit = Commit::find_by_id(tx, pr.tip_commit_id)
        .await
        .with_context(|| format!("failed to find tip commit for PR {}", pr.pr_number))?
        .expect("commit is in database");

    // The parent for the merge is either the last commit in the stack or the target branch
    let stack_tip = if let Some(last_commit) = commits.last() {
        last_commit.jj_change_id.as_str()
    } else {
        target_branch
    };

    // Create merge commit: merge PR tip into stack tip.
    // TODO should compute the real merge description rather than using "<placeholder>". This
    //  is okay for now since the db maintenance loop will get it..
    match jj::create_merge_commit(
        &repo.repo_shell,
        tip_commit.git_commit_id.as_str(),
        stack_tip,
        None, // description
    ).await {
        Ok(jj_change_id) => {
            // Get the git commit ID for the new merge
            let git_commit_id =
                jj::get_current_git_commit_for_change_id(&repo.repo_shell, &jj_change_id)
                    .await
                    .context("getting git commit ID for stack merge")?;

            // Create commit record for the new merge
            let new_commit = NewCommit {
                repository_id: pr.repository_id,
                git_commit_id,
                jj_change_id,
                review_status: ReviewStatus::Approved,
                should_run_ci: true,
                ci_status: CiStatus::Unstarted,
                nix_derivation: None,
                review_text: Some(format!("Stack merge commit for PR #{}", pr.pr_number)),
            };

            let stack_merge_commit = Commit::create(tx, new_commit).await.map_err(|e| {
                anyhow::anyhow!("Failed to create stack merge commit record: {}", e)
            })?;

            // Add to PR as a merge commit
            lcilib::db::models::PrCommit::create(
                tx,
                pr.id,
                stack_merge_commit.id,
                0, // sequence order
                lcilib::db::models::CommitType::Merge,
            )
            .await
            .context("failed to create pr_commit record for stack merge")?;

            // Add to stack, or create new one
            if let Some(stack_id) = stack_id {
                let next_order = i32::try_from(commits.len())? + 1;
                stack_id
                    .add_commit(tx, stack_merge_commit.id, next_order)
                    .await
                    .context("failed to add commit to stack")?;

                log::info(format_args!(
                    "Extended stack {} with PR #{}",
                    stack_id, pr.pr_number
                ));
            } else {
                // Create new stack with direct merge as first commit
                let new_stack = NewStack {
                    repository_id: pr.repository_id,
                    target_branch: pr.target_branch.clone(),
                };

                let stack = Stack::create(tx, new_stack)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to create stack: {}", e))?;

                stack
                    .id
                    .add_commit(tx, stack_merge_commit.id, 1)
                    .await
                    .context("failed to add commit to new stack")?;

                log::info(format_args!(
                    "Created direct merge and new stack for PR #{}",
                    pr.pr_number
                ));
            }
            Ok(true)
        }
        Err(e) => {
            if let Some(id) = stack_id {
                log::info(format_args!(
                    "Failed to extend stack {} with PR #{}: {:?}",
                    id, pr.pr_number, anyhow::Error::from(e)
                ));
            } else {
                log::info(format_args!(
                    "Failed to create new stack with PR #{}: {:?}",
                    pr.pr_number, anyhow::Error::from(e)
                ));
            }
            Ok(false)
        }
    }
}

async fn process_existing_stacks(db: &mut Db) -> anyhow::Result<bool> {
    let mut work_done = false;

    // Get all stacks grouped by repo/target
    let tx = db
        .transaction()
        .await
        .context("starting stacks transaction")?;
    let all_stacks = Stack::get_all(&tx)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to find stacks: {}", e))?;
    tx.commit().await.context("committing stacks query")?;

    let mut stacks_by_repo_target: HashMap<(DbRepositoryId, String), Vec<Stack>> = HashMap::new();
    for stack in all_stacks {
        let key = (stack.repository_id, stack.target_branch.clone());
        stacks_by_repo_target.entry(key).or_default().push(stack);
    }

    // Process each group
    for ((repo_id, _target_branch), mut stacks) in stacks_by_repo_target {
        // Sort by priority (highest first)
        let tx = db
            .transaction()
            .await
            .context("starting stack priority transaction")?;
        let repo = Repository::get_by_id(&tx, repo_id).await?;

        let mut stack_priorities = Vec::new();
        for stack in &stacks {
            let commits = stack
                .id
                .get_commits(&tx)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to get stack commits: {}", e))?;
            let priority = util::calculate_stack_priority(&commits, &tx)
                .await
                .context("calculating stack priority")?;
            stack_priorities.push((stack, priority));
        }
        tx.commit()
            .await
            .context("committing stack priority transaction")?;

        stack_priorities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        stacks = stack_priorities
            .into_iter()
            .map(|(s, _)| s.clone())
            .collect();

        // Process each stack
        for stack in stacks {
            if process_stack_updates(db, &stack, &repo).await
                .with_context(|| format!("processing updates for stack {}", stack.id))?
            {
                work_done = true;
            }
        }
    }

    Ok(work_done)
}

async fn process_stack_updates(
    db: &mut Db,
    stack: &Stack,
    repo: &Repository,
) -> anyhow::Result<bool> {
    let tx = db
        .transaction()
        .await
        .context("starting stack update transaction")?;

    let stack_commits = stack
        .id
        .get_commits(&tx)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get stack commits: {}", e))?;

    if stack_commits.is_empty() {
        tx.commit()
            .await
            .context("committing empty stack transaction")?;
        return Ok(false);
    }

    // Check if first commit's first parent matches target
    let first_commit = &stack_commits[0];
    let parents = git::list_parents(&repo.repo_shell, &first_commit.git_commit_id)
        .await
        .context("getting commit parents")?;

    let target_commit = git::resolve_ref(&repo.repo_shell, &stack.target_branch)
        .await
        .context("resolving target branch")?;

    let needs_rebase = parents.is_empty() || parents[0].to_string() != target_commit.to_string();

    if needs_rebase {
        log::info(format_args!(
            "Stack {} needs rebase due to target branch change",
            stack.id
        ));

        // Check for GPG signatures before rebasing
        for commit in &stack_commits {
            if is_commit_gpg_signed(&repo.repo_shell, &commit.jj_change_id).await? {
                log::info(format_args!(
                    "Throwing away GPG signature for commit {} due to rebase",
                    commit.jj_change_id
                ));
            }
        }

        // Mark all commits as not current and delete stack
        for commit in &stack_commits {
            tx.execute(
                "UPDATE pr_commits SET is_current = false WHERE commit_id = $1",
                &[&commit.id],
            )
            .await
            .context("marking commits as not current")?;
        }

        tx.execute(
            "DELETE FROM stack_commits WHERE stack_id = $1",
            &[&stack.id],
        )
        .await
        .context("deleting stack commits")?;
        tx.execute("DELETE FROM stacks WHERE id = $1", &[&stack.id])
            .await
            .context("deleting stack")?;

        tx.commit().await.context("committing rebase transaction")?;

        // TODO: Recreate stack in correct order - this is complex and would need
        // to re-run the extend_stack logic for each PR in the original order
        return Ok(true);
    }

    // Update descriptions and check for commit changes
    let mut work_done = false;
    for commit in &stack_commits {
        let pr = commit.id.get_pull_request(&tx).await?;
        // Update description (dummy implementation for now)
        let description = git::compute_merge_description(&tx, &pr, commit).await
            .with_context(|| format!("computing merge description for merge of PR {} (commit {})", pr.pr_number, commit.git_commit_id))?;
        if let Err(e) =
            jj::update_commit_description(&repo.repo_shell, &commit.jj_change_id, &description)
            .await
        {
            log::warn(&e, format_args!(
                "Failed to update description for commit {}",
                commit.jj_change_id
            ));
        }

        // Check if git commit ID has changed, and if so, if this was a "real" change (tree changed,
        // which should never happen, but okay, let's check) or just an accounting change (update
        // description or sign, which may happen externally).
        let mut stack_poisoned = false;
        match jj::get_current_git_commit_for_change_id(&repo.repo_shell, &commit.jj_change_id).await {
            Ok(current_git_id) => {
                if current_git_id != commit.git_commit_id {
                    // Get tree hash and parents to check what changed
                    let current_tree =
                        git::resolve_ref(&repo.repo_shell, format!("{}^{{tree}}", current_git_id))
                            .await
                            .context("getting current tree hash")?;
                    let original_tree = git::resolve_ref(
                        &repo.repo_shell,
                        format!("{}^{{tree}}", commit.git_commit_id),
                    )
                    .await
                    .context("getting original tree hash")?;

                    if current_tree == original_tree {
                        // Tree unchanged -- just update the commit ID in place (lol)
                        tx.execute(
                            "UPDATE commits SET git_commit_id = $1 WHERE id = $2",
                            &[&current_git_id, &commit.id],
                        )
                        .await
                        .context("updating git commit ID")?;

                        // Check for lost GPG signature
                        let was_signed = is_commit_gpg_signed(&repo.repo_shell, &commit.jj_change_id).await?;
                        if was_signed {
                            let is_signed = jj::is_commit_gpg_signed(
                                &repo.repo_shell,
                                &commit.jj_change_id,
                            ).await?;
                            if !is_signed {
                                log::info(format_args!(
                                    "Throwing away GPG signature on change {} due to commit ID change",
                                    commit.jj_change_id
                                ));
                            }
                        }
                    } else {
                        // Tree changed - kill the rest of the stack
                        // FIXME this should be a warning and somehow have a std::Error associated to it (by refactoring functions probably)
                        log::info(format_args!(
                            "Change {} tree changed (commit {} to {}), marking as not current removing remainder of stack.",
                            commit.jj_change_id, commit.git_commit_id, current_git_id,
                        ));
                        stack_poisoned = true;
                    }
                    work_done = true;
                }

                if stack_poisoned {
                    tx.execute(
                        "UPDATE pr_commits SET is_current = false WHERE commit_id = $1 AND pull_request_id = $2",
                        &[&commit.id, &pr.id],
                    )
                    .await
                    .context("marking commit as not current")?;
                    tx.execute(
                        "DELETE FROM stack_commits WHERE stack_id = $1 AND commit_id = $2",
                        &[&stack.id, &commit.id],
                    )
                    .await
                    .context("deleting commit from stack")?;
                }
            }
            Err(e) => {
                log::warn(&e, format_args!(
                    "Failed to get current git commit for {}",
                    commit.jj_change_id
                ));
            }
        }
    }

    tx.commit()
        .await
        .context("committing stack update transaction")?;
    Ok(work_done)
}

async fn check_signed_merges(
    _db: &mut Db,
    _log_limit: &mut log::RateLimitToken,
) -> anyhow::Result<bool> {
    // TODO: Query for signed merge commits that passed CI
    // TODO: Push them
    // Return true if work was done, false if nothing to do
    time::sleep(Duration::from_secs(15)).await;
    Ok(false)
}

async fn get_repository_for_commit(
    db: &mut Db,
    commit: &CommitToTest,
) -> anyhow::Result<Repository> {
    let tx = db.transaction().await.context("starting transaction")?;
    let repo = Repository::find_by_id(&tx, commit.repository_id)
        .await
        .map_err(|e| anyhow::anyhow!("Database error: {}", e))?
        .ok_or_else(|| anyhow::anyhow!("Repository not found for ID: {}", commit.repository_id))?;
    tx.commit().await.context("committing transaction")?;
    Ok(repo)
}

async fn mark_commit_status(
    db: &mut Db,
    commit: DbCommitId,
    new_status: CiStatus,
) -> anyhow::Result<()> {
    let tx = db.transaction().await.context("starting transaction")?;
    let updates = UpdateCommit {
        ci_status: Some(new_status),
        ..Default::default()
    };
    commit
        .apply_update(&tx, &updates)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to update commit: {}", e))?;
    tx.commit().await.context("committing transaction")?;
    Ok(())
}

async fn sync_all_repositories(db: &mut Db) -> anyhow::Result<()> {
    let tx = db
        .transaction()
        .await
        .context("starting transaction to get repositories")?;

    let (repositories, errors) = Repository::list_all(&tx).await;

    for e in errors {
        log::warn(&e, "failed to load repository (not syncing this repo)");
    }

    tx.commit().await.context("committing transaction")?;

    for repo in repositories {
        if let Err(e) = sync_repository_prs(db, &repo).await {
            log::warn(&*e.into_boxed_dyn_error(), format_args!(
                "Failed to sync PRs for repository {}",
                repo.name
            ));
        }
    }

    Ok(())
}

async fn sync_repository_prs(db: &mut Db, repo: &Repository) -> anyhow::Result<()> {
    // Use spawn_blocking for the GitHub API calls and shell operations
    let last_synced = repo.last_synced_at;

    // Do a 'git fetch'
    git::fetch(&repo.repo_shell)
        .await
        .context("failed git or jj fetch")?;

    // Get updated PRs from GitHub
    let pr_infos = gh::list_updated_prs(&repo.repo_shell, last_synced)
        .await
        .context("failed sync of recent activity via 'gh' utility")?;

    if !pr_infos.is_empty() {
        log::info(format_args!(
            "Found {} updated PRs for repository {}",
            pr_infos.len(),
            repo.name
        ));
    }
    for pr_info in &pr_infos {
        if let Err(e) = crate::pr::refresh(repo, pr_info, db)
            .await
            .with_context(|| format!("failed to refresh PR #{}", pr_info.number))
        {
            log::warn(&*e.into_boxed_dyn_error(), format_args!(
                "Warning: Failed to process PR #{} in repository {}",
                pr_info.number, repo.name,
            ));
        } else {
            log::info(format_args!(
                "Successfully synced PR #{} in repository {}",
                pr_info.number, repo.name
            ));
        }
    }

    // Update last synced time -- FIXME we should compute the time before invoking gh.
    let tx = db
        .transaction()
        .await
        .context("starting transaction to update last synced time")?;
    repo.update_last_synced(&tx)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to update last synced time: {}", e))?;
    tx.commit().await.context("committing transaction")?;

    Ok(())
}
