// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::{DateTime, Utc};
use std::str::FromStr;
use std::fmt;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AckStatus {
    Pending,
    Failed,
    Posted,
    External,
}

impl FromStr for CommitType {
    type Err = ParseEnumError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "normal" => Ok(Self::Normal),
            "single" => Ok(Self::Single),
            "tip" => Ok(Self::Tip),
            "merge" => Ok(Self::Merge),
            _ => Err(ParseEnumError::new("CommitType", s.to_string())),
        }
    }
}

impl FromStr for ReviewStatus {
    type Err = ParseEnumError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "unreviewed" => Ok(Self::Unreviewed),
            "rejected" => Ok(Self::Rejected),
            "approved" => Ok(Self::Approved),
            _ => Err(ParseEnumError::new("ReviewStatus", s.to_string())),
        }
    }
}

impl FromStr for MergeStatus {
    type Err = ParseEnumError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "cancelled" => Ok(Self::Cancelled),
            "failed" => Ok(Self::Failed),
            "pushed" => Ok(Self::Pushed),
            _ => Err(ParseEnumError::new("MergeStatus", s.to_string())),
        }
    }
}

impl FromStr for CiStatus {
    type Err = ParseEnumError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "unstarted" => Ok(Self::Unstarted),
            "skipped" => Ok(Self::Skipped),
            "failed" => Ok(Self::Failed),
            "passed" => Ok(Self::Passed),
            _ => Err(ParseEnumError::new("CiStatus", s.to_string())),
        }
    }
}

impl FromStr for AckStatus {
    type Err = ParseEnumError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "failed" => Ok(Self::Failed),
            "posted" => Ok(Self::Posted),
            "external" => Ok(Self::External),
            _ => Err(ParseEnumError::new("AckStatus", s.to_string())),
        }
    }
}

impl AckStatus {
    pub fn as_str(self) -> &'static str {
        self.as_str2()
    }

    /// This goofy method is needed in order to obtain an `&dyn ToSql`
    /// which you cannot get from a `&'static str`, since `ToSql` is
    /// not implemented for `str`, just for `&str`.
    pub fn as_str2(self) -> &'static &'static str {
        match self {
            Self::Pending => &"pending",
            Self::Failed => &"failed",
            Self::Posted => &"posted",
            Self::External => &"external",
        }
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CiStatus {
    Unstarted,
    Skipped,
    Failed,
    Passed,
}

impl CiStatus {
    pub fn as_str(self) -> &'static str {
        self.as_str2()
    }

    /// This goofy method is needed in order to obtain an `&dyn ToSql`
    /// which you cannot get from a `&'static str`, since `ToSql` is
    /// not implemented for `str`, just for `&str`.
    pub fn as_str2(self) -> &'static &'static str {
        match self {
            Self::Unstarted => &"unstarted",
            Self::Skipped => &"skipped",
            Self::Failed => &"failed",
            Self::Passed => &"passed",
        }
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeStatus {
    Pending,
    Cancelled,
    Failed,
    Pushed,
}

impl MergeStatus {
    pub fn as_str(self) -> &'static str {
        self.as_str2()
    }

    /// This goofy method is needed in order to obtain an `&dyn ToSql`
    /// which you cannot get from a `&'static str`, since `ToSql` is
    /// not implemented for `str`, just for `&str`.
    pub fn as_str2(self) -> &'static &'static str {
        match self {
            Self::Pending => &"pending",
            Self::Cancelled => &"cancelled",
            Self::Failed => &"failed",
            Self::Pushed => &"pushed",
        }
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewStatus {
    Unreviewed,
    Rejected,
    Approved,
}

impl ReviewStatus {
    pub fn as_str(self) -> &'static str {
        self.as_str2()
    }

    /// This goofy method is needed in order to obtain an `&dyn ToSql`
    /// which you cannot get from a `&'static str`, since `ToSql` is
    /// not implemented for `str`, just for `&str`.
    pub fn as_str2(self) -> &'static &'static str {
        match self {
            Self::Unreviewed => &"unreviewed",
            Self::Rejected => &"rejected",
            Self::Approved => &"approved",
        }
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitType {
    Normal,
    Single,
    Tip,
    Merge,
}

impl CommitType {
    pub fn as_str(self) -> &'static str {
        self.as_str2()
    }

    /// This goofy method is needed in order to obtain an `&dyn ToSql`
    /// which you cannot get from a `&'static str`, since `ToSql` is
    /// not implemented for `str`, just for `&str`.
    pub fn as_str2(self) -> &'static &'static str {
        match self {
            Self::Normal => &"normal",
            Self::Single => &"single",
            Self::Tip => &"tip",
            Self::Merge => &"merge",
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
    pub commit_type: CommitType,
    pub nix_derivation: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Pull request model
#[derive(Debug, Clone)]
pub struct PullRequest {
    pub id: i32,
    pub repository_id: i32,
    pub pr_number: i32,
    pub tip_commit_id: i32,
    pub review_status: ReviewStatus,
    pub priority: i32,
    pub ok_to_merge: bool,
    pub required_reviewers: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub synced_at: DateTime<Utc>,
}

/// PR-Commit relationship model
#[derive(Debug, Clone)]
pub struct PrCommit {
    pub id: i32,
    pub pull_request_id: i32,
    pub commit_id: i32,
    pub sequence_order: i32,
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
    pub commit_type: CommitType,
    pub nix_derivation: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewPullRequest {
    pub repository_id: i32,
    pub pr_number: i32,
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
    pub commit_type: Option<CommitType>,
    pub nix_derivation: Option<Option<String>>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdatePullRequest {
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
