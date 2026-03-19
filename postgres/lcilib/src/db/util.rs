// SPDX-License-Identifier: GPL-3.0-or-later

use tokio_postgres::{Error, Transaction, Client};
use std::str::FromStr;

/// Entity types that can be logged, matching the database enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityType {
    Commit,
    PullRequest,
    Stack,
    Ack,
    System,
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
            _ => Err(crate::db::models::ParseEnumError::new("EntityType", s.to_string())),
        }
    }
}

impl EntityType {
    /// Convert to string representation for database storage.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Commit => "commit",
            Self::PullRequest => "pull_request",
            Self::Stack => "stack",
            Self::Ack => "ack",
            Self::System => "system",
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
pub async fn log_action(
    tx: &Transaction<'_>,
    entity_type: EntityType,
    entity_id: i32,
    action: &str,
    description: Option<&str>,
    reason: Option<&str>,
) -> Result<(), Error> {
    tx.execute(
        r#"
        INSERT INTO logs (entity_type, entity_id, action, description, reason)
        VALUES ($1, $2, $3, $4, $5)
        "#,
        &[&entity_type.as_str(), &entity_id, &action, &description, &reason],
    )
    .await?;

    Ok(())
}

/// Log an action using a database client (creates its own transaction)
///
/// Convenience wrapper around `log_action` for cases where you don't already have a transaction.
///
/// # Errors
///
/// Errors if the transaction or INSERT query fails.
pub async fn log_action_simple(
    client: &mut Client,
    entity_type: EntityType,
    entity_id: i32,
    action: &str,
    description: Option<&str>,
    reason: Option<&str>,
) -> Result<(), Error> {
    let tx = client.transaction().await?;
    log_action(&tx, entity_type, entity_id, action, description, reason).await?;
    tx.commit().await?;
    Ok(())
}

/// Determines whether a given table exists.
///
/// Takes a transaction rather than a client on the assumption that
/// this information will be needed for subsequent operations.
///
/// # Errors
///
/// Errors if the `SELECT` query fails.
pub async fn table_exists(
    tx: &Transaction<'_>,
    table: &str,
) -> Result<bool, Error> {
    let row = tx
        .query_one(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM information_schema.tables
                WHERE table_name = $1
            )
            "#,
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
pub async fn get_schema_version(tx: &Transaction<'_>) -> Result<i32, Error> {
    let row = tx
        .query_one("SELECT schema_version FROM global", &[])
        .await?;

    Ok(row.get::<_, i32>(0))
}

/// Check if a repository exists by path
///
/// # Errors
///
/// Errors if the SELECT query fails.
pub async fn repository_exists_by_path(
    tx: &Transaction<'_>,
    path: &str,
) -> Result<bool, Error> {
    let row = tx
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
pub async fn get_repository_id_by_path(
    tx: &Transaction<'_>,
    path: &str,
) -> Result<Option<i32>, Error> {
    let rows = tx
        .query("SELECT id FROM repositories WHERE path = $1", &[&path])
        .await?;

    Ok(rows.first().map(|row| row.get::<_, i32>(0)))
}

/// Check if a commit exists by git commit ID and repository
///
/// # Errors
///
/// Errors if the SELECT query fails.
pub async fn commit_exists_by_git_id(
    tx: &Transaction<'_>,
    repository_id: i32,
    git_commit_id: &str,
) -> Result<bool, Error> {
    let row = tx
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
pub async fn pull_request_exists(
    tx: &Transaction<'_>,
    repository_id: i32,
    pr_number: i32,
) -> Result<bool, Error> {
    let row = tx
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
