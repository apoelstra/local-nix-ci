// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Context as _;
use chrono::{DateTime, Utc};
use lcilib::{
    Db,
    db::{models::{Repository, PullRequest, NewRepository, Commit, CommitType, NewCommit, ReviewStatus, CiStatus, UpdatePullRequest, NewPullRequest, PrCommit, Ack, NewAck, AckStatus}, EntityType, Log},
    gh,
    git,
    jj,
    repo,
};
use std::{env, fs, io::{self, Write}, collections::{HashMap, HashSet}};
use xshell::{Shell, cmd};

/// Show information about a PR
/// 
/// # Errors
/// 
/// Returns an error if:
/// - Failed to get current repository information
/// - Database transaction fails
/// - Repository or PR lookup fails
pub async fn info(pr_number: usize, db: &mut Db) -> anyhow::Result<()> {
    let shell = Shell::new()?;
    let current_repo = repo::current_repo(&shell)
        .context("failed to get current repository")?;

    let tx = db.transaction().await
        .context("failed to start database transaction")?;

    // Find the repository in the database
    let Some(repo_record) = Repository::find_by_path(&tx, current_repo.repo_root.to_str().unwrap()).await
        .context("failed to query repository")?
    else {
        println!("Repository not found in database. Please run 'refresh' first to initialize it.");
        return Ok(());
    };

    // Look up the PR in the database
    if let Some(pr) = PullRequest::find_by_number(&tx, repo_record.id, pr_number.try_into()?).await
        .context("failed to query pull request")?
    {
        println!("{} PR #{}: {}", current_repo.project_name, pr.pr_number, pr.title);
        println!();
        println!("{}", if pr.body.is_empty() { "(empty)" } else { &pr.body });
        println!();
        println!("Review Status: {:?}", pr.review_status);
        println!("Priority: {}", pr.priority);
        println!("OK to Merge: {}", pr.ok_to_merge);
        println!("Required Reviewers: {}", pr.required_reviewers);
        println!("Created: {}", pr.created_at);
        println!("Updated: {}", pr.updated_at);
        println!("Last Synced: {}", pr.synced_at);

        // Show commits
        let commits = pr.get_commits(&tx).await
            .context("failed to get PR commits")?;
        
        if !commits.is_empty() {
            println!("\nCommits:");
            for (i, (commit, commit_type)) in commits.iter().enumerate() {
                println!("  {}. {} ({:?}) - Review: {:?}, CI: {:?}", 
                    i + 1, 
                    commit.git_commit_id, 
                    commit_type,
                    commit.review_status,
                    commit.ci_status
                );
            }
        }

        // Show previous tips
        let previous_tips = pr.get_previous_tips(&tx).await
            .context("failed to get previous tip commits")?;
        
        if !previous_tips.is_empty() {
            println!("\nPrevious tip commits:");
            for tip in previous_tips.iter() {
                println!("  {}", tip.git_commit_id);
            }
        }

        // Show ACKs
        let acks = Ack::find_by_pull_request(&tx, pr.id).await
            .context("failed to find ACKs for PR")?;

        if !acks.is_empty() {
            println!("\nACKs:");
            for ack in acks {
                println!("  {} by {} ({}): {}", 
                    ack.created_at.format("%Y-%m-%d %H:%M:%S"),
                    ack.reviewer_name,
                    ack.status,
                    ack.message
                );
            }
        } else {
            println!("\nACKs: None");
        }

        // Show next action
        let next_action = determine_next_action_for_pr(&tx, &pr).await
            .context("failed to determine next action")?;
        println!("\nNext action: {}", next_action);
    } else {
        println!("PR #{} not found in database.", pr_number);
        println!("Use 'local-ci refresh pr {}' to download it from GitHub.", pr_number);
    }

    tx.commit().await
        .context("failed to commit transaction")?;

    Ok(())
}

