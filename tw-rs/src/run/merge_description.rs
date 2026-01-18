// SPDX-License-Identifier: GPL-3.0-or-later

use crate::git::GitCommit;
use crate::tw::TaskCollection;

use std::collections::HashMap;

use anyhow::{self, Context as _};
use bitcoin_hashes::{HashEngine as _, sha512};
use xshell::{Shell, cmd};
use serde_json::Value;

pub fn compute_merge_description(
    sh: &Shell,
    tasks: &TaskCollection,
    pr_task: &crate::tw::PrTask,
    merge_change_id: &str,
) -> anyhow::Result<String> {
    let pr_number = pr_task.number();
    let project = pr_task.project();
    
    // Get merge commit ID from JJ change
    let merge_commit_id = crate::jj::jj_log(sh, "commit_id", merge_change_id)
        .context("Failed to get merge commit ID")?;
    let merge_commit_id = merge_commit_id.trim();
    
    // Parse merge commit ID as GitCommit
    let merge_commit: GitCommit = merge_commit_id.parse()
        .context("Failed to parse merge commit ID")?;

    // Get PR info from task and GitHub
    let title = pr_task.title();
    
    // Get PR body from GitHub using gh tool
    let num_s = pr_number.to_string();
    let body_json = cmd!(sh, "gh pr view {num_s} --json body")
        .read()
        .context("Failed to get PR body")?;
    
    let body_data: Value = serde_json::from_str(&body_json)
        .context("Failed to parse PR body JSON")?;
    
    let body = body_data["body"].as_str().unwrap_or("").to_string();
    
    // Get base and head commits
    let base_commit = pr_task.base_commit();
    let head_commit = pr_task.tip_commit(tasks).commit_id();

    // Build description
    let mut message = if !title.is_empty() {
        format!("Merge {}#{}: {}\n\n", project, pr_number, title)
    } else {
        format!("Merge {}#{}\n\n", project, pr_number)
    };

    // Add commit list
    let commit_list = cmd!(sh, "git --no-pager log --no-merges --topo-order --pretty='format:%H %s (%an)' {base_commit}..{head_commit}")
        .read()
        .context("Failed to get commit list")?;
    message.push_str(&commit_list);

    // Add PR body
    if !body.is_empty() {
        message.push_str("\n\nPull request description:\n\n  ");
        message.push_str(&body.replace('\n', "\n  "));
        message.push('\n');
    }

    // Get comments and reviews from GitHub using gh tool
    let acks = get_acks_from_github(sh, pr_number, head_commit)
        .context("Failed to get ACKs from GitHub")?;

    // Add ACKs section
    if !acks.is_empty() {
        message.push_str("\n\nACKs for top commit:\n");
        for (name, ack_msg) in acks {
            message.push_str(&format!("  {}:\n    {}\n", name, ack_msg));
        }
    } else {
        message.push_str("\n\nTop commit has no ACKs.\n");
    }

    // Add tree SHA512
    let tree_hash = tree_sha512sum(sh, &merge_commit)
        .context("Failed to compute tree SHA512")?;
    message.push_str(&format!("\n\nTree-SHA512: {}", tree_hash));

    Ok(message)
}

fn get_acks_from_github(
    sh: &Shell,
    pr_number: usize,
    head_commit: &GitCommit,
) -> anyhow::Result<Vec<(String, String)>> {
    let head_abbrev = &head_commit.to_string()[..6];
    let mut acks = Vec::new();
    let pr_number = pr_number.to_string();

    // Get PR comments using gh tool
    let comments_json = cmd!(sh, "gh pr view {pr_number} --json comments")
        .read()
        .context("Failed to get PR comments")?;
    
    let comments_data: Value = serde_json::from_str(&comments_json)
        .context("Failed to parse comments JSON")?;

    if let Some(comments_array) = comments_data["comments"].as_array() {
        for comment in comments_array {
            if let (Some(body), Some(author)) = (
                comment["body"].as_str(),
                comment["author"]["login"].as_str(),
            ) {
                // Look for ACK lines that contain the abbreviated commit ID
                for line in body.lines() {
                    if line.contains("ACK") 
                        && line.contains(head_abbrev)
                        && !line.starts_with("> ")     // omit quoted comments
                        && !line.starts_with("    ")   // omit markdown indentation
                    {
                        acks.push((author.to_string(), line.to_string()));
                        break; // Only take first ACK per user
                    }
                }
            }
        }
    }

    // Get PR reviews using gh tool
    let reviews_json = cmd!(sh, "gh pr view {pr_number} --json reviews")
        .read()
        .context("Failed to get PR reviews")?;
    
    let reviews_data: Value = serde_json::from_str(&reviews_json)
        .context("Failed to parse reviews JSON")?;

    if let Some(reviews_array) = reviews_data["reviews"].as_array() {
        for review in reviews_array {
            if let (Some(body), Some(author)) = (
                review["body"].as_str(),
                review["author"]["login"].as_str(),
            ) {
                // Look for ACK lines that contain the abbreviated commit ID
                for line in body.lines() {
                    if line.contains("ACK") 
                        && line.contains(head_abbrev)
                        && !line.starts_with("> ")     // omit quoted comments
                        && !line.starts_with("    ")   // omit markdown indentation
                    {
                        // Check if we already have an ACK from this user
                        if !acks.iter().any(|(name, _)| name == author) {
                            acks.push((author.to_string(), line.to_string()));
                        }
                        break;
                    }
                }
            }
        }
    }

    Ok(acks)
}

fn tree_sha512sum(sh: &Shell, commit_id: &GitCommit) -> anyhow::Result<String> {
    // Get all files in the tree recursively
    let ls_tree_output = cmd!(sh, "git ls-tree --full-tree -r {commit_id}")
        .read()
        .context("Failed to list tree contents")?;

    let mut files = Vec::new();
    let mut blob_by_name = HashMap::new();

    // Parse git ls-tree output
    for line in ls_tree_output.lines() {
        if let Some(tab_pos) = line.find('\t') {
            let metadata_part = &line[..tab_pos];
            let name = &line[tab_pos + 1..];
            
            let parts: Vec<&str> = metadata_part.split_whitespace().collect();
            if parts.len() >= 3 && parts[1] == "blob" {
                files.push(name.to_string());
                blob_by_name.insert(name.to_string(), parts[2].to_string());
            }
        }
    }

    files.sort();

    // Create overall hash engine
    let mut overall_engine = sha512::Hash::engine();

    // Process each file
    for file in files {
        if let Some(blob_id) = blob_by_name.get(&file) {
            // Get blob content
            let blob_content = cmd!(sh, "git cat-file blob {blob_id}")
                .output()
                .with_context(|| format!("Failed to read blob {}", blob_id))?;

            // Hash the blob content
            let file_hash = sha512::Hash::hash(&blob_content.stdout);
            
            // Update overall hash: hash + "  " + filename + "\n"
            overall_engine.input(file_hash.to_string().as_bytes());
            overall_engine.input(b"  ");
            overall_engine.input(file.as_bytes());
            overall_engine.input(b"\n");
        }
    }

    let final_hash = sha512::Hash::from_engine(overall_engine);
    Ok(final_hash.to_string())
}

