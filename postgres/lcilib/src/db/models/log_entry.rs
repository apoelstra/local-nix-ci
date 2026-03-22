// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::{DateTime, Utc};
use core::fmt;
use postgres_types::{FromSql, ToSql};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromSql, ToSql)]
#[postgres(transparent)]
pub struct DbLogEntryId(i32);

impl DbLogEntryId {
    /// An i32 representation of the log entry ID.
    pub fn bare_i32(self) -> i32 {
        self.0
    }
}

impl fmt::Display for DbLogEntryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[log_entry {}]", self.0)
    }
}

/// Log entry model
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub id: DbLogEntryId,
    pub entity_type: String,
    pub entity_id: i32,
    pub action: String,
    pub description: Option<String>,
    pub reason: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl LogEntry {
    pub(crate) fn from_row(row: &tokio_postgres::Row) -> Self {
        Self {
            id: row.get("id"),
            entity_type: row.get("entity_type"),
            entity_id: row.get("entity_id"),
            action: row.get("action"),
            description: row.get("description"),
            reason: row.get("reason"),
            timestamp: row.get("timestamp"),
        }
    }
}