/// Determine the next action for a PR
async fn determine_next_action_for_pr(
    tx: &lcilib::Transaction<'_>,
    pr: &PullRequest,
) -> anyhow::Result<String> {
    // Get current commits for this PR in sequence order
    let commits = pr.get_commits(tx).await
        .context("failed to get PR commits")?;

    // Find the first unreviewed commit
    for (commit, _) in &commits {
        if commit.review_status == ReviewStatus::Unreviewed {
            return Ok(format!("local-ci review commit {}", commit.git_commit_id));
        }
    }

    // All commits are reviewed, check if PR itself needs review
    if pr.review_status == ReviewStatus::Unreviewed {
        return Ok(format!("local-ci review pr {}", pr.pr_number));
    }

    // Nothing to do
    Ok("none".to_string())
}

/// Execute the next action for a PR
/// 
/// # Errors
/// 
/// Returns an error if:
/// - Failed to get current repository information
/// - Database transaction fails
/// - Repository or PR lookup fails
/// - No next action available
pub async fn next(pr_number: usize, db: &mut Db) -> anyhow::Result<()> {
    let shell = Shell::new()?;
    let current_repo = repo::current_repo(&shell)
        .context("failed to get current repository")?;

    let tx = db.transaction().await
        .context("failed to start database transaction")?;

    // Find the repository in the database
    let Some(repo_record) = Repository::find_by_path(&tx, current_repo.repo_root.to_str().unwrap()).await
        .context("failed to query repository")?
    else {
        anyhow::bail!("Repository not found in database. Please run 'refresh' first to initialize it.");
    };

    // Look up the PR in the database
    let Some(pr) = PullRequest::find_by_number(&tx, repo_record.id, pr_number.try_into()?).await
        .context("failed to query pull request")?
    else {
        anyhow::bail!("PR #{} not found in database. Use 'local-ci refresh pr {}' to download it from GitHub.", pr_number, pr_number);
    };

    // Get current commits for this PR in sequence order
    let commits = pr.get_commits(&tx).await
        .context("failed to get PR commits")?;

    // Find the first unreviewed commit
    for (commit, _) in &commits {
        if commit.review_status == ReviewStatus::Unreviewed {
            // Drop the transaction since we're going to call review which will start its own
            tx.commit().await
                .context("failed to commit transaction")?;
            
            // Review this commit
            return crate::commit::review(&commit.git_commit_id, db).await
                .context("failed to review commit");
        }
    }

    // All commits are reviewed, check if PR itself needs review
    if pr.review_status == ReviewStatus::Unreviewed {
        // Drop the transaction since we're going to call review which will start its own
        tx.commit().await
            .context("failed to commit transaction")?;
        
        // Review the PR
        return review(pr_number, db).await
            .context("failed to review PR");
    }

    // Nothing to do
    tx.commit().await
        .context("failed to commit transaction")?;
    
    println!("Nothing to do");
    Ok(())
}

