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

fn parse_github_url(url: &str) -> Option<String> {
    let url = url.to_ascii_lowercase();

    for prefix in [
        "git@github.com:",
        "https://github.com/",
        "https://www.github.com/",
    ] {
        if let Some(https_part) = url.strip_prefix(prefix) {
            let mut repo_part = https_part;
            for _ in 0..2 {
                repo_part = repo_part.strip_suffix(".git").unwrap_or(repo_part);
                repo_part = repo_part.strip_suffix("/").unwrap_or(repo_part);
            }
            return Some(repo_part.replace('/', "."));
        }
    }
    
    None
}

pub fn current_repo() -> Result<Repository, RepoError> {
    let sh = Shell::new().map_err(RepoError::ConstructingShell)?;
    
    // Get repository root using git
    let repo_root_str = cmd!(sh, "git rev-parse --show-toplevel")
        .read()
        .map_err(RepoError::GitCommandFailed)?;
    
    let repo_root = PathBuf::from(repo_root_str.trim());
    
    let mut project_name = None;
    // Try to get project name from git remotes first
    for remote in ["origin", "upstream"] {
        if let Ok(origin_url) = cmd!(sh, "git remote get-url {remote}").read() {
            if let Some(project) = parse_github_url(origin_url.trim()) {
                project_name = Some(project);
                break;
            }
        }
    }
    // Failing that, invoke gh (though will gh succeed without remotes either?)
    let project_name = match project_name {
        Some(x) => x,
        None => cmd!(sh, "gh repo view --json owner,name --jq '.owner.login + \".\" + .name'")
            .read()
            .map_err(RepoError::GhCommandFailed)?
            .trim()
            .to_string(),
    };
    
    Ok(Repository {
        project_name,
        repo_root,
    })
}
