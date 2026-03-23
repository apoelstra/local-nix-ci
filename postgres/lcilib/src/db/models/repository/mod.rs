// SPDX-License-Identifier: GPL-3.0-or-later

mod shell;

use chrono::{DateTime, Utc};
use core::{fmt, ops};
use postgres_types::{FromSql, ToSql};
use std::path::Path;

use super::{PullRequest, Stack};
use crate::db::{DbQueryError, EntityType, util::log_action};
pub use shell::{RepoShell, RepoShellLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromSql, ToSql)]
#[postgres(transparent)]
pub struct DbRepositoryId(i32);

impl DbRepositoryId {
    /// An i32 representation of the repository ID.
    pub fn bare_i32(self) -> i32 {
        self.0
    }
}

impl fmt::Display for DbRepositoryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[repository {}]", self.0)
    }
}

/// Repository model
#[derive(Debug)]
pub struct Repository {
    pub id: DbRepositoryId,
    pub name: String,
    pub path: String,
    pub nixfile_path: String,
    pub created_at: DateTime<Utc>,
    pub last_synced_at: DateTime<Utc>,
    pub repo_shell: RepoShell,
}

impl ops::Deref for Repository {
    type Target = DbRepositoryId;
    fn deref(&self) -> &Self::Target {
        &self.id
    }
}

#[derive(Debug)]
pub enum RepositoryError {
    CreateShell(xshell::Error),
    Query(DbQueryError),
    RepoPathNotExist(String),
    NixfilePathNotExist(String),
}

impl fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::CreateShell(..) => f.write_str("failed to create repository shell"),
            Self::Query(..) => f.write_str("database query error"),
            Self::RepoPathNotExist(ref path) => write!(f, "repository path {} does not exist", path),
            Self::NixfilePathNotExist(ref path) => write!(f, "repository nixfile path {} does not exist", path),
        }
        
    }
}

impl std::error::Error for RepositoryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            Self::CreateShell(ref e) => Some(e),
            Self::Query(ref e) => Some(e),
            Self::RepoPathNotExist(..) => None,
            Self::NixfilePathNotExist(..) => None,
        }
        
    }
}

#[derive(Debug, Clone)]
pub struct NewRepository {
    pub name: String,
    pub path: String,
    pub nixfile_path: String,
}

impl DbRepositoryId {
    /// Returns the list of current (not pushed) pull requests associated with
    /// this repository.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn get_current_pull_requests(
        self,
        tx: &tokio_postgres::Transaction<'_>,
    ) -> Result<Vec<PullRequest>, DbQueryError> {
        let rows = tx
            .query(
                r#"
                SELECT id, repository_id, pr_number, title, body, author_login, target_branch, tip_commit_id, merge_status, review_status,
                       priority, ok_to_merge, required_reviewers, created_at, updated_at, synced_at
                FROM pull_requests
                WHERE repository_id = $1 AND merge_status != 'pushed' AND merge_status != 'conflicted'
                ORDER BY pr_number DESC
                "#,
                &[&self],
            )
            .await
            .map_err(|error| {
                DbQueryError {
                    action: "get_current_pull_requests",
                    entity_type: EntityType::Repository,
                    raw_id: Some(self.bare_i32()),
                    clauses: vec![],
                    error,
                }
            })?;
        Ok(rows.iter().map(PullRequest::from_row).collect())
    }

    /// Returns the list of merge stacks associated with
    /// this repository.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn get_stacks(
        self,
        tx: &tokio_postgres::Transaction<'_>,
    ) -> Result<Vec<Stack>, DbQueryError> {
        let rows = tx
            .query(
                r#"
                SELECT id, repository_id, target_branch, created_at, updated_at
                FROM stacks
                WHERE repository_id = $1
                ORDER BY created_at ASC
                "#,
                &[&self],
            )
            .await
            .map_err(|error| DbQueryError {
                action: "get_stacks",
                entity_type: EntityType::Repository,
                raw_id: Some(self.bare_i32()),
                clauses: vec![],
                error,
            })?;
        Ok(rows.iter().map(Stack::from_row).collect())
    }

    /// Update the last synced timestamp for this repository
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn update_last_synced(
        &self,
        tx: &tokio_postgres::Transaction<'_>,
    ) -> Result<(), DbQueryError> {
        tx
            .execute(
                r#"
                UPDATE repositories SET last_synced_at = NOW()
                WHERE id = $1
                RETURNING id, name, path, nixfile_path, created_at, last_synced_at
                "#,
                &[&self],
            )
            .await
            .map_err(|error| {
                DbQueryError {
                    action: "update_last_synced",
                    entity_type: EntityType::Repository,
                    raw_id: Some(self.bare_i32()),
                    clauses: vec![],
                    error,
                }
            })?;

        Ok(())
    }
}

