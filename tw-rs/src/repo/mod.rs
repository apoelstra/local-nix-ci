// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::PathBuf;
use xshell::{Shell, cmd};

#[derive(Debug, Clone)]
pub struct Repository {
    pub project_name: String,
    pub repo_root: PathBuf,
}

#[derive(Debug)]
pub enum RepoError {
    ConstructingShell(xshell::Error),
    GitCommandFailed(xshell::Error),
    GhCommandFailed(xshell::Error),
}

impl std::fmt::Display for RepoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConstructingShell(e) => write!(f, "Failed to construct shell: {e}"),
            Self::GitCommandFailed(e) => write!(f, "Failed to get repository root: {e}"),
            Self::GhCommandFailed(e) => write!(f, "Failed to get project name: {e}"),
        }
    }
}

impl std::error::Error for RepoError {}

pub fn current_repo() -> Result<Repository, RepoError> {
    let sh = Shell::new().map_err(RepoError::ConstructingShell)?;
    
    // Get repository root using git
    let repo_root_str = cmd!(sh, "git rev-parse --show-toplevel")
        .read()
        .map_err(RepoError::GitCommandFailed)?;
    
    let repo_root = PathBuf::from(repo_root_str.trim());
    
    // Get project name using gh
    let project_name = cmd!(sh, "gh repo view --json owner,name --jq '.owner.login + \".\" + .name'")
        .read()
        .map_err(RepoError::GhCommandFailed)?
        .trim()
        .to_string();
    
    Ok(Repository {
        project_name,
        repo_root,
    })
}
