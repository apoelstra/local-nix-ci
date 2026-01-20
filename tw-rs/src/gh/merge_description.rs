// SPDX-License-Identifier: GPL-3.0-or-later

use crate::git::GitCommit;

use core::fmt;
use std::collections::HashMap;

use bitcoin_hashes::{HashEngine as _, sha512};
use xshell::{Shell, cmd};
use serde_json::Value;

#[derive(Debug)]
pub enum MergeDescriptionError {
    GetCommitId(String, crate::jj::Error),
    ParseCommitId(String, crate::git::Error),
    GetPrBody(usize, xshell::Error),
    ParsePrBodyJson(usize, serde_json::Error),
    GetCommitList(xshell::Error),
    GetPrComments(usize, xshell::Error),
    ParseCommentsJson(usize, serde_json::Error),
    GetPrReviews(usize, xshell::Error),
    ParseReviewsJson(usize, serde_json::Error),
    ListTreeContents(GitCommit, xshell::Error),
    ReadBlob(String, xshell::Error),
}

impl fmt::Display for MergeDescriptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::GetCommitId(ref change_id, _) => write!(f, "failed to get commit ID for change ID {change_id}"),
            Self::ParseCommitId(ref commit_id, _) => write!(f, "failed to parse commit ID {commit_id}"),
            Self::GetPrBody(pr_num, _) => write!(f, "failed get PR body for {pr_num} from 'gh pr view'"),
            Self::ParsePrBodyJson(pr_num, _) => write!(f, "failed to parse PR body JSON for {pr_num}"),
            Self::GetCommitList(_) => write!(f, "failed to get commit list"),
            Self::GetPrComments(pr_num, _) => write!(f, "failed to get PR comments for {pr_num}"),
            Self::ParseCommentsJson(pr_num, _) => write!(f, "failed to parse comments JSON for {pr_num}"),
            Self::GetPrReviews(pr_num, _) => write!(f, "failed to get PR reviews for {pr_num}"),
            Self::ParseReviewsJson(pr_num, _) => write!(f, "failed to parse reviews JSON for {pr_num}"),
            Self::ListTreeContents(ref commit, _) => write!(f, "failed to list tree contents for commit {commit}"),
            Self::ReadBlob(ref blob_id, _) => write!(f, "failed to read blob {blob_id}"),
        }
    }
}

impl std::error::Error for MergeDescriptionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            Self::GetCommitId(_, ref e) => Some(e),
            Self::ParseCommitId(_, ref e) => Some(e),
            Self::GetPrBody(_, ref e) => Some(e),
            Self::ParsePrBodyJson(_, ref e) => Some(e),
            Self::GetCommitList(ref e) => Some(e),
            Self::GetPrComments(_, ref e) => Some(e),
            Self::ParseCommentsJson(_, ref e) => Some(e),
            Self::GetPrReviews(_, ref e) => Some(e),
            Self::ParseReviewsJson(_, ref e) => Some(e),
            Self::ListTreeContents(_, ref e) => Some(e),
            Self::ReadBlob(_, ref e) => Some(e),
        }
    }
}

    
pub fn compute_merge_description(
    sh: &Shell,
    pr_task: &crate::tw::PrTask,
    head_commit: &GitCommit,
    merge_change_id: &str,
) -> Result<String, MergeDescriptionError> {
    let pr_number = pr_task.number();
    let project = pr_task.project().replace('.', "/");
    
    // Get merge commit ID from JJ change
    let merge_commit_id = crate::jj::jj_log(sh, "commit_id", merge_change_id)
        .map_err(|e| MergeDescriptionError::GetCommitId(merge_change_id.to_owned(), e))?;
    let merge_commit_id = merge_commit_id.trim();
    
    // Parse merge commit ID as GitCommit
    let merge_commit: GitCommit = merge_commit_id.parse()
        .map_err(|e| MergeDescriptionError::ParseCommitId(merge_commit_id.to_owned(), e))?;

    // Get PR info from task and GitHub
    let title = pr_task.title();
    
    // Get PR body from GitHub using gh tool
    let num_s = pr_number.to_string();
    let body_json = cmd!(sh, "gh pr view {num_s} --json body")
        .read()
        .map_err(|e| MergeDescriptionError::GetPrBody(pr_number, e))?;
    
    let body_data: Value = serde_json::from_str(&body_json)
        .map_err(|e| MergeDescriptionError::ParsePrBodyJson(pr_number, e))?;
    
    let body = body_data["body"].as_str().unwrap_or("").to_string();
    
    // Get base and head commits
    let base_commit = pr_task.base_commit();

    // Build description
    let mut message = if !title.is_empty() {
        format!("Merge {}#{}: {}\n\n", project, pr_number, title)
    } else {
        format!("Merge {}#{}\n\n", project, pr_number)
    };

    // Add commit list
    let commit_list = cmd!(sh, "git --no-pager log --no-merges --topo-order --pretty='format:%H %s (%an)' {base_commit}..{head_commit}")
        .read()
        .map_err(MergeDescriptionError::GetCommitList)?;
    message.push_str(&commit_list);

    // Add PR body
    if !body.is_empty() {
        message.push_str("\n\nPull request description:\n\n  ");
        message.push_str(&body.trim().replace('\r', "").replace('\n', "\n  "));
        message.push('\n');
    }

    // Get comments and reviews from GitHub using gh tool
    let acks = get_acks_from_github(sh, pr_number, head_commit)?;

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
    let tree_hash = tree_sha512sum(sh, &merge_commit)?;
    message.push_str(&format!("\n\nTree-SHA512: {}", tree_hash));

    Ok(message)
}

fn get_acks_from_github(
    sh: &Shell,
    pr_number: usize,
    head_commit: &GitCommit,
) -> Result<Vec<(String, String)>, MergeDescriptionError> {
    let head_abbrev = &head_commit.to_string()[..6];
    let mut acks = Vec::new();
    let pr_number_s = pr_number.to_string();

    // Get PR comments using gh tool
    let comments_json = cmd!(sh, "gh pr view {pr_number_s} --json comments")
        .read()
        .map_err(|e| MergeDescriptionError::GetPrComments(pr_number, e))?;
    
    let comments_data: Value = serde_json::from_str(&comments_json)
        .map_err(|e| MergeDescriptionError::ParseCommentsJson(pr_number, e))?;

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
    let reviews_json = cmd!(sh, "gh pr view {pr_number_s} --json reviews")
        .read()
        .map_err(|e| MergeDescriptionError::GetPrReviews(pr_number, e))?;
    
    let reviews_data: Value = serde_json::from_str(&reviews_json)
        .map_err(|e| MergeDescriptionError::ParseReviewsJson(pr_number, e))?;

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

fn tree_sha512sum(sh: &Shell, commit_id: &GitCommit) -> Result<String, MergeDescriptionError> {
    // Get all files in the tree recursively
    let ls_tree_output = cmd!(sh, "git ls-tree --full-tree -r {commit_id}")
        .read()
        .map_err(|e| MergeDescriptionError::ListTreeContents(commit_id.clone(), e))?;

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
                .map_err(|e| MergeDescriptionError::ReadBlob(blob_id.clone(), e))?;

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