/// Scan GitHub comments and reviews for ACKs and update the database
async fn scan_and_update_acks(
    tx: &lcilib::Transaction<'_>,
    pr_info: &gh::PrInfo,
    pr_record: &PullRequest,
    commit_records: &[Commit],
) -> anyhow::Result<()> {
    use std::collections::HashMap;
    use chrono::{DateTime, Utc};

    // Build a map of commit ID prefixes to commit records for fast lookup
    let mut commit_map = HashMap::new();
    for commit in commit_records {
        let commit_id = &commit.git_commit_id;
        // Add all possible prefixes of 7+ characters
        for len in 7..=commit_id.len() {
            let prefix = &commit_id[..len];
            commit_map.insert(prefix.to_string(), commit);
        }
    }

    // Collect all ACKs from comments and reviews
    let mut found_acks: HashMap<String, (String, String, DateTime<Utc>, i32)> = HashMap::new(); // reviewer -> (message, reviewer, timestamp, commit_id)

    // Scan comments
    for comment in &pr_info.comments {
        if let Some((ack_text, commit_id)) = extract_ack_from_text(&comment.body, &commit_map) {
            let timestamp = parse_github_timestamp(&comment.created_at)?;
            let reviewer = &comment.author.login;
            
            // Keep the latest ACK per reviewer
            if let Some((_, _, existing_timestamp, _)) = found_acks.get(reviewer) {
                if timestamp > *existing_timestamp {
                    found_acks.insert(reviewer.clone(), (ack_text, reviewer.clone(), timestamp, commit_id));
                }
            } else {
                found_acks.insert(reviewer.clone(), (ack_text, reviewer.clone(), timestamp, commit_id));
            }
        }
    }

    // Scan reviews
    for review in &pr_info.reviews {
        if let Some((ack_text, commit_id)) = extract_ack_from_text(&review.body, &commit_map) {
            let timestamp = parse_github_timestamp(&review.submitted_at)?;
            let reviewer = &review.author.login;
            
            // Keep the latest ACK per reviewer
            if let Some((_, _, existing_timestamp, _)) = found_acks.get(reviewer) {
                if timestamp > *existing_timestamp {
                    found_acks.insert(reviewer.clone(), (ack_text, reviewer.clone(), timestamp, commit_id));
                }
            } else {
                found_acks.insert(reviewer.clone(), (ack_text, reviewer.clone(), timestamp, commit_id));
            }
        }
    }

    // Get all existing ACKs for this PR
    let existing_acks = Ack::find_by_pull_request(tx, pr_record.id).await
        .context("failed to get existing ACKs")?;
    
    let current_user = "apoelstra"; // FIXME: should use per-repository git configuration
    
    // Separate existing ACKs by reviewer and status
    let mut existing_external_acks = HashMap::new();
    let mut existing_user_acks = HashMap::new();
    
    for ack in existing_acks {
        if ack.reviewer_name == current_user {
            // Group current user's ACKs by message
            existing_user_acks.insert(ack.message.clone(), ack);
        } else if ack.status == AckStatus::External {
            let key = format!("{}:{}", ack.reviewer_name, ack.message);
            existing_external_acks.insert(key, ack);
        }
    }

    // Process found ACKs
    for (ack_text, reviewer, _timestamp, commit_id) in found_acks.values() {
        if reviewer == current_user {
            // Handle current user's ACKs with special logic
            if let Some(existing_user_ack) = existing_user_acks.get(ack_text) {
                // If we have a pending/failed ACK with identical text, upgrade it to posted
                if existing_user_ack.status == AckStatus::Pending || existing_user_ack.status == AckStatus::Failed {
                    let update = lcilib::db::models::UpdateAck {
                        status: Some(AckStatus::Posted),
                        ..Default::default()
                    };
                    existing_user_ack.update(tx, update).await
                        .context("failed to update ACK status to posted")?;
                }
                // If text is identical and status is already posted/external, do nothing
            } else {
                // Check if we have any pending/failed ACK with different text
                let has_pending_or_failed = existing_user_acks.values()
                    .any(|ack| ack.status == AckStatus::Pending || ack.status == AckStatus::Failed);
                
                if !has_pending_or_failed {
                    // No conflicting pending/failed ACK, create new external ACK
                    let new_ack = NewAck {
                        pull_request_id: pr_record.id,
                        commit_id: *commit_id,
                        reviewer_name: reviewer.clone(),
                        message: ack_text.clone(),
                        status: AckStatus::External,
                    };
                    
                    Ack::create(tx, new_ack).await
                        .context("failed to create external ACK")?;
                }
                // If there is a pending/failed ACK with different text, drop this external one
            }
        } else {
            // Handle other users' ACKs normally
            let key = format!("{}:{}", reviewer, ack_text);
            if !existing_external_acks.contains_key(&key) {
                let new_ack = NewAck {
                    pull_request_id: pr_record.id,
                    commit_id: *commit_id,
                    reviewer_name: reviewer.clone(),
                    message: ack_text.clone(),
                    status: AckStatus::External,
                };
                
                Ack::create(tx, new_ack).await
                    .context("failed to create external ACK")?;
            }
        }
    }

    // Delete external ACKs that are no longer present in GitHub
    let found_keys: HashSet<String> = found_acks.values()
        .filter(|(_, reviewer, _, _)| *reviewer != current_user) // Don't delete current user's ACKs
        .map(|(ack_text, reviewer, _, _)| format!("{}:{}", reviewer, ack_text))
        .collect();
    
    Ack::delete_external_acks_not_in_set(tx, pr_record.id, &found_keys).await
        .context("failed to delete obsolete external ACKs")?;

    Ok(())
}

