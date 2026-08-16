// SPDX-License-Identifier: GPL-3.0-or-later

mod shell;

use crate::db::Db;
use crate::db::models::{self, NewRepository, Repository};

pub use shell::{RepoShell, RepoShellError, RepoShellLock};

/// Which API to use when talking to the upstream.
#[derive(Debug, Clone)]
pub enum Upstream {
    Github,
    Forgejo { https_url: String },
}

impl Upstream {
    fn gitea_bitcoin_ninja() -> Self {
        Self::Forgejo {
            https_url: "https://gitea.bitcoin.ninja".to_string(),
        }
    }
}

#[derive(Debug)]
pub enum RepoError {
    CreateShell(RepoShellError),
    DatabaseTransaction(tokio_postgres::Error),
    Database(models::RepositoryError),
    GitCommandFailed(xshell::Error),
    UnknownProjectName,
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
        }
        
    }
}

/// # Errors
///
/// Returns an error if the git command to get the repository root fails, if no project name
/// can be determined from the origin/upstream remote URLs, or if the upstream type cannot
/// be determined from the remote URLs.
pub async fn current_repo(db: &mut Db) -> Result<Repository, RepoError> {
    let sh = RepoShell::new_at_cwd()
        .map_err(RepoError::CreateShell)?;
    // non UTF-8 characters in paths are annoying because ToSql wants to type them as BYTEA and I
    // need to investigate whether that will work with a VARCHAR column or what. Probably we should
    // just return an error here.
    //
    // We need the object in the database to actually match a disk path because we use that to
    // access repos from the daemon etc.
    let repo_root = sh.repo_root();
    let repo_root = repo_root
        .to_str()
        .expect("FIXME we have not handled non-UTF8 characters in repo paths");

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
        let project_name = sh.project_name().replace('/', ".");

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
