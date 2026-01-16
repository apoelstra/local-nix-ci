// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::PathBuf;

use crate::git::GitCommit;

#[derive(Copy, Debug, Clone, PartialEq, Eq, Default)]
#[derive(serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CiStatus {
    #[default]
    Unstarted,
    Started,
    Success,
    Failed,
}

#[derive(Copy, Debug, Clone, PartialEq, Eq, Default)]
#[derive(serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MergeStatus {
    #[default]
    Unstarted,
    NeedSig,
    Pushed,
}

#[derive(Copy, Debug, Clone, PartialEq, Eq, Default)]
#[derive(serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReviewStatus {
    #[default]
    Unreviewed,
    NeedsChange,
    Nacked,
    Approved,
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
    pub derivation: Option<String>,
    #[serde(default)]
    #[serde(rename = "claimedby")]
    pub claimed_by: Option<String>,

    // PR data
    #[serde(default)]
    pub pr_title: Option<String>,
    #[serde(default)]
    pub pr_author: Option<String>,
    #[serde(default)]
    pub pr_number: Option<usize>,
    #[serde(default)]
    pub base_commit: Option<GitCommit>,
    #[serde(default)]
    pub merge_change_id: Option<String>,
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