/// Extract ACK text and commit ID from a GitHub comment/review body
fn extract_ack_from_text(text: &str, commit_map: &HashMap<String, &Commit>) -> Option<(String, i32)> {
    for line in text.lines() {
        if let Some((ack_text, commit_id)) = extract_ack_from_line(line, commit_map) {
            return Some((ack_text, commit_id));
        }
    }
    None
}

/// Extract ACK text and commit ID from a single line
fn extract_ack_from_line(line: &str, commit_map: &HashMap<String, &Commit>) -> Option<(String, i32)> {
    // Split line into alphanumeric words (punctuation acts as separator)
    let words: Vec<&str> = line
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    
    // Look for words ending in "ACK" (case sensitive for ACK part)
    let mut ack_word_pos = None;

    for (i, word) in words.iter().enumerate() {
        if word.ends_with("ACK") && !word.ends_with("NACK") && !word.ends_with("nACK") {
            ack_word_pos = Some(i);
            break;
        }
    }
    
    let ack_pos = ack_word_pos?;
    
    // Look for commit IDs (7+ lowercase hex characters) in the same line, occurring after the ACK word
    for word in words.iter().skip(ack_pos) {
        if word.len() >= 7 && word.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()) {
            if let Some(commit) = commit_map.get(*word) {
                // Found a valid commit ID, construct the ACK text
                let ack_text = line.trim().to_string();
                return Some((ack_text, commit.id));
            }
        }
    }
    
    None
}

/// Parse GitHub timestamp string to DateTime<Utc>
fn parse_github_timestamp(timestamp: &str) -> anyhow::Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(timestamp)
        .map(|dt| dt.with_timezone(&Utc))
        .with_context(|| format!("failed to parse GitHub timestamp: {}", timestamp))
}

