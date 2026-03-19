// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::{DateTime, Utc};

/// Database enums matching the schema
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AckStatus {
    Pending,
    Failed,
    Posted,
    External,
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

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "failed" => Some(Self::Failed),
            "posted" => Some(Self::Posted),
            "external" => Some(Self::External),
            _ => None,
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

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "unstarted" => Some(Self::Unstarted),
            "skipped" => Some(Self::Skipped),
            "failed" => Some(Self::Failed),
            "passed" => Some(Self::Passed),
            _ => None,
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

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "cancelled" => Some(Self::Cancelled),
            "failed" => Some(Self::Failed),
            "pushed" => Some(Self::Pushed),
            _ => None,
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

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "unreviewed" => Some(Self::Unreviewed),
            "rejected" => Some(Self::Rejected),
            "approved" => Some(Self::Approved),
            _ => None,
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

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "normal" => Some(Self::Normal),
            "single" => Some(Self::Single),
            "tip" => Some(Self::Tip),
            "merge" => Some(Self::Merge),
            _ => None,
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
