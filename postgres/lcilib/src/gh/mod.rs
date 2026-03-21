// SPDX-License-Identifier: GPL-3.0-or-later

mod serde_types;

pub use serde_types::PrInfo;
use std::fmt;
use xshell::{Shell, cmd};

#[derive(Debug)]
pub enum Error {
    Shell(String, xshell::Error),
    Json(String, serde_json::Error),
    PrNotFound(usize),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Shell(cmd, _) => write!(f, "failed to invoke command: {}", cmd),
            Self::Json(json, _) => write!(f, "failed to parse JSON response: {}", json),
            Self::PrNotFound(pr_number) => write!(f, "PR #{} not found", pr_number),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Shell(_, e) => Some(e),
            Self::Json(_, e) => Some(e),
            Self::PrNotFound(..) => None,
        }
    }
}

/// Fetches PR information from GitHub using the `gh` CLI tool.
///
/// # Errors
///
/// Returns an error if the PR is not found, if the `gh pr view` invocation fails, or if
/// Github returns JSON we cannot parse.
pub fn get_pr_info(shell: &Shell, pr_number: usize) -> Result<PrInfo, Error> {
    let pr_num_s = pr_number.to_string();
    let cmd_str = format!("gh pr view {pr_number} --json title,body,author,commits,comments,reviews,headRefOid,baseRefName,state,mergeable,mergeStateStatus,closed,mergedAt");
    let output = cmd!(
        shell,
        "gh pr view {pr_num_s} --json title,body,author,commits,comments,reviews,headRefOid,baseRefName,state,mergeable,mergeStateStatus,closed,mergedAt"
    )
    .read()
    .map_err(|e| Error::Shell(cmd_str.clone(), e))?;

    // Check if the output indicates the PR was not found
    if output.contains("could not resolve to a PullRequest") || output.contains("not found") {
        return Err(Error::PrNotFound(pr_number));
    }

    serde_json::from_str(&output).map_err(|e| Error::Json(output, e))
}

/// Posts a comment on a GitHub PR using the `gh` CLI tool.
///
/// # Errors
///
/// Returns an error if the `gh pr comment` invocation fails.
pub fn post_pr_comment(shell: &Shell, pr_number: usize, comment: &str) -> Result<(), Error> {
    let pr_num_s = pr_number.to_string();
    let cmd_str = format!("gh pr comment {pr_number} --body '{comment}'");
    cmd!(shell, "gh pr comment {pr_num_s} --body {comment}")
        .run()
        .map_err(|e| Error::Shell(cmd_str, e))?;
    
    Ok(())
}