/// Interactive review of a PR
/// 
/// # Errors
/// 
/// Returns an error if:
/// - Failed to get current repository information
/// - Database transaction fails
/// - Repository or PR lookup fails
/// - Editor invocation fails
pub async fn review(pr_number: usize, db: &mut Db) -> anyhow::Result<()> {
    let shell = Shell::new()?;
    let current_repo = repo::current_repo(&shell)
        .context("failed to get current repository")?;

    let tx = db.transaction().await
        .context("failed to start database transaction")?;

    // Find the repository in the database
    let Some(repo_record) = Repository::find_by_path(&tx, current_repo.repo_root.to_str().unwrap()).await
        .context("failed to query repository")?
    else {
        anyhow::bail!("Repository not found in database. Please run 'refresh' first to initialize it.");
    };

    // Look up the PR in the database
    let Some(pr) = PullRequest::find_by_number(&tx, repo_record.id, pr_number.try_into()?).await
        .context("failed to query pull request")?
    else {
        anyhow::bail!("PR #{} not found in database. Use 'local-ci refresh pr {}' to download it from GitHub.", pr_number, pr_number);
    };

    // Get the tip commit
    let Some(tip_commit) = Commit::find_by_id(&tx, pr.tip_commit_id).await
        .context("failed to find tip commit")?
    else {
        anyhow::bail!("Tip commit not found for PR #{}", pr_number);
    };

    // Show PR info first
    show_pr_info(&shell, &current_repo, &pr, &tip_commit).await?;

    loop {
        // Show menu
        println!("\nWhat would you like to do?");
        println!("1a) ACK (approve)");
        println!("1b) NACK (reject)");
        println!();
        println!("2a) View existing ACKs");
        println!("2b) Erase ACK");
        println!();
        println!("3a) View diff (50-line context)");
        println!("3b) View diff (3-line context)");
        println!("3c) View diff (5000-line context)");
        println!("3d) View diff (stat)");
        println!();
        println!("4) Cancel");
        print!("Choice (1a-4): ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let choice = input.trim();

        match choice {
            "1a" => {
                if let Some(ack_message) = handle_ack_with_editor(&tip_commit, true).await? {
                    create_or_overwrite_ack(&tx, pr.id, tip_commit.id, &ack_message).await
                        .context("failed to create ACK")?;
                    
                    // Update PR review status to Approved
                    let updates = UpdatePullRequest {
                        review_status: Some(ReviewStatus::Approved),
                        ..Default::default()
                    };
                    pr.update(&tx, updates).await
                        .context("failed to update PR review status")?;
                    
                    println!("ACK created successfully and PR marked as approved.");
                    break;
                }
            }
            "1b" => {
                if let Some(ack_message) = handle_ack_with_editor(&tip_commit, false).await? {
                    create_or_overwrite_ack(&tx, pr.id, tip_commit.id, &ack_message).await
                        .context("failed to create NACK")?;
                    
                    // Update PR review status to Rejected
                    let updates = UpdatePullRequest {
                        review_status: Some(ReviewStatus::Rejected),
                        ..Default::default()
                    };
                    pr.update(&tx, updates).await
                        .context("failed to update PR review status")?;
                    
                    println!("NACK created successfully and PR marked as rejected.");
                    break;
                }
            }
            "2a" => {
                show_existing_acks(&tx, pr.id).await?;
            }
            "2b" => {
                erase_ack(&tx, pr.id, tip_commit.id).await
                    .context("failed to erase ACK")?;
                println!("ACK erased successfully.");
            }
            "3a" => {
                show_diff(&shell, &tip_commit.git_commit_id, Some(50))?;
            }
            "3b" => {
                show_diff(&shell, &tip_commit.git_commit_id, Some(3))?;
            }
            "3c" => {
                show_diff(&shell, &tip_commit.git_commit_id, Some(5000))?;
            }
            "3d" => {
                show_diff_stat(&shell, &tip_commit.git_commit_id)?;
            }
            "4" => {
                println!("Cancelled.");
                break;
            }
            _ => {
                println!("Invalid choice. Please enter 1a, 1b, 2a, 2b, 3a, 3b, 3c, 3d, or 4.");
                continue;
            }
        }
    }

    tx.commit().await
        .context("failed to commit transaction")?;

    Ok(())
}

/// Show PR information (extracted for reuse)
async fn show_pr_info(
    shell: &Shell,
    current_repo: &repo::Repository,
    pr: &PullRequest,
    tip_commit: &Commit,
) -> anyhow::Result<()> {
    // Get commit details from git
    let commit_info = git::get_commit_info(shell, &tip_commit.git_commit_id)
        .context("failed to get commit info from git")?;

    println!("{} PR #{}: {}", current_repo.project_name, pr.pr_number, pr.title);
    println!();
    println!("{}", if pr.body.is_empty() { "(empty)" } else { &pr.body });
    println!();
    println!("Tip commit: {}", tip_commit.git_commit_id);
    println!("Author: {}", commit_info.author);
    println!("Date: {}", commit_info.date);
    println!();
    println!("{}", commit_info.message);
    println!();
    println!("Diffstat: {}", commit_info.diffstat);
    println!();
    println!("Review Status: {:?}", pr.review_status);
    println!("Priority: {}", pr.priority);
    println!("OK to Merge: {}", pr.ok_to_merge);
    println!("Required Reviewers: {}", pr.required_reviewers);
    println!("Created: {}", pr.created_at);
    println!("Updated: {}", pr.updated_at);

    Ok(())
}

/// Handle ACK/NACK with text editor
async fn handle_ack_with_editor(
    commit: &Commit,
    is_ack: bool,
) -> anyhow::Result<Option<String>> {
    // Create temporary directory and file
    let shell = Shell::new()?;
    let temp_dir = shell.create_temp_dir()
        .context("failed to create temporary directory")?;
    
    let temp_file_path = temp_dir.path().join("ack.txt");

    let action_text = if is_ack { "ACK" } else { "NACK" };
    let default_message = if is_ack {
        format!("ACK {}; successfully ran local tests", commit.git_commit_id)
    } else {
        format!("NACK {}; needs changes", commit.git_commit_id)
    };

    let prefill_content = format!(
        "# Enter your {} message here. Commit: {}\n# Edit the message above. Lines starting with # will be removed.\n{}",
        action_text,
        commit.git_commit_id,
        default_message
    );

    fs::write(&temp_file_path, prefill_content.as_bytes())
        .context("failed to write to temporary file")?;

    // Get editor from environment or default to vim
    let editor = env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());

    // Launch editor
    let result = cmd!(shell, "{editor} {temp_file_path}")
        .run()
        .with_context(|| format!("failed to launch editor: {}", editor));

    if result.is_err() {
        println!("Editor exited with error, {} cancelled.", action_text);
        return Ok(None);
    }

    // Read the edited content
    let content = fs::read_to_string(&temp_file_path)
        .context("failed to read edited file")?;

    // Remove lines starting with #
    let ack_message: String = content
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");

    Ok(Some(ack_message))
}

