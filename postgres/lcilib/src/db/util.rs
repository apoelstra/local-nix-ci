// SPDX-License-Identifier: GPL-3.0-or-later

use std::str::FromStr;
use tokio_postgres::Error;

use crate::db::{DbQueryError, Transaction};

/// Entity types that can be logged, matching the database enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, postgres_types::FromSql, postgres_types::ToSql)]
#[postgres(name = "entity_type")]
pub enum EntityType {
    #[postgres(name = "repository")]
    Repository,
    #[postgres(name = "commit")]
    Commit,
    #[postgres(name = "pull_request")]
    PullRequest,
    #[postgres(name = "stack")]
    Stack,
    #[postgres(name = "ack")]
    Ack,
    #[postgres(name = "system")]
    System,
}

impl std::fmt::Display for EntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Repository => write!(f, "repository"),
            Self::Commit => write!(f, "commit"),
            Self::PullRequest => write!(f, "pull_request"),
            Self::Stack => write!(f, "stack"),
            Self::Ack => write!(f, "ack"),
            Self::System => write!(f, "system"),
        }
    }
}

impl FromStr for EntityType {
    type Err = crate::db::models::ParseEnumError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "commit" => Ok(Self::Commit),
            "pull_request" | "pr" => Ok(Self::PullRequest),
            "stack" => Ok(Self::Stack),
            "ack" => Ok(Self::Ack),
            "system" => Ok(Self::System),
            _ => Err(crate::db::models::ParseEnumError::new(
                "EntityType",
                s.to_string(),
            )),
        }
    }
}

/// Log an action to the polymorphic logs table
///
/// # Arguments
/// * `tx` - Database transaction
/// * `entity_type` - Type of entity being logged
/// * `entity_id` - ID of the entity (use 0 for system-level logs)
/// * `action` - Short action description
/// * `description` - Optional detailed description
/// * `reason` - Optional reason for the action
///
/// # Errors
///
/// Errors if the INSERT query fails.
pub(super) async fn log_action(
    tx: &Transaction<'_>,
    entity_type: EntityType,
    entity_id: i32,
    action: &str,
    description: Option<&str>,
    reason: Option<&str>,
) -> Result<u64, DbQueryError> {
    tx.inner.execute(
        r#"
        INSERT INTO logs (entity_type, entity_id, action, description, reason)
        VALUES ($1, $2, $3, $4, $5)
        "#,
        &[&entity_type, &entity_id, &action, &description, &reason],
    )
    .await
    .map_err(|error| DbQueryError {
        action: "insert log",
        entity_type: EntityType::System,
        raw_id: None,
        clauses: vec![
            "entity_type".into(),
            "entity_id".into(),
            "action".into(),
            "description".into(),
            "reason".into(),
        ],
        error,
    })
}

/// Determines whether a given table exists.
///
/// Takes a transaction rather than a client on the assumption that
/// this information will be needed for subsequent operations.
///
/// # Errors
///
/// Errors if the `SELECT` query fails.
pub(super) async fn table_exists(tx: &tokio_postgres::Transaction<'_>, table: &str) -> Result<bool, Error> {
    let row = tx
        .query_one(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema = 'public' AND table_name = $1)",
            &[&table],
        )
        .await?;

    Ok(row.get::<_, bool>(0))
}

/// Get the current schema version from the global table
///
/// # Errors
///
/// Errors if the SELECT query fails or if no version is found.
pub(super) async fn get_schema_version(tx: &Transaction<'_>) -> Result<i32, Error> {
    let row = tx
        .inner
        .query_one("SELECT schema_version FROM global", &[])
        .await?;

    Ok(row.get::<_, i32>(0))
}

/// Check if a repository exists by path
///
/// # Errors
///
/// Errors if the SELECT query fails.
pub(super) async fn repository_exists_by_path(tx: &Transaction<'_>, path: &str) -> Result<bool, Error> {
    let row = tx
        .inner
        .query_one(
            "SELECT EXISTS (SELECT 1 FROM repositories WHERE path = $1)",
            &[&path],
        )
        .await?;

    Ok(row.get::<_, bool>(0))
}

/// Get repository ID by path
///
/// # Errors
///
/// Errors if the SELECT query fails or if no repository is found.
pub(super) async fn get_repository_id_by_path(
    tx: &Transaction<'_>,
    path: &str,
) -> Result<Option<i32>, Error> {
    let rows = tx
        .inner
        .query("SELECT id FROM repositories WHERE path = $1", &[&path])
        .await?;

    Ok(rows.first().map(|row| row.get::<_, i32>(0)))
}

/// Check if a commit exists by git commit ID and repository
///
/// # Errors
///
/// Errors if the SELECT query fails.
pub(super) async fn commit_exists_by_git_id(
    tx: &Transaction<'_>,
    repository_id: i32,
    git_commit_id: &str,
) -> Result<bool, Error> {
    let row = tx
        .inner
        .query_one(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM commits
                WHERE repository_id = $1 AND git_commit_id = $2
            )
            "#,
            &[&repository_id, &git_commit_id],
        )
        .await?;

    Ok(row.get::<_, bool>(0))
}

/// Check if a pull request exists by PR number and repository
///
/// # Errors
///
/// Errors if the SELECT query fails.
pub(super) async fn pull_request_exists(
    tx: &Transaction<'_>,
    repository_id: i32,
    pr_number: i32,
) -> Result<bool, Error> {
    let row = tx
        .inner
        .query_one(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM pull_requests
                WHERE repository_id = $1 AND pr_number = $2
            )
            "#,
            &[&repository_id, &pr_number],
        )
        .await?;

    Ok(row.get::<_, bool>(0))
}
