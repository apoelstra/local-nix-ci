// SPDX-License-Identifier: GPL-3.0-or-later

pub mod log;

use chrono::{DateTime, Utc};
use std::fmt;
use postgres_types::{FromSql, ToSql};

pub use log::Log;

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
#[postgres(name = "ci_status")]
pub enum CiStatus {
    #[postgres(name = "unstarted")]
    Unstarted,
    #[postgres(name = "skipped")]
    Skipped,
    #[postgres(name = "failed")]
    Failed,
    #[postgres(name = "passed")]
    Passed,
}

impl fmt::Display for CiStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unstarted => write!(f, "unstarted"),
            Self::Skipped => write!(f, "skipped"),
            Self::Failed => write!(f, "failed"),
            Self::Passed => write!(f, "passed"),
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
    #[postgres(name = "failed")]
    Failed,
    #[postgres(name = "pushed")]
    Pushed,
}

impl fmt::Display for MergeStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Cancelled => write!(f, "cancelled"),
            Self::Failed => write!(f, "failed"),
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

/// Repository model
#[derive(Debug, Clone)]
pub struct Repository {
    pub id: i32,
    pub name: String,
    pub path: String,
    pub nixfile_path: String,
    pub created_at: DateTime<Utc>,
}

/// Commit model
#[derive(Debug, Clone)]
pub struct Commit {
    pub id: i32,
    pub repository_id: i32,
    pub git_commit_id: String,
    pub jj_change_id: String,
    pub review_status: ReviewStatus,
    pub should_run_ci: bool,
    pub ci_status: CiStatus,
    pub nix_derivation: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Pull request model
#[derive(Debug, Clone)]
pub struct PullRequest {
    pub id: i32,
    pub repository_id: i32,
    pub pr_number: i32,
    pub title: String,
    pub body: String,
    pub tip_commit_id: i32,
    pub review_status: ReviewStatus,
    pub priority: i32,
    pub ok_to_merge: bool,
    pub required_reviewers: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub synced_at: DateTime<Utc>,
}


/// Stack model
#[derive(Debug, Clone)]
pub struct Stack {
    pub id: i32,
    pub repository_id: i32,
    pub target_branch: String,
    pub status: MergeStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Stack-Commit relationship model
#[derive(Debug, Clone)]
pub struct StackCommit {
    pub id: i32,
    pub stack_id: i32,
    pub commit_id: i32,
    pub sequence_order: i32,
}

/// ACK model
#[derive(Debug, Clone)]
pub struct Ack {
    pub id: i32,
    pub pull_request_id: i32,
    pub commit_id: i32,
    pub reviewer_name: String,
    pub message: String,
    pub status: AckStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Allowed approver model
#[derive(Debug, Clone)]
pub struct AllowedApprover {
    pub id: i32,
    pub repository_id: i32,
    pub approver_name: String,
    pub created_at: DateTime<Utc>,
}

/// PR-Commit relationship model
#[derive(Debug, Clone)]
pub struct PrCommit {
    pub id: i32,
    pub pull_request_id: i32,
    pub commit_id: i32,
    pub sequence_order: i32,
    pub commit_type: CommitType,
    pub is_current: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Log entry model
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub id: i32,
    pub entity_type: String,
    pub entity_id: i32,
    pub action: String,
    pub description: Option<String>,
    pub reason: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// Input structs for creating new records
#[derive(Debug, Clone)]
pub struct NewRepository {
    pub name: String,
    pub path: String,
    pub nixfile_path: String,
}

#[derive(Debug, Clone)]
pub struct NewCommit {
    pub repository_id: i32,
    pub git_commit_id: String,
    pub jj_change_id: String,
    pub review_status: ReviewStatus,
    pub should_run_ci: bool,
    pub ci_status: CiStatus,
    pub nix_derivation: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewPullRequest {
    pub repository_id: i32,
    pub pr_number: i32,
    pub title: String,
    pub body: String,
    pub tip_commit_id: i32,
    pub review_status: ReviewStatus,
    pub priority: i32,
    pub ok_to_merge: bool,
    pub required_reviewers: i32,
}

#[derive(Debug, Clone)]
pub struct NewStack {
    pub repository_id: i32,
    pub target_branch: String,
    pub status: MergeStatus,
}

#[derive(Debug, Clone)]
pub struct NewAck {
    pub pull_request_id: i32,
    pub commit_id: i32,
    pub reviewer_name: String,
    pub message: String,
    pub status: AckStatus,
}

#[derive(Debug, Clone)]
pub struct NewAllowedApprover {
    pub repository_id: i32,
    pub approver_name: String,
}

/// Update structs for modifying existing records
#[allow(clippy::option_option)] // optional update of an Option type
#[derive(Debug, Clone, Default)]
pub struct UpdateCommit {
    pub review_status: Option<ReviewStatus>,
    pub should_run_ci: Option<bool>,
    pub ci_status: Option<CiStatus>,
    pub nix_derivation: Option<Option<String>>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdatePullRequest {
    pub title: Option<String>,
    pub body: Option<String>,
    pub tip_commit_id: Option<i32>,
    pub review_status: Option<ReviewStatus>,
    pub priority: Option<i32>,
    pub ok_to_merge: Option<bool>,
    pub required_reviewers: Option<i32>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateStack {
    pub target_branch: Option<String>,
    pub status: Option<MergeStatus>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateAck {
    pub commit_id: Option<i32>,
    pub message: Option<String>,
    pub status: Option<AckStatus>,
}