/// Create a new ACK record, deleting any existing ACKs by the same reviewer for this PR
async fn create_or_overwrite_ack(
    tx: &lcilib::Transaction<'_>,
    pull_request_id: i32,
    commit_id: i32,
    message: &str,
) -> anyhow::Result<()> {
    let reviewer_name = "apoelstra"; // FIXME: should use per-repository git configuration
    
    // Delete any existing ACKs by this reviewer for this PR
    let existing_acks = Ack::find_by_pull_request(tx, pull_request_id).await
        .context("failed to find existing ACKs for PR")?;
    
    for ack in existing_acks {
        if ack.reviewer_name == reviewer_name {
            ack.delete(tx).await
                .context("failed to delete existing ACK")?;
        }
    }

    // Create the new ACK
    let new_ack = NewAck {
        pull_request_id,
        commit_id,
        reviewer_name: reviewer_name.to_string(),
        message: message.to_string(),
        status: AckStatus::Pending,
    };

    Ack::create(tx, new_ack).await
        .context("failed to create ACK record")?;

    Ok(())
}

/// Show existing ACKs for a PR
async fn show_existing_acks(
    tx: &lcilib::Transaction<'_>,
    pull_request_id: i32,
) -> anyhow::Result<()> {
    let acks = Ack::find_by_pull_request(tx, pull_request_id).await
        .context("failed to find ACKs for PR")?;

    if acks.is_empty() {
        println!("No existing ACKs for this PR.");
    } else {
        println!("\nExisting ACKs:");
        for ack in acks {
            println!("  {} by {} ({}): {}", 
                ack.created_at.format("%Y-%m-%d %H:%M:%S"),
                ack.reviewer_name,
                ack.status,
                ack.message
            );
        }
    }

    Ok(())
}

/// Erase ACK for a specific commit by the current reviewer
async fn erase_ack(
    tx: &lcilib::Transaction<'_>,
    pull_request_id: i32,
    commit_id: i32,
) -> anyhow::Result<()> {
    // Find existing ACK by this reviewer for this commit
    let acks = Ack::find_by_pull_request(tx, pull_request_id).await
        .context("failed to find ACKs for PR")?;

    let reviewer_name = "apoelstra"; // FIXME: should use per-repository git configuration
    
    let matching_ack = acks.iter().find(|ack| 
        ack.commit_id == commit_id && ack.reviewer_name == reviewer_name
    );

    if let Some(ack) = matching_ack {
        // For now, we don't have a delete method, so we could update the status to indicate it's been withdrawn
        // or implement a delete method. For simplicity, let's just inform the user.
        println!("Found ACK to erase: {}", ack.message);
        println!("Note: ACK erasure not yet fully implemented - would need to add delete functionality");
    } else {
        println!("No ACK found by {} for this commit.", reviewer_name);
    }

    Ok(())
}

/// Show git diff with specified context
fn show_diff(shell: &Shell, commit_hash: &str, context: Option<u32>) -> anyhow::Result<()> {
    if let Some(lines) = context {
        let lines = lines.to_string();
        cmd!(shell, "git show --unified={lines} {commit_hash}")
            .run()
            .context("failed to run git show")?;
    } else {
        cmd!(shell, "git show {commit_hash}")
            .run()
            .context("failed to run git show")?;
    }

    Ok(())
}