impl Repository {
    fn from_row(row: &tokio_postgres::Row) -> Result<Self, RepositoryError> {
        let repo_path = row.get("path");
        let nixfile_path = row.get("path");
        if !Path::new(&repo_path).exists() {
            return Err(RepositoryError::RepoPathNotExist(repo_path));
        }
        if !Path::new(&nixfile_path).exists() {
            return Err(RepositoryError::NixfilePathNotExist(nixfile_path));
        }
        let repo_shell = RepoShell::new(&repo_path)
            .map_err(RepositoryError::CreateShell)?;

        Ok(Self {
            id: row.get("id"),
            name: row.get("name"),
            path: repo_path,
            nixfile_path,
            created_at: row.get("created_at"),
            last_synced_at: row.get("last_synced_at"),
            repo_shell,
        })
    }

    /// Create a new repository
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn create(
        tx: &tokio_postgres::Transaction<'_>,
        new_repo: NewRepository,
    ) -> Result<Self, RepositoryError> {
        // Do path checks explicitly before any database operations.
        if !Path::new(&new_repo.path).exists() {
            return Err(RepositoryError::RepoPathNotExist(new_repo.path));
        }
        if !Path::new(&new_repo.nixfile_path).exists() {
            return Err(RepositoryError::NixfilePathNotExist(new_repo.nixfile_path));
        }

        let row = tx
            .query_one(
                r#"
                INSERT INTO repositories (name, path, nixfile_path)
                VALUES ($1, $2, $3)
                RETURNING id, name, path, nixfile_path, created_at, last_synced_at
                "#,
                &[&new_repo.name, &new_repo.path, &new_repo.nixfile_path],
            )
            .await
            .map_err(|error| {
                DbQueryError {
                    action: "insert_into_repositories",
                    entity_type: EntityType::Repository,
                    raw_id: None,
                    clauses: vec![],
                    error,
                }
            })
            .map_err(RepositoryError::Query)?;

        log_action(
            tx,
            EntityType::System,
            0,
            "repository_created",
            Some(&format!("Created repository: {}", new_repo.name)),
            None,
        )
        .await
        .map_err(RepositoryError::Query)?;

        Self::from_row(&row)
    }

    /// List all repositories
    ///
    /// # Errors
    ///
    /// If any repository fails the path checks, it is omitted from the list and an error
    /// is put into the returned error vector. If the database query fails, returns an
    /// empty result vector and an error vector with a single database query error.
    pub async fn list_all(
        tx: &tokio_postgres::Transaction<'_>
    ) -> (Vec<Self>, Vec<RepositoryError>) {
        let rows = match tx
            .query("SELECT id, name, path, nixfile_path, created_at, last_synced_at FROM repositories ORDER BY name", &[])
            .await
            .map_err(|error| {
                DbQueryError {
                    action: "list_all_repositories",
                    entity_type: EntityType::Repository,
                    raw_id: None,
                    clauses: vec![],
                    error,
                }
            }) {
            Ok(rows) => rows,
            Err(e) => return (vec![], vec![RepositoryError::Query(e)]),
        };

        let mut ret_res = vec![];
        let mut ret_err = vec![];
        for row in &rows {
            match Self::from_row(row) {
                Ok(r) => ret_res.push(r),
                Err(e) => ret_err.push(e),
            }
        }

        (ret_res, ret_err)
    }

    /// Find repository by ID
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails or if the repository paths do not exist.
    ///
    /// # Panics
    ///
    /// Panics if no repository with the given ID is in the database.
    pub async fn get_by_id(
        tx: &tokio_postgres::Transaction<'_>,
        id: DbRepositoryId,
    ) -> Result<Self, RepositoryError> {
        match Self::find_by_id(tx, id).await {
            Ok(Some(x)) => Ok(x),
            Ok(None) => panic!("no repository with id {id} in database"),
            Err(e) => Err(e),
        }
    }

    /// Find repository by ID
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn find_by_id(
        tx: &tokio_postgres::Transaction<'_>,
        id: DbRepositoryId,
    ) -> Result<Option<Self>, RepositoryError> {
        let rows = tx
            .query("SELECT id, name, path, nixfile_path, created_at, last_synced_at FROM repositories WHERE id = $1", &[&id])
            .await
            .map_err(|error| {
                DbQueryError {
                    action: "find_by_id",
                    entity_type: EntityType::Repository,
                    raw_id: Some(id.bare_i32()),
                    clauses: vec![],
                    error,
                }
            })
            .map_err(RepositoryError::Query)?;

        rows.first().map(Self::from_row).transpose()
    }

    /// Find repository by path
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn find_by_path(
        tx: &tokio_postgres::Transaction<'_>,
        path: &str,
    ) -> Result<Option<Self>, RepositoryError> {
        let rows = tx
            .query("SELECT id, name, path, nixfile_path, created_at, last_synced_at FROM repositories WHERE path = $1", &[&path])
            .await
            .map_err(|error| {
                DbQueryError {
                    action: "find_by_path",
                    entity_type: EntityType::Repository,
                    raw_id: None,
                    clauses: vec![path.to_owned()],
                    error,
                }
            })
            .map_err(RepositoryError::Query)?;

        rows.first().map(Self::from_row).transpose()
    }
}
