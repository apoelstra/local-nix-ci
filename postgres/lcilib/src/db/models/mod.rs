// SPDX-License-Identifier: GPL-3.0-or-later

mod ack;
mod allowed_approver;
mod commit;
pub mod log;
mod log_entry;
mod pr_commit;
mod pull_request;
mod repository;
mod stack;

use postgres_types::{FromSql, ToSql};
use std::fmt;

pub use ack::{Ack, DbAckId, NewAck, UpdateAck};
pub use allowed_approver::{AllowedApprover, DbAllowedApproverId, NewAllowedApprover};
pub use commit::{CiStatus, Commit, CommitToTest, DbCommitId, NewCommit, UpdateCommit};
pub use log::Log;
pub use log_entry::{DbLogEntryId, LogEntry};
pub use pr_commit::{DbPrCommitId, PrCommit};
pub use pull_request::{DbPullRequestId, NewPullRequest, PullRequest, UpdatePullRequest};
pub use repository::{DbRepositoryId, NewRepository, Repository, RepositoryError, RepoShell, RepoShellLock};
pub use stack::{DbStackId, NewStack, Stack, StackCommit, UpdateStack};

/// Error type for parsing enum values from strings
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseEnumError {
    enum_name: &'static str,
    invalid_value: String,
}

impl ParseEnumError {
    pub(super) fn new(enum_name: &'static str, invalid_value: String) -> Self {
        Self {
            enum_name,
            invalid_value,
        }
    }
}

impl fmt::Display for ParseEnumError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid {}: '{}'", self.enum_name, self.invalid_value)
    }
}

impl std::error::Error for ParseEnumError {}

/// Database enums matching the schema
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromSql, ToSql)]
#[postgres(name = "ack_status")]
pub enum AckStatus {
    #[postgres(name = "pending")]
    Pending,
    #[postgres(name = "failed")]
    Failed,
    #[postgres(name = "posted")]
    Posted,
    #[postgres(name = "external")]
    External,
}

impl fmt::Display for AckStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Failed => write!(f, "failed"),
            Self::Posted => write!(f, "posted"),
            Self::External => write!(f, "external"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromSql, ToSql)]
#[postgres(name = "merge_status")]
pub enum MergeStatus {
    #[postgres(name = "pending")]
    Pending,
    #[postgres(name = "cancelled")]
    Cancelled,
    #[postgres(name = "conflicted")]
    Conflicted,
    #[postgres(name = "pushed")]
    Pushed,
}

impl fmt::Display for MergeStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Cancelled => write!(f, "cancelled"),
            Self::Conflicted => write!(f, "conflicted"),
            Self::Pushed => write!(f, "pushed"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromSql, ToSql)]
#[postgres(name = "review_status")]
pub enum ReviewStatus {
    #[postgres(name = "unreviewed")]
    Unreviewed,
    #[postgres(name = "rejected")]
    Rejected,
    #[postgres(name = "approved")]
    Approved,
}

impl fmt::Display for ReviewStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreviewed => write!(f, "unreviewed"),
            Self::Rejected => write!(f, "rejected"),
            Self::Approved => write!(f, "approved"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromSql, ToSql)]
#[postgres(name = "commit_type")]
pub enum CommitType {
    #[postgres(name = "normal")]
    Normal,
    #[postgres(name = "single")]
    Single,
    #[postgres(name = "tip")]
    Tip,
    #[postgres(name = "merge")]
    Merge,
}

impl fmt::Display for CommitType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Normal => write!(f, "normal"),
            Self::Single => write!(f, "single"),
            Self::Tip => write!(f, "tip"),
            Self::Merge => write!(f, "merge"),
        }
    }
}

/// Utility struct to allow parsing numbers as usizes from the database.
#[derive(FromSql)]
#[postgres(transparent)]
struct Count(i64);
impl From<Count> for usize {
    fn from(value: Count) -> Self {
        Self::try_from(value.0).expect("any count from the database fits in i64")
    }
}

/// Statistics about a list of commits in the database.
///
/// Contains information about what's tested, but not about what's
/// signed (since the database doesn't track that).
pub struct CommitCounts {
    pub total: usize,
    pub approved: usize,
    pub unapproved: usize,
    pub untested: usize,
    /// Both 'approved' and CI passed.
    pub ready: usize,
}

impl CommitCounts {
    fn from_row(row: &tokio_postgres::Row) -> Self {
        let total = row.get::<_, Count>("total").into();
        let approved = row.get::<_, Count>("approved").into();
        let untested = row.get::<_, Count>("untested").into();
        let ready = row.get::<_, Count>("ready").into();
        Self {
            total,
            approved,
            unapproved: total - approved,
            untested,
            ready,
        }
    }
}