/// Show git diff stat
fn show_diff_stat(shell: &Shell, commit_hash: &str) -> anyhow::Result<()> {
    cmd!(shell, "git show --stat {commit_hash}")
        .run()
        .context("failed to run git show --stat")?;

    Ok(())
}

/// Show logs for a PR and its commits
/// 
/// # Errors
/// 
/// Returns an error if:
/// - Failed to get current repository information
/// - Database transaction fails
/// - Repository or PR lookup fails
/// - Date parsing fails
pub async fn log(pr_number: usize, since: Option<&str>, until: Option<&str>, db: &mut Db) -> anyhow::Result<()> {
    let shell = Shell::new()?;
    let current_repo = repo::current_repo(&shell)
        .context("failed to get current repository")?;

    let tx = db.transaction().await
        .context("failed to start database transaction")?;

    // Find the repository in the database
    let Some(repo_record) = Repository::find_by_path(&tx, current_repo.repo_root.to_str().unwrap()).await
        .context("failed to query repository")?
    else {
        anyhow::bail!("Repository not found in database. Please run 'refresh' first to initialize it.");
    };

    // Look up the PR in the database
    let Some(pr) = PullRequest::find_by_number(&tx, repo_record.id, pr_number.try_into()?).await
        .context("failed to query pull request")?
    else {
        anyhow::bail!("PR #{} not found in database. Use 'local-ci refresh pr {}' to download it from GitHub.", pr_number, pr_number);
    };

    // Get all commits associated with this PR
    let commits = pr.get_commits(&tx).await
        .context("failed to get PR commits")?;

    // Build list of entities to query logs for (PR + all its commits)
    let mut entities = vec![(EntityType::PullRequest, pr.id)];
    for (commit, _) in &commits {
        entities.push((EntityType::Commit, commit.id));
    }

    // Query logs for all entities
    let logs = Log::query_for_entities(&tx, &entities, since, until).await
        .context("failed to query logs")?;

    // Display the logs
    for log in logs {
        println!("{}", log.format_for_display());
    }

    tx.commit().await
        .context("failed to commit transaction")?;

    Ok(())
}

