// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::PathBuf;
use xshell::{Shell, cmd};

#[derive(Debug, Clone)]
pub struct Repository {
    pub project_name: String,
    pub repo_root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepoError {
    GitCommandFailed,
    GitOutputInvalid,
    GhCommandFailed,
    GhOutputInvalid,
    Utf8Error,
}

impl std::fmt::Display for RepoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GitCommandFailed => write!(f, "Failed to get repository root with git rev-parse"),
            Self::GitOutputInvalid => write!(f, "Git command output was invalid"),
            Self::GhCommandFailed => write!(f, "Failed to get project name with gh repo view"),
            Self::GhOutputInvalid => write!(f, "GitHub CLI command output was invalid"),
            Self::Utf8Error => write!(f, "Command output contained invalid UTF-8"),
        }
    }
}

impl std::error::Error for RepoError {}

pub fn current_repo() -> Result<Repository, RepoError> {
    let sh = Shell::new().map_err(|_| RepoError::GitCommandFailed)?;
    
    // Get repository root using git
    let repo_root_str = cmd!(sh, "git rev-parse --show-toplevel")
        .read()
        .map_err(|_| RepoError::GitCommandFailed)?;
    
    let repo_root = PathBuf::from(repo_root_str.trim());
    
    // Get project name using gh
    let project_name = cmd!(sh, "gh repo view --json owner,name --jq .owner.login + \".\" + .name")
        .read()
        .map_err(|_| RepoError::GhCommandFailed)?
        .trim()
        .to_string();
    
    Ok(Repository {
        project_name,
        repo_root,
    })
}
