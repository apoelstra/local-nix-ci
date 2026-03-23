// SPDX-License-Identifier: GPL-3.0-or-later

use crate::db::{
    DbQueryError,
    models::{CommitToTest, PullRequest, Repository},
};
use crate::git::CommitId;

use core::fmt;
use std::collections::HashMap;

use bitcoin_hashes::{HashEngine as _, sha512};
use xshell::{Shell, cmd};

#[derive(Debug)]
pub enum MergeDescriptionError {
    GetRepository(crate::db::models::RepositoryError),
    GetCommitList(crate::jj::Error),
    ListTreeContents(CommitId, xshell::Error),
    ReadBlob(String, xshell::Error),
    DatabaseQuery(&'static str, DbQueryError),
}

impl fmt::Display for MergeDescriptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::GetRepository(_) => write!(f, "failed to get git repository for PR"),
            Self::GetCommitList(_) => write!(f, "failed to get commit list"),
            Self::ListTreeContents(ref commit, _) => {
                write!(f, "failed to list tree contents for commit {commit}")
            }
            Self::ReadBlob(ref blob_id, _) => write!(f, "failed to read blob {blob_id}"),
            Self::DatabaseQuery(action, _) => write!(f, "database query failed: {}", action),
        }
    }
}

impl std::error::Error for MergeDescriptionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            Self::GetRepository(ref e) => Some(e),
            Self::GetCommitList(ref e) => Some(e),
            Self::ListTreeContents(_, ref e) => Some(e),
            Self::ReadBlob(_, ref e) => Some(e),
            Self::DatabaseQuery(_, ref e) => Some(e),
        }
    }
}

/// Computes a complete merge description that can be used as the git message for the merge
/// commit, using the database abstraction layer.
///
/// # Errors
///
/// Returns an error if:
/// - The repository cannot be found in the database
/// - Database queries for commits or ACKs fail
/// - JJ operations to get commit information fail
/// - Git operations to calculate the tree hash fail
///
/// # Panics
///
/// Panics if `std::current_env` returns an error.
pub async fn compute_merge_description(
    tx: &tokio_postgres::Transaction<'_>,
    pr: &PullRequest,
    merge_commit: &CommitToTest,
) -> Result<String, MergeDescriptionError> {
    // Get repository information
    let repository = Repository::get_by_id(tx, pr.repository_id)
        .await
        .map_err(MergeDescriptionError::GetRepository)?;

    // Create shell and set working directory to repository path
    let sh = Shell::new().unwrap();
    sh.change_dir(&repository.path);

    // Derive project name from repository name
    let project = repository.name.replace('.', "/");

    // Build description
    let mut message = if pr.title.is_empty() {
        format!("Merge {}#{}\n\n", project, pr.pr_number)
    } else {
        format!("Merge {}#{}: {}\n\n", project, pr.pr_number, pr.title)
    };

    // Get commit list from database and format using jj
    let commits = pr
        .id
        .get_current_non_merge_commits(tx)
        .await
        .map_err(|e| MergeDescriptionError::DatabaseQuery("get merge commits", e))?;

    let mut commit_lines = Vec::new();
    for commit in commits.iter().rev() {
        // Use jj log to get formatted commit info
        let commit_info = crate::jj::jj_log(
            &repository.repo_shell,
            Some("commit_id ++ \" \" ++ description.first_line() ++ \" (\" ++ author.name() ++ \")\""),
            &commit.jj_change_id,
        )
        .await
        .map_err(MergeDescriptionError::GetCommitList)?;
        commit_lines.push(commit_info.trim().to_string());
    }
    message.push_str(&commit_lines.join("\n"));

    // Add PR body
    message.push_str("\n\nPull request description:\n\n  ");
    message.push_str(&pr.body.trim().replace('\r', "").replace('\n', "\n  "));
    message.push('\n');

    // Get ACKs from database
    let acks = pr
        .id
        .get_posted_acks_for_tip(tx)
        .await
        .map_err(|e| MergeDescriptionError::DatabaseQuery("get posted acks", e))?;

    // Add ACKs section
    if acks.is_empty() {
        message.push_str("\n\nTop commit has no ACKs.\n");
    } else {
        message.push_str("\n\nACKs for top commit:\n");
        for (name, ack_msg) in &acks {
            message.push_str(&format!("  {}:\n    {}\n", name, ack_msg));
        }
    }

    // Add tree SHA512
    let git_commit_id = merge_commit.git_commit_id.clone();
    let tree_hash = tokio::task::spawn_blocking(move || tree_sha512sum(&sh, &git_commit_id))
        .await
        .expect("no panics in tree_sha512")?;

    message.push_str(&format!("\n\nTree-SHA512: {}", tree_hash));

    Ok(message)
}

fn tree_sha512sum(sh: &Shell, commit_id: &CommitId) -> Result<String, MergeDescriptionError> {
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
