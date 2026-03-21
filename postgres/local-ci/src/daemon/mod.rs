// SPDX-License-Identifier: GPL-3.0-or-later

mod ci_cycle;
mod log;
mod util;

use anyhow::Context as _;
use lcilib::Db;
use lcilib::db::models::{Commit, PullRequest, Repository, UpdateCommit, CiStatus, Stack};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use tokio::{time, task};
use xshell::{cmd, Shell};

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
        task.await
            .context("daemon task failed")??;
    }
    
    Ok(())
}

async fn run_db_maintenance_cycle() -> anyhow::Result<()> {
    let mut db = Db::connect().await
        .context("connecting to database for maintenance cycle")?;
    
    loop {
        let mut had_work = false;

        // Check for pending ACKs that need to be posted
        if check_pending_acks(&mut db).await? {
            had_work = true;
        }

        // Check for approved PRs that need merge commits created
        if check_approved_prs(&mut db).await? {
            had_work = true;
        }

        // Check for signed merge commits that need to be pushed
        if check_signed_merges(&mut db).await? {
            had_work = true;
        }

        if !had_work {
            time::sleep(Duration::from_secs(5)).await;
        }
    }
}

async fn run_pr_sync_cycle() -> anyhow::Result<()> {
    let mut db = Db::connect().await
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
                log::warn(format_args!("Error in PR sync cycle: {}", e));
            }
        }
    }
}

async fn check_pending_acks(db: &mut Db) -> anyhow::Result<bool> {
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
                pr.pr_number,
                pr.review_status as pr_review_status,
                pr.author_login,
                r.path as repo_path
            FROM acks a
            JOIN pull_requests pr ON a.pull_request_id = pr.id
            JOIN repositories r ON pr.repository_id = r.id
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
        let ack_id: i32 = row.get("ack_id");
        let pull_request_id: i32 = row.get("pull_request_id");
        let pr_number: i32 = row.get("pr_number");
        let reviewer_name: String = row.get("reviewer_name");
        let message: String = row.get("message");
        let repo_path: String = row.get("repo_path");
        let author_login: String = row.get("author_login");

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
                log::info(format_args!(
                    "ACK for PR #{} not posted: no non-merge commits found",
                    pr_number
                ));
                continue;
            }

            if ready_commits != total_commits {
                log::info(format_args!(
                    "ACK for PR #{} not posted: {}/{} non-merge commits are approved and passed CI",
                    pr_number, ready_commits, total_commits
                ));
                continue;
            }

            // All conditions met, post the ACK
            let shell = Shell::new().context("creating shell")?;
            shell.change_dir(&repo_path);

            let post_result = if author_login == "apoelstra" {
                // Post comment instead of approval for own PRs
                lcilib::gh::post_pr_comment(&shell, pr_number, &message)
            } else {
                // Post approval review
                lcilib::gh::post_pr_approval(&shell, pr_number, &message)
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
                    log::warn_backoff(format!(
                        "Failed to post ACK for PR #{} from reviewer {}: {}",
                        pr_number, reviewer_name, e
                    )).await;

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
    
    // Get all repositories for path lookup
    let tx = db.transaction().await.context("starting transaction for repositories")?;
    let repositories = Repository::list_all(&tx).await
        .map_err(|e| anyhow::anyhow!("Failed to get repositories: {}", e))?;
    let repo_map: HashMap<i32, Repository> = repositories.into_iter().map(|r| (r.id, r)).collect();
    tx.commit().await.context("committing repository transaction")?;
    
    // Step 1 & 2: For each approved PR, create merge commits and try to extend stacks
    let tx = db.transaction().await.context("starting transaction for approved PRs")?;
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
        
        if process_approved_pr(db, &pr, &repo_map).await? {
            work_done = true;
        }
    }
    
    // Step 3: Process existing stacks for rebasing and updates
    if process_existing_stacks(db, &repo_map).await? {
        work_done = true;
    }
    
    Ok(work_done)
}

