// SPDX-License-Identifier: GPL-3.0-or-later

use crate::db::{Db, DbTransactionError};
use crate::db::models::{self, NewRepository, Repository};
use xshell::{Shell, cmd};

#[derive(Debug, Clone)]
pub enum Upstream {
    Github,
    GiteaBitcoinNinja,
}

#[derive(Debug)]
pub enum RepoError {
    CreateShell(xshell::Error),
    DatabaseTransaction(DbTransactionError),
    Database(models::RepositoryError),
    GitCommandFailed(xshell::Error),
    UnknownProjectName,
    UnknownUpstream,
}

impl std::fmt::Display for RepoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CreateShell(..) => write!(f, "failed to create shell"),
            Self::GitCommandFailed(..) => write!(f, "failed to get repository root"),
            Self::DatabaseTransaction(..) => write!(f, "database transaction error"),
            Self::Database(..) => write!(f, "database error"),
            Self::UnknownProjectName => {
                write!(f, "Failed to get project name from upstream/origin URLs")
            }
            Self::UnknownUpstream => write!(
                f,
                "Failed to get upstream type (Github, Gitea) from upstream/origin URLs"
            ),
        }
    }
}

impl std::error::Error for RepoError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            Self::CreateShell(ref e) => Some(e),
            Self::GitCommandFailed(ref e) => Some(e),
            Self::DatabaseTransaction(ref e) => Some(e),
            Self::Database(ref e) => Some(e),
            Self::UnknownProjectName => None,
            Self::UnknownUpstream => None,
        }
        
    }
}

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

/// # Errors
///
/// Returns an error if the git command to get the repository root fails, if no project name
/// can be determined from the origin/upstream remote URLs, or if the upstream type cannot
/// be determined from the remote URLs.
pub async fn current_repo(db: &mut Db) -> Result<Repository, RepoError> {
    let sh = Shell::new()
        .map_err(RepoError::CreateShell)?;

    // Get repository root using git
    let repo_root = cmd!(sh, "git rev-parse --show-toplevel")
        .read()
        .map_err(RepoError::GitCommandFailed)?;
    let repo_root = repo_root.trim();

    // Find or create the repository record
    let tx = db
        .transaction()
        .await
        .map_err(RepoError::DatabaseTransaction)?;

    let existing_model = Repository::find_by_path(&tx, repo_root)
        .await
        .map_err(RepoError::Database)?;
    if let Some(model) = existing_model {
        Ok(model)
    } else {
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

        let project_name = project_name.ok_or(RepoError::UnknownProjectName)?;
        // TODO we will want to store the upstream in the database so we can switch on github/gitea.
        let _upstream = upstream.ok_or(RepoError::UnknownUpstream)?;

        // Create repository record
        let new_repo = NewRepository {
            nixfile_path: format!("/home/apoelstra/code/local-nix-ci/main/{project_name}.check-pr.nix"), // Default, can be configured later
            name: project_name,
            path: repo_root.to_owned(),
        };
        let ret = Repository::create(&tx, new_repo)
            .await
            .map_err(RepoError::Database)?;
        tx.commit().await.map_err(RepoError::DatabaseTransaction)?;
        Ok(ret)
    }
}
