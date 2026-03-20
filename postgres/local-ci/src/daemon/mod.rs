mod stacks;

use anyhow::Context as _;
use lcilib::Db;
use std::time::Duration;
use tokio::time;

pub async fn run(_db: &mut Db) -> anyhow::Result<()> {
    println!("Starting local-ci daemon...");
    
    // Start all cycles concurrently, each with its own database connection
    let tasks = vec![
        tokio::spawn(run_ack_cycle()),
        tokio::spawn(run_pr_cycle()),
        tokio::spawn(run_merge_cycle()),
        tokio::spawn(run_ci_cycle_loop()),
    ];
    
    // Wait for all tasks to complete (which should never happen)
    for task in tasks {
        task.await
            .context("daemon task failed")??;
    }
    
    Ok(())
}

async fn run_ack_cycle() -> anyhow::Result<()> {
    let mut db = Db::connect().await
        .context("connecting to database for ACK cycle")?;
    
    loop {
        match check_pending_acks(&mut db).await {
            Ok(had_work) => {
                if !had_work {
                    time::sleep(Duration::from_secs(5)).await;
                }
            }
            Err(e) => {
                eprintln!("Error in ACK cycle: {}", e);
                time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

async fn run_pr_cycle() -> anyhow::Result<()> {
    let mut db = Db::connect().await
        .context("connecting to database for PR cycle")?;
    
    loop {
        match check_approved_prs(&mut db).await {
            Ok(had_work) => {
                if !had_work {
                    time::sleep(Duration::from_secs(5)).await;
                }
            }
            Err(e) => {
                eprintln!("Error in PR cycle: {}", e);
                time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

async fn run_merge_cycle() -> anyhow::Result<()> {
    let mut db = Db::connect().await
        .context("connecting to database for merge cycle")?;
    
    loop {
        match check_signed_merges(&mut db).await {
            Ok(had_work) => {
                if !had_work {
                    time::sleep(Duration::from_secs(5)).await;
                }
            }
            Err(e) => {
                eprintln!("Error in merge cycle: {}", e);
                time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

async fn run_ci_cycle_loop() -> anyhow::Result<()> {
    let mut db = Db::connect().await
        .context("connecting to database for CI cycle")?;
    
    loop {
        match run_ci_cycle(&mut db).await {
            Ok(had_work) => {
                if !had_work {
                    time::sleep(Duration::from_secs(5)).await;
                }
            }
            Err(e) => {
                eprintln!("Error in CI cycle: {}", e);
                time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

async fn check_pending_acks(_db: &mut Db) -> anyhow::Result<bool> {
    // TODO: Query for PRs with pending ACKs
    // TODO: Check if all non-merge commits are approved and passed CI
    // TODO: Post ACK and update status to 'posted'
    // Return true if work was done, false if nothing to do
    Ok(false)
}

async fn check_approved_prs(_db: &mut Db) -> anyhow::Result<bool> {
    // TODO: Query for approved but unmerged PRs
    // TODO: Create merge commit on highest-priority non-conflicting stack
    // TODO: Create individual stack for the merge
    // TODO: Handle rebase warnings
    // Return true if work was done, false if nothing to do
    Ok(false)
}

async fn check_signed_merges(_db: &mut Db) -> anyhow::Result<bool> {
    // TODO: Query for signed merge commits that passed CI
    // TODO: Push them
    // Return true if work was done, false if nothing to do
    Ok(false)
}

async fn run_ci_cycle(db: &mut Db) -> anyhow::Result<bool> {
    match stacks::find_next_commit_to_test(db).await
        .context("finding next commit to test")? {
        Some(commit) => {
            // TODO: Actually start CI for this commit
            println!("Would start CI for commit: {} ({})", commit.git_commit_id, commit.jj_change_id);
            Ok(true)
        }
        None => {
            // No work to do
            Ok(false)
        }
    }
}