async fn process_approved_pr(
    db: &mut Db,
    pr: &PullRequest,
    repo_map: &HashMap<i32, Repository>,
) -> anyhow::Result<bool> {
    use lcilib::db::models::{Stack, NewStack, Commit, NewCommit, Ack, CommitType, ReviewStatus, CiStatus};
    
    let Some(repo) = repo_map.get(&pr.repository_id) else {
        log::warn(format_args!("Repository not found for PR #{}", pr.pr_number));
        return Ok(false);
    };
    
    let tx = db.transaction().await.context("starting PR transaction")?;
    
    // Get tip commit
    let Some(tip_commit) = Commit::find_by_id(&tx, pr.tip_commit_id).await
        .map_err(|e| anyhow::anyhow!("Failed to find tip commit: {}", e))?
    else {
        log::warn(format_args!("Tip commit not found for PR #{}", pr.pr_number));
        tx.commit().await.context("committing transaction")?;
        return Ok(false);
    };
    
    // Check if direct merge commit already exists
    let existing_merge = tx.query_opt(
        r#"
        SELECT c.id, c.jj_change_id 
        FROM commits c
        JOIN pr_commits pc ON c.id = pc.commit_id
        WHERE pc.pull_request_id = $1 
        AND pc.commit_type = 'merge'
        AND pc.is_current = true
        "#,
        &[&pr.id],
    ).await.context("checking for existing merge commit")?;
    
    let merge_commit = if let Some(row) = existing_merge {
        // Use existing merge commit
        let commit_id: i32 = row.get("id");
        Commit::find_by_id(&tx, commit_id).await
            .map_err(|e| anyhow::anyhow!("Failed to find existing merge commit: {}", e))?
            .unwrap()
    } else {
        // Create new merge commit
        let acks = Ack::find_by_pull_request(&tx, pr.id).await
            .map_err(|e| anyhow::anyhow!("Failed to get ACKs: {}", e))?;
        
        let ack_lines: Vec<String> = acks.iter()
            .filter(|ack| matches!(ack.status, lcilib::db::AckStatus::Posted | lcilib::db::AckStatus::External))
            .map(|ack| format!("- {}: {}", ack.reviewer_name, ack.message))
            .collect();
        
        let description = if ack_lines.is_empty() {
            format!("Merge PR #{}: {}", pr.pr_number, pr.title)
        } else {
            format!("Merge PR #{}: {}\n\nACKs:\n{}", pr.pr_number, pr.title, ack_lines.join("\n"))
        };
        
        // Create merge commit using jj
        let shell = Shell::new().context("creating shell")?;
        shell.change_dir(&repo.path);
        
        let change_id = match lcilib::jj::create_merge_commit(&shell, &tip_commit.git_commit_id, &pr.target_branch, &description) {
            Ok(id) => id,
            Err(e) => {
                log::warn(format_args!("Failed to create merge commit for PR #{}: {}", pr.pr_number, e));
                tx.commit().await.context("committing transaction")?;
                return Ok(false);
            }
        };
        
        let git_commit_id = lcilib::jj::get_current_git_commit_for_change_id(&shell, &change_id)
            .context("getting git commit ID for merge")?;
        
        // Create commit record
        let new_commit = NewCommit {
            repository_id: pr.repository_id,
            git_commit_id: git_commit_id.parse().context("parsing git commit ID")?,
            jj_change_id: change_id,
            review_status: ReviewStatus::Approved,
            should_run_ci: true,
            ci_status: CiStatus::Unstarted,
            nix_derivation: None,
            review_text: Some(format!("Merge commit for PR #{}", pr.pr_number)),
        };
        
        let commit = Commit::create(&tx, new_commit).await
            .map_err(|e| anyhow::anyhow!("Failed to create merge commit record: {}", e))?;
        
        // Add to PR
        lcilib::db::models::PrCommit::create(&tx, pr.id, commit.id, 999, CommitType::Merge).await
            .map_err(|e| anyhow::anyhow!("Failed to create pr_commit record: {}", e))?;
        
        commit
    };
    
    // Try to extend existing stacks
    let mut extended_stack = false;
    let stacks = Stack::find_all(&tx).await
        .map_err(|e| anyhow::anyhow!("Failed to find stacks: {}", e))?;
    
    let matching_stacks: Vec<_> = stacks.into_iter()
        .filter(|s| s.repository_id == pr.repository_id && s.target_branch == pr.target_branch)
        .collect();
    
    for stack in matching_stacks {
        if try_extend_stack(&tx, &stack, pr, &merge_commit, repo).await? {
            extended_stack = true;
            break;
        }
    }
    
    // If no stack was extended, create new stack
    if !extended_stack {
        let new_stack = NewStack {
            repository_id: pr.repository_id,
            target_branch: pr.target_branch.clone(),
        };
        
        let stack = Stack::create(&tx, new_stack).await
            .map_err(|e| anyhow::anyhow!("Failed to create stack: {}", e))?;
        
        stack.add_commit(&tx, merge_commit.id, 1).await
            .map_err(|e| anyhow::anyhow!("Failed to add commit to stack: {}", e))?;
        
        log::info(format_args!("Created new stack for PR #{}", pr.pr_number));
    }
    
    tx.commit().await.context("committing PR transaction")?;
    Ok(true)
}

