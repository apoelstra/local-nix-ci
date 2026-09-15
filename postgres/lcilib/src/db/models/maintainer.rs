// SPDX-License-Identifier: GPL-3.0-or-later

use postgres_types::{FromSql, ToSql};
use std::fmt;

/// Newtype wrapper for maintainer IDs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromSql, ToSql)]
#[postgres(transparent)]
pub struct DbMaintainerId(i32);

impl DbMaintainerId {
    pub fn bare_i32(self) -> i32 {
        self.0
    }
}

impl fmt::Display for DbMaintainerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A maintainer whose ACKs count toward review approval.
#[derive(Debug, Clone)]
pub struct Maintainer {
    pub id: DbMaintainerId,
    pub username: String,
    pub domain: String,
    pub review_score: f32,
}

impl Maintainer {
    #[expect(dead_code)] // FIXME: no directly-manipulated 'maintainer' rows in the tool yet
    pub(crate) fn from_row(row: &tokio_postgres::Row) -> Self {
        Self {
            id: row.get("id"),
            username: row.get("username"),
            domain: row.get("domain"),
            review_score: row.get("review_score"),
        }
    }
}

/// Data required to create a new maintainer.
#[derive(Debug, Clone)]
pub struct NewMaintainer {
    pub username: String,
    pub domain: String,
    pub review_score: f32,
}
