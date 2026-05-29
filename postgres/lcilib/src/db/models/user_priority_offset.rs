// SPDX-License-Identifier: GPL-3.0-or-later

use core::fmt;
use postgres_types::{FromSql, ToSql};

use crate::db::{DbQueryError, EntityType, Transaction};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromSql, ToSql)]
#[postgres(transparent)]
pub struct DbUserPriorityOffsetId(i32);

impl DbUserPriorityOffsetId {
    /// An i32 representation of the user priority offset ID.
    pub fn bare_i32(self) -> i32 {
        self.0
    }
}

impl fmt::Display for DbUserPriorityOffsetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[user_priority_offset {}]", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct UserPriorityOffset {
    pub id: DbUserPriorityOffsetId,
    pub github_username: String,
    pub priority_offset: f32,
}

#[derive(Debug, Clone)]
pub struct NewUserPriorityOffset {
    pub github_username: String,
    pub priority_offset: f32,
}

impl UserPriorityOffset {
    /// Gets the priority offset for a GitHub username.
    /// Returns 0.0 if no offset is found for the user.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn get_offset_by_username(
        tx: &Transaction<'_>,
        github_username: &str,
    ) -> Result<f32, DbQueryError> {
        let result = tx
            .inner
            .query_opt(
                "SELECT priority_offset FROM user_priority_offsets WHERE github_username = $1",
                &[&github_username],
            )
            .await
            .map_err(|error| DbQueryError {
                action: "get_offset_by_username",
                entity_type: EntityType::System,
                raw_id: None,
                clauses: vec![format!("github_username = '{}'", github_username)],
                error,
            })?;

        Ok(result.map_or(0.0, |row| row.get("priority_offset")))
    }
}