async fn try_extend_stack(
    tx: &lcilib::Transaction<'_>,
    stack: &Stack,
    pr: &PullRequest,
    merge_commit: &Commit,
    repo: &Repository,
) -> anyhow::Result<bool> {
    // Check if stack already has a merge for this PR
    let existing = tx.query_opt(
        r#"
        SELECT 1 FROM stack_commits sc
        JOIN commits c ON sc.commit_id = c.id
        JOIN pr_commits pc ON c.id = pc.commit_id
        WHERE sc.stack_id = $1 AND pc.pull_request_id = $2 AND pc.commit_type = 'merge'
        "#,
        &[&stack.id, &pr.id],
    ).await.context("checking for existing merge in stack")?;
    
    if existing.is_some() {
        return Ok(true); // Already in stack
    }
    
    // Get current stack commits
    let stack_commits = stack.get_commits(tx).await
        .map_err(|e| anyhow::anyhow!("Failed to get stack commits: {}", e))?;
    
    // Try to create merge on top of stack
    let shell = Shell::new().context("creating shell")?;
    shell.change_dir(&repo.path);
    
    let parent_commit = if let Some(last_commit) = stack_commits.last() {
        &last_commit.jj_change_id
    } else {
        &pr.target_branch
    };
    
    let acks = lcilib::db::models::Ack::find_by_pull_request(tx, pr.id).await
        .map_err(|e| anyhow::anyhow!("Failed to get ACKs: {}", e))?;
    
    let ack_lines: Vec<String> = acks.iter()
        .filter(|ack| matches!(ack.status, lcilib::db::AckStatus::Posted | lcilib::db::AckStatus::External))
        .map(|ack| format!("- {}: {}", ack.reviewer_name, ack.message))
        .collect();
    
    let description = if ack_lines.is_empty() {
        format!("Merge PR #{}: {}", pr.pr_number, pr.title)
    } else {
        format!("Merge PR #{}: {}\n\nACKs:\n{}", pr.pr_number, pr.title, ack_lines.join("\n"))
    };
    
    match lcilib::jj::create_merge_commit(&shell, &merge_commit.git_commit_id, parent_commit, &description) {
        Ok(change_id) => {
            // Success - add to stack
            let next_order = stack_commits.len() as i32 + 1;
            stack.add_commit(tx, merge_commit.id, next_order).await
                .map_err(|e| anyhow::anyhow!("Failed to add commit to stack: {}", e))?;
            
            log::info(format_args!("Extended stack {} with PR #{}", stack.id, pr.pr_number));
            Ok(true)
        }
        Err(e) => {
            log::info(format_args!("Failed to extend stack {} with PR #{}: {}", stack.id, pr.pr_number, e));
            Ok(false)
        }
    }
}

