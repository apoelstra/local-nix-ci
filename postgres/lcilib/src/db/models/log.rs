// SPDX-License-Identifier: GPL-3.0-or-later

use crate::db::Transaction;
use crate::db::util::EntityType;
use chrono::{DateTime, Utc};

/// A log entry from the database
#[derive(Debug, Clone)]
pub struct Log {
    pub id: i32,
    pub entity_type: EntityType,
    pub entity_id: i32,
    pub action: String,
    pub description: Option<String>,
    pub reason: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// Options for filtering log queries
#[derive(Debug, Clone, Default)]
pub struct LogFilter {
    /// Filter by entity type
    pub entity_type: Option<EntityType>,
    /// Filter by specific entity IDs
    pub entity_ids: Vec<i32>,
    /// Only show logs at or after this time
    pub since: Option<String>,
    /// Only show logs at or before this time
    pub until: Option<String>,
}

impl Log {
    /// Query logs with optional filtering
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub async fn query_filtered(
        tx: &Transaction<'_>,
        filter: LogFilter,
    ) -> Result<Vec<Self>, tokio_postgres::Error> {
        let mut query =
            "SELECT id, entity_type, entity_id, action, description, reason, timestamp FROM logs"
                .to_string();
        let mut conditions = Vec::new();
        let mut params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = Vec::new();
        let mut param_count = 1;

        // Filter by entity type
        if let Some(entity_type) = &filter.entity_type {
            conditions.push(format!("entity_type = ${}", param_count));
            params.push(entity_type);
            param_count += 1;
        }

        // Filter by entity IDs
        if !filter.entity_ids.is_empty() {
            let mut id_conditions = Vec::new();
            for entity_id in &filter.entity_ids {
                id_conditions.push(format!("entity_id = ${}", param_count));
                params.push(entity_id);
                param_count += 1;
            }
            conditions.push(format!("({})", id_conditions.join(" OR ")));
        }

        // Filter by since date
        if let Some(since) = &filter.since {
            conditions.push(format!("timestamp >= ${}", param_count));
            params.push(since);
            param_count += 1;
        }

        // Filter by until date
        #[expect(unused_assignments)] // final param_count += 1
        if let Some(until) = &filter.until {
            conditions.push(format!("timestamp <= ${}", param_count));
            params.push(until);
            param_count += 1;
        }

        // Add WHERE clause if we have conditions
        if !conditions.is_empty() {
            query.push_str(&format!(" WHERE {}", conditions.join(" AND ")));
        }

        // Order by timestamp
        query.push_str(" ORDER BY timestamp ASC");

        let rows = tx.inner.query(&query, &params).await?;

        let mut logs = Vec::new();
        for row in rows {
            logs.push(Self {
                id: row.get(0),
                entity_type: row.get(1),
                entity_id: row.get(2),
                action: row.get(3),
                description: row.get(4),
                reason: row.get(5),
                timestamp: row.get(6),
            });
        }

        Ok(logs)
    }

    /// Query logs for a specific entity
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub async fn query_for_entity(
        tx: &Transaction<'_>,
        entity_type: EntityType,
        entity_id: i32,
        since: Option<&str>,
        until: Option<&str>,
    ) -> Result<Vec<Self>, tokio_postgres::Error> {
        let filter = LogFilter {
            entity_type: Some(entity_type),
            entity_ids: vec![entity_id],
            since: since.map(String::from),
            until: until.map(String::from),
        };
        Self::query_filtered(tx, filter).await
    }

    /// Query logs for multiple entities
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub async fn query_for_entities(
        tx: &Transaction<'_>,
        entities: &[(EntityType, i32)],
        since: Option<&str>,
        until: Option<&str>,
    ) -> Result<Vec<Self>, tokio_postgres::Error> {
        if entities.is_empty() {
            return Ok(Vec::new());
        }

        // Convert date strings to owned values to avoid lifetime issues
        let since_owned = since.map(String::from);
        let until_owned = until.map(String::from);

        // Build conditions for each entity type/id pair
        let mut query = "SELECT id, entity_type, entity_id, action, description, reason, timestamp FROM logs WHERE ".to_string();
        let mut entity_conditions = Vec::new();
        let mut params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = Vec::new();
        let mut param_count = 1;

        for (entity_type, entity_id) in entities {
            entity_conditions.push(format!(
                "(entity_type = ${} AND entity_id = ${})",
                param_count,
                param_count + 1
            ));
            params.push(entity_type);
            params.push(entity_id);
            param_count += 2;
        }

        query.push_str(&format!("({})", entity_conditions.join(" OR ")));

        // Add date filtering
        if let Some(ref since_date) = since_owned {
            query.push_str(&format!(" AND timestamp >= ${}", param_count));
            params.push(since_date);
            param_count += 1;
        }

        if let Some(ref until_date) = until_owned {
            query.push_str(&format!(" AND timestamp <= ${}", param_count));
            params.push(until_date);
        }

        // Order by timestamp
        query.push_str(" ORDER BY timestamp ASC");

        let rows = tx.inner.query(&query, &params).await?;

        let mut logs = Vec::new();
        for row in rows {
            logs.push(Self {
                id: row.get(0),
                entity_type: row.get(1),
                entity_id: row.get(2),
                action: row.get(3),
                description: row.get(4),
                reason: row.get(5),
                timestamp: row.get(6),
            });
        }

        Ok(logs)
    }

    /// Format this log entry for display
    pub fn format_for_display(&self) -> String {
        let mut log_line = format!(
            "{} - action: {}",
            self.timestamp.format("%Y-%m-%d %H:%M:%S"),
            self.action
        );

        if let Some(desc) = &self.description {
            log_line.push_str(&format!(" - description: {}", desc));
        }

        if let Some(reason) = &self.reason {
            log_line.push_str(&format!(" - reason: {}", reason));
        }

        log_line
    }
}
