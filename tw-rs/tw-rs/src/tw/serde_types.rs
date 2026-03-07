// SPDX-License-Identifier: GPL-3.0-or-later

use core::fmt;
use std::path::PathBuf;

use crate::git::GitCommit;

#[derive(Copy, Debug, Clone, PartialEq, Eq, Default, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low,
    #[default]
    Medium,
    High,
}

impl PartialOrd for Priority {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Priority {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use Priority::*;
        match (self, other) {
            (High, High) | (Medium, Medium) | (Low, Low) => std::cmp::Ordering::Equal,
            (High, _) => std::cmp::Ordering::Greater,
            (_, High) => std::cmp::Ordering::Less,
            (Medium, Low) => std::cmp::Ordering::Greater,
            (Low, Medium) => std::cmp::Ordering::Less,
        }
    }
}

#[derive(Copy, Debug, Clone, PartialEq, Eq, Default, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CiStatus {
    #[default]
    Unstarted,
    Started,
    Success,
    Failed,
    Cancelled,
}

impl fmt::Display for CiStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unstarted => f.write_str("unstarted"),
            Self::Started => f.write_str("started"),
            Self::Success => f.write_str("success"),
            Self::Failed => f.write_str("FAILED"),
            Self::Cancelled => f.write_str("cancelled"),
        }
    }
}

#[derive(Copy, Debug, Clone, PartialEq, Eq, Default, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MergeStatus {
    Cancelled,
    #[default]
    Unstarted,
    NeedSig,
    Pushed,
}

impl fmt::Display for MergeStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled => f.write_str("cancelled"),
            Self::Unstarted => f.write_str("unstarted"),
            Self::NeedSig => f.write_str("NEEDS SIGNATURE"),
            Self::Pushed => f.write_str("pushed"),
        }
    }
}

#[derive(Copy, Debug, Clone, PartialEq, Eq, Default, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReviewStatus {
    #[default]
    Unreviewed,
    NeedsChange,
    Nacked,
    Approved,
    ApprovedNoCi,
}

impl fmt::Display for ReviewStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreviewed => f.write_str("unreviewed"),
            Self::NeedsChange => f.write_str("needs change"),
            Self::Nacked => f.write_str("NACKed"),
            Self::Approved => f.write_str("ACKed"),
            Self::ApprovedNoCi => f.write_str("ACKed (but skip CI)"),
        }
    }
}

#[derive(serde::Deserialize)]
pub struct Task {
    #[serde(default)]
    pub depends: Vec<uuid::Uuid>,
    #[serde(default)]
    pub tags: Vec<String>,

    pub project: String,
    pub repo_root: PathBuf,
    #[serde(default)]
    pub description: String,

    #[serde(default)]
    pub ci_status: CiStatus,
    #[serde(default)]
    pub merge_status: MergeStatus,
    #[serde(default)]
    pub review_status: ReviewStatus,
    #[serde(default)]
    pub review_notes: String,

    // Commit data
    #[serde(default)]
    pub commit_id: Option<GitCommit>,
    #[serde(default)]
    pub local_ci_commit_id: Option<String>,
    #[serde(default)]
    pub derivation: Option<String>,
    #[serde(default)]
    #[serde(rename = "claimedby")]
    pub claimed_by: Option<String>,
    #[serde(default)]
    pub priority: Priority,

    // PR data
    #[serde(default)]
    pub pr_title: Option<String>,
    #[serde(default)]
    pub pr_author: Option<String>,
    #[serde(default)]
    pub pr_number: Option<usize>,
    #[serde(default)]
    pub github_acks: String,
    #[serde(default)]
    pub base_commit: Option<GitCommit>,
    #[serde(default)]
    pub merge_change_id: Option<String>,
    #[serde(default)]
    pub merge_uuid: Option<uuid::Uuid>,
    #[serde(default)]
    pub base_ref: Option<String>,
}

impl Task {
    /// Checks whether the tag exists in the task's tage list.
    ///
    /// Is case-sensitive, though I may change this if I learn
    /// that it shouldn't be.
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|own| own == tag)
    }
}
