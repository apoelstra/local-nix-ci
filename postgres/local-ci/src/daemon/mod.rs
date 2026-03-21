// SPDX-License-Identifier: GPL-3.0-or-later

mod ci_cycle;
mod log;

use anyhow::Context as _;
use lcilib::Db;
use lcilib::db::models::{Commit, Repository, UpdateCommit, CiStatus};
use std::path::Path;
use std::time::Duration;
use tokio::{time, task};
use xshell::Shell;

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

async fn check_approved_prs(_db: &mut Db) -> anyhow::Result<bool> {
    // TODO: Query for approved but unmerged PRs
    // TODO: Create merge commit on highest-priority non-conflicting stack
    // TODO: Create individual stack for the merge
    // TODO: Handle rebase warnings
    // Return true if work was done, false if nothing to do
    time::sleep(Duration::from_secs(15)).await;
    Ok(false)
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
    let last_synced = repo.last_synced_at;
    
    let data = task::spawn_blocking(move || -> Option<Data> {
        // Check if repository path exists
        let shell = Shell::new().ok()?; // just eat shell creation error; this basically cannot happen
        if !Path::new(&repo_path).exists() {
            log::warn(format_args!("Warning: Repository path does not exist: {}", repo_path));
            return None;
        }
        shell.change_dir(&repo_path);
        
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