/// Refresh a PR from GitHub
/// 
/// # Errors
/// 
/// Returns an error if:
/// - Failed to get current repository information
/// - GitHub API call fails or PR not found
/// - Git fetch of head commit fails
/// - Database transaction or operations fail
/// - PR has no commits
pub async fn refresh(pr_number: usize, db: &mut Db) -> anyhow::Result<()> {
    let shell = Shell::new()?;
    let current_repo = repo::current_repo(&shell)
        .context("failed to get current repository")?;

    // Fetch PR info from GitHub
    let pr_info = gh::get_pr_info(&shell, pr_number)
        .context("failed to fetch PR from GitHub")?;

    // Fetch the head commit to ensure it's available locally
    git::fetch_commit(&shell, &pr_info.head_commit)
        .context("failed to fetch head commit")?;

    // Start database transaction
    let tx = db.transaction().await
        .context("failed to start database transaction")?;

    // Find or create the repository record
    let repo_record = if let Some(repo) = Repository::find_by_path(&tx, current_repo.repo_root.to_str().unwrap()).await
        .context("failed to query repository")? 
    {
        repo
    } else {
        // Create repository record
        let new_repo = NewRepository {
            name: current_repo.project_name.clone(),
            path: current_repo.repo_root.to_str().unwrap().to_string(),
            nixfile_path: "default.nix".to_string(), // Default, can be configured later
        };
        Repository::create(&tx, new_repo).await
            .context("failed to create repository record")?
    };

    // Create or find commits for all commits in the PR
    let mut commit_records = Vec::new();
    for commit_oid in pr_info.commit_ids() {
        let commit_record = if let Some(commit) = Commit::find_by_git_id(&tx, repo_record.id, commit_oid).await
            .context("failed to query commit")? 
        {
            commit
        } else {
            // Get the jj change ID for this commit
            let jj_change_id = jj::get_change_id_for_commit(&shell, &commit_oid.to_string())
                .with_context(|| format!("failed to get jj change ID for commit {}", commit_oid))?;

            // Create new commit record
            let new_commit = NewCommit {
                repository_id: repo_record.id,
                git_commit_id: commit_oid.clone(),
                jj_change_id,
                review_status: ReviewStatus::Unreviewed,
                should_run_ci: false,
                ci_status: CiStatus::Unstarted,
                nix_derivation: None,
                review_text: None,
            };

            Commit::create(&tx, new_commit).await
                .context("failed to create commit record")?
        };
        commit_records.push(commit_record);
    }

    // Find the tip commit (last commit in the list)
    let tip_commit = commit_records.last()
        .context("PR has no commits")?;

    // Create or update the PR record
    let pr_record = if let Some(pr) = PullRequest::find_by_number(&tx, repo_record.id, pr_number.try_into()?).await
        .context("failed to query pull request")?
    {
        // Update existing PR
        let updates = UpdatePullRequest {
            title: Some(pr_info.title.clone()),
            body: Some(pr_info.body.clone()),
            tip_commit_id: Some(tip_commit.id),
            ..Default::default()
        };
        pr.update(&tx, updates).await
            .context("failed to update pull request")?
    } else {
        // Create new PR
        let new_pr = NewPullRequest {
            repository_id: repo_record.id,
            pr_number: pr_number.try_into()?,
            title: pr_info.title.clone(),
            body: pr_info.body.clone(),
            tip_commit_id: tip_commit.id,
            review_status: ReviewStatus::Unreviewed,
            priority: 0,
            ok_to_merge: true,
            required_reviewers: 1,
        };
        PullRequest::create(&tx, new_pr).await
            .context("failed to create pull request")?
    };

    // Implement the new refresh logic
    use std::collections::{HashMap, HashSet};

    // Get existing pr_commits for this PR
    let existing_pr_commits = PrCommit::find_by_pr(&tx, pr_record.id).await
        .context("failed to get existing PR commits")?;

    // Build maps for efficient lookup
    let mut existing_by_commit_id: HashMap<i32, PrCommit> = HashMap::new();
    for pr_commit in existing_pr_commits {
        existing_by_commit_id.insert(pr_commit.commit_id, pr_commit);
    }

    // Build the new state we want
    let mut new_commit_ids = HashSet::new();
    let mut updates_needed = Vec::new();
    
    for (i, commit) in commit_records.iter().enumerate() {
        new_commit_ids.insert(commit.id);
        
        let new_sequence = (i + 1) as i32;
        let new_commit_type = if i == commit_records.len() - 1 {
            CommitType::Tip
        } else {
            CommitType::Normal
        };
        
        if let Some(existing) = existing_by_commit_id.get(&commit.id) {
            // Check if we need to update this record
            let needs_update = existing.sequence_order != new_sequence
                || existing.commit_type != new_commit_type
                || !existing.is_current;
                
            if needs_update {
                updates_needed.push((
                    existing.id,
                    Some(new_sequence),
                    Some(new_commit_type),
                    Some(true),
                ));
            }
        } else {
            // This is a new commit, insert it
            PrCommit::create(&tx, pr_record.id, commit.id, new_sequence, new_commit_type).await
                .context("failed to create new pr_commit record")?;
        }
    }

    // Mark commits that are no longer in the PR as not current
    for (commit_id, existing) in &existing_by_commit_id {
        if !new_commit_ids.contains(commit_id) && existing.is_current {
            updates_needed.push((existing.id, None, None, Some(false)));
        }
    }

    // Apply all updates
    for (id, sequence_order, commit_type, is_current) in updates_needed {
        PrCommit::update_status(&tx, id, sequence_order, commit_type, is_current).await
            .context("failed to update pr_commit record")?;
    }

    // Scan for ACKs in GitHub comments and reviews
    scan_and_update_acks(&tx, &pr_info, &pr_record, &commit_records).await
        .context("failed to scan and update ACKs")?;

    // Commit the transaction
    tx.commit().await
        .context("failed to commit transaction")?;

    println!("Successfully refreshed PR #{}", pr_number);
    println!("Title: {}", pr_info.title);
    println!("Commits: {}", commit_records.len());
    println!("Head commit: {}", pr_info.head_commit);

    Ok(())
}
