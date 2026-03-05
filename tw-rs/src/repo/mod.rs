// SPDX-License-Identifier: GPL-3.0-or-later

use core::marker::PhantomData;
use std::path::PathBuf;
use xshell::{Shell, cmd};

#[derive(Debug, Clone)]
pub enum Upstream {
    Github,
    GiteaBitcoinNinja,
}

#[derive(Debug, Clone)]
pub struct Repository {
    pub project_name: String,
    pub repo_root: PathBuf,
    pub upstream: Upstream,
    _marker: PhantomData<()>,
}

#[derive(Debug)]
pub enum RepoError {
    GitCommandFailed(xshell::Error),
    UnknownProjectName,
    UnknownUpstream,
}

impl std::fmt::Display for RepoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GitCommandFailed(e) => write!(f, "Failed to get repository root: {e}"),
            Self::UnknownProjectName => write!(f, "Failed to get project name from upstream/origin URLs"),
            Self::UnknownUpstream => write!(f, "Failed to get upstream type (Github, Gitea) from upstream/origin URLs"),
        }
    }
}

impl std::error::Error for RepoError {}

fn parse_github_url(url: &str) -> Option<String> {
    for prefix in [
        "git@github.com:",
        "https://github.com/",
        "https://www.github.com/",
        "https://gitea.bitcoin.ninja/",
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

pub fn current_repo(sh: &Shell) -> Result<Repository, RepoError> {
    // Get repository root using git
    let repo_root_str = cmd!(sh, "git rev-parse --show-toplevel")
        .read()
        .map_err(RepoError::GitCommandFailed)?;

    let repo_root = PathBuf::from(repo_root_str.trim());

    let mut project_name = None;
    let mut upstream = None;
    // Try to get project name from git remotes first
    for remote in ["origin", "upstream"] {
        if let Ok(origin_url) = cmd!(sh, "git remote get-url {remote}").read()
            && let Some(project) = parse_github_url(origin_url.trim())
        {
            project_name = Some(project);
            if origin_url.contains("github.com") {
                upstream = Some(Upstream::Github);
            }
            if origin_url.contains("gitea.bitcoin.ninja") {
                upstream = Some(Upstream::GiteaBitcoinNinja);
            }
            break;
        }
    }

    let project_name = project_name
        .ok_or(RepoError::UnknownProjectName)?;
    let upstream = upstream
        .ok_or(RepoError::UnknownUpstream)?;

    Ok(Repository {
        project_name,
        repo_root,
        upstream,
        _marker: PhantomData,
    })
}