async fn process_existing_stacks(
    db: &mut Db,
    repo_map: &HashMap<i32, Repository>,
) -> anyhow::Result<bool> {
    let mut work_done = false;
    
    // Get all stacks grouped by repo/target
    let tx = db.transaction().await.context("starting stacks transaction")?;
    let all_stacks = Stack::find_all(&tx).await
        .map_err(|e| anyhow::anyhow!("Failed to find stacks: {}", e))?;
    tx.commit().await.context("committing stacks query")?;
    
    let mut stacks_by_repo_target: HashMap<(i32, String), Vec<Stack>> = HashMap::new();
    for stack in all_stacks {
        let key = (stack.repository_id, stack.target_branch.clone());
        stacks_by_repo_target.entry(key).or_default().push(stack);
    }
    
    // Process each group
    for ((repo_id, _target_branch), mut stacks) in stacks_by_repo_target {
        let Some(repo) = repo_map.get(&repo_id) else {
            continue;
        };
        
        // Sort by priority (highest first)
        let tx = db.transaction().await.context("starting stack priority transaction")?;
        let repo_ref_map: HashMap<i32, &Repository> = std::iter::once((repo_id, repo)).collect();
        
        let mut stack_priorities = Vec::new();
        for stack in &stacks {
            let commits = stack.get_commits(&tx).await
                .map_err(|e| anyhow::anyhow!("Failed to get stack commits: {}", e))?;
            let priority = util::calculate_stack_priority(&commits, &tx, &repo_ref_map).await
                .context("calculating stack priority")?;
            stack_priorities.push((stack, priority));
        }
        tx.commit().await.context("committing stack priority transaction")?;
        
        stack_priorities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        stacks = stack_priorities.into_iter().map(|(s, _)| s.clone()).collect();
        
        // Process each stack
        for stack in stacks {
            if process_stack_updates(db, &stack, repo).await? {
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
    let tx = db.transaction().await.context("starting stack update transaction")?;
    
    let stack_commits = stack.get_commits(&tx).await
        .map_err(|e| anyhow::anyhow!("Failed to get stack commits: {}", e))?;
    
    if stack_commits.is_empty() {
        tx.commit().await.context("committing empty stack transaction")?;
        return Ok(false);
    }
    
    let shell = Shell::new().context("creating shell")?;
    shell.change_dir(&repo.path);
    
    // Check if first commit's first parent matches target
    let first_commit = &stack_commits[0];
    let parents = lcilib::git::list_parents(&shell, &first_commit.git_commit_id)
        .context("getting commit parents")?;
    
    let target_commit = lcilib::git::resolve_ref(&shell, &stack.target_branch)
        .context("resolving target branch")?;
    
    let needs_rebase = parents.is_empty() || parents[0].to_string() != target_commit.to_string();
    
    if needs_rebase {
        log::info(format_args!("Stack {} needs rebase due to target branch change", stack.id));
        
        // Check for GPG signatures before rebasing
        for commit in &stack_commits {
            if util::is_commit_gpg_signed(commit, &repo.path).await? {
                log::warn(format_args!("Warning: Throwing away GPG signature for commit {} due to rebase", commit.jj_change_id));
            }
        }
        
        // Mark all commits as not current and delete stack
        for commit in &stack_commits {
            tx.execute(
                "UPDATE pr_commits SET is_current = false WHERE commit_id = $1",
                &[&commit.id],
            ).await.context("marking commits as not current")?;
        }
        
        tx.execute("DELETE FROM stack_commits WHERE stack_id = $1", &[&stack.id])
            .await.context("deleting stack commits")?;
        tx.execute("DELETE FROM stacks WHERE id = $1", &[&stack.id])
            .await.context("deleting stack")?;
        
        tx.commit().await.context("committing rebase transaction")?;
        
        // TODO: Recreate stack in correct order - this is complex and would need
        // to re-run the extend_stack logic for each PR in the original order
        return Ok(true);
    }
    
    // Update descriptions and check for commit changes
    let mut work_done = false;
    for commit in &stack_commits {
        // Update description (dummy implementation for now)
        if let Err(e) = lcilib::jj::update_commit_description(&shell, &commit.jj_change_id, &format!("Updated description for {}", commit.jj_change_id)) {
            log::warn(format_args!("Failed to update description for commit {}: {}", commit.jj_change_id, e));
        }
        
        // Check if git commit ID has changed
        match lcilib::jj::get_current_git_commit_for_change_id(&shell, &commit.jj_change_id) {
            Ok(current_git_id) => {
                if current_git_id != commit.git_commit_id {
                    // Get tree hash and parents to check what changed
                    let current_tree = lcilib::git::resolve_ref(&shell, &format!("{}^{{tree}}", current_git_id))
                        .context("getting current tree hash")?;
                    let original_tree = lcilib::git::resolve_ref(&shell, &format!("{}^{{tree}}", commit.git_commit_id))
                        .context("getting original tree hash")?;
                    
                    let current_parents = lcilib::git::list_parents(&shell, &current_git_id)
                        .context("getting current parents")?;
                    let original_parents = lcilib::git::list_parents(&shell, &commit.git_commit_id)
                        .context("getting original parents")?;
                    
                    if current_tree.to_string() != original_tree.to_string() || 
                       current_parents.len() != original_parents.len() ||
                       current_parents.iter().zip(original_parents.iter()).any(|(a, b)| a.to_string() != b.to_string()) {
                        // Tree or parents changed - mark as not current
                        log::warn(format_args!("Commit {} tree or parents changed, marking as not current", commit.jj_change_id));
                        tx.execute(
                            "UPDATE pr_commits SET is_current = false WHERE commit_id = $1",
                            &[&commit.id],
                        ).await.context("marking commit as not current")?;
                        work_done = true;
                    } else {
                        // Only commit ID changed - update it
                        tx.execute(
                            "UPDATE commits SET git_commit_id = $1 WHERE id = $2",
                            &[&current_git_id, &commit.id],
                        ).await.context("updating git commit ID")?;
                        
                        // Check for lost GPG signature
                        let was_signed = util::is_commit_gpg_signed(commit, &repo.path).await?;
                        if was_signed {
                            let shell_for_current = Shell::new().context("creating shell for current commit")?;
                            shell_for_current.change_dir(&repo.path);
                            let is_signed = lcilib::jj::is_commit_gpg_signed(&shell_for_current, &commit.jj_change_id)?;
                            if !is_signed {
                                log::warn(format_args!("Warning: Throwing away GPG signature for commit {} due to description change", commit.jj_change_id));
                            }
                        }
                        work_done = true;
                    }
                }
            }
            Err(e) => {
                log::warn(format_args!("Failed to get current git commit for {}: {}", commit.jj_change_id, e));
            }
        }
    }
    
    tx.commit().await.context("committing stack update transaction")?;
    Ok(work_done)
}

async fn check_signed_merges(_db: &mut Db) -> anyhow::Result<bool> {
    // TODO: Query for signed merge commits that passed CI
    // TODO: Push them
    // Return true if work was done, false if nothing to do
    time::sleep(Duration::from_secs(15)).await;
    Ok(false)
}

async fn get_repository_for_commit(db: &mut Db, commit: &Commit) -> anyhow::Result<Repository> {
    let tx = db.transaction().await.context("starting transaction")?;
    let repo = Repository::find_by_id(&tx, commit.repository_id).await
        .map_err(|e| anyhow::anyhow!("Database error: {}", e))?
        .ok_or_else(|| anyhow::anyhow!("Repository not found for ID: {}", commit.repository_id))?;
    tx.commit().await.context("committing transaction")?;
    Ok(repo)
}

async fn mark_commit_failed(db: &mut Db, commit: &Commit, reason: &str) -> anyhow::Result<()> {
    let tx = db.transaction().await.context("starting transaction")?;
    let updates = UpdateCommit {
        ci_status: Some(CiStatus::Failed),
        review_text: Some(Some(reason.to_string())),
        ..Default::default()
    };
    commit.update(&tx, updates).await
        .map_err(|e| anyhow::anyhow!("Failed to update commit: {}", e))?;
    tx.commit().await.context("committing transaction")?;
    Ok(())
}

async fn mark_commit_passed(db: &mut Db, commit: &Commit) -> anyhow::Result<()> {
    let tx = db.transaction().await.context("starting transaction")?;
    let updates = UpdateCommit {
        ci_status: Some(CiStatus::Passed),
        ..Default::default()
    };
    commit.update(&tx, updates).await
        .map_err(|e| anyhow::anyhow!("Failed to update commit: {}", e))?;
    tx.commit().await.context("committing transaction")?;
    Ok(())
}

async fn sync_all_repositories(db: &mut Db) -> anyhow::Result<()> {
    let tx = db.transaction().await
        .context("starting transaction to get repositories")?;
    
    let repositories = Repository::list_all(&tx).await
        .map_err(|e| anyhow::anyhow!("Failed to get repositories: {}", e))?;
    
    tx.commit().await
        .context("committing transaction")?;
    
    for repo in repositories {
        if let Err(e) = sync_repository_prs(db, &repo).await {
            log::warn(format_args!("Failed to sync PRs for repository {}: {}", repo.name, e));
        }
    }
    
    Ok(())
}

async fn sync_repository_prs(db: &mut Db, repo: &Repository) -> anyhow::Result<()> {
    struct Data {
        current_repo: lcilib::repo::Repository,
        pr_infos: Vec<lcilib::gh::PrInfo>,
    }

    // Use spawn_blocking for the GitHub API calls and shell operations
    let repo_path = repo.path.clone();
    let repo_name = repo.name.clone();
    let last_synced = repo.last_synced_at;
    
    let data = task::spawn_blocking(move || -> Option<Data> {
        // Check if repository path exists
        let shell = Shell::new().ok()?; // just eat shell creation error; this basically cannot happen
        if !Path::new(&repo_path).exists() {
            log::warn(format_args!("Warning: Repository path does not exist: {}", repo_path));
            return None;
        }
        shell.change_dir(&repo_path);
        
        // Fetch latest changes from git remote
        if let Err(e) = cmd!(shell, "git fetch origin").run() {
            let error = anyhow::Error::from(e);
            log::warn(format_args!(
                "Failed to run 'git fetch origin' in repository {}: {:?}",
                repo_name, error
            ));
        }
        
        // Fetch changes into jj
        if let Err(e) = cmd!(shell, "jj git fetch").run() {
            let error = anyhow::Error::from(e);
            log::warn(format_args!(
                "Failed to run 'jj git fetch' in repository {}: {:?}",
                repo_name, error
            ));
        }
        
        // Get updated PRs from GitHub
        let pr_infos = match lcilib::gh::list_updated_prs(&shell, last_synced)  {
            Ok(prs) => prs,
            Err(e) => {
                log::warn(format_args!("Warning: Failed to get updated PRs for repository {}: {}", repo_path, e));
                return None;
            }
        };

        let current_repo = match lcilib::repo::current_repo(&shell) {
            Ok(repo) => repo,
            Err(e) => {
                log::warn(format_args!("Warning: failed to get current repo for repository path {}: {}", repo_path, e));
                return None;
            }
        };

        Some(Data { current_repo, pr_infos })
    }).await
    .context("spawning blocking task for GitHub API calls")?;

    let Some(data) = data else { return Ok(()) };
    if !data.pr_infos.is_empty() {
        log::info(format_args!("Found {} updated PRs for repository {}", data.pr_infos.len(), repo.name));
    }
    for pr_info in &data.pr_infos {
        // A shell cannot live across await points so we have to recreate it on every iteration
        // and hand ownership to the refresh function.
        let shell = Shell::new().expect("this just worked above..");
        shell.change_dir(&repo.path);
        if let Err(e) = crate::pr::refresh(shell, &data.current_repo, pr_info, db).await
            .with_context(|| format!("failed to refresh PR #{}", pr_info.number))
        {
            log::warn(format_args!("Warning: Failed to process PR #{} in repository {}: {}", 
                pr_info.number, repo.name, e));
        } else {
            log::info(format_args!("Successfully synced PR #{} in repository {}", pr_info.number, repo.name));
        }
    }
    
    // Update last synced time
    let tx = db.transaction().await
        .context("starting transaction to update last synced time")?;
    
    repo.update_last_synced(&tx).await
        .map_err(|e| anyhow::anyhow!("Failed to update last synced time: {}", e))?;
    
    tx.commit().await
        .context("committing transaction")?;
    
    Ok(())
}
