// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct Repository {
    pub project_name: String,
    pub repo_root: PathBuf,
}

pub fn current_repo() -> Result<Repository, Box<dyn std::error::Error>> {
    // Get repository root using git
    let git_output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()?;
    
    if !git_output.status.success() {
        return Err("Failed to get repository root with git rev-parse".into());
    }
    
    let repo_root = PathBuf::from(
        String::from_utf8(git_output.stdout)?
            .trim()
    );
    
    // Get project name using gh
    let gh_output = Command::new("gh")
        .args(["repo", "view", "--json", "owner,name", "--jq", ".owner.login + \".\" + .name"])
        .output()?;
    
    if !gh_output.status.success() {
        return Err("Failed to get project name with gh repo view".into());
    }
    
    let project_name = String::from_utf8(gh_output.stdout)?
        .trim()
        .to_string();
    
    Ok(Repository {
        project_name,
        repo_root,
    })
}
