// SPDX-License-Identifier: GPL-3.0-or-later

use crate::git::CommitId;
use crate::gh;
use chrono::{DateTime, Utc};

/// Custom deserializer for the RFC 3339 timestamps that Forgejo returns
/// (e.g. `2024-01-15T12:34:56Z` or `2024-01-15T12:34:56+02:00`).
fn deserialize_forgejo_datetime<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let s = String::deserialize(deserializer)?;
    DateTime::parse_from_rfc3339(&s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(serde::de::Error::custom)
}

#[derive(serde::Deserialize, Debug)]
pub struct User {
    pub login: String,
}

#[derive(serde::Deserialize, Debug)]
pub struct BranchInfo {
    #[serde(rename = "ref")]
    pub ref_name: String,
    pub sha: CommitId,
}

#[derive(serde::Deserialize, Debug)]
pub struct PullRequest {
    pub number: i32,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub body: String,
    pub user: User,
    pub head: BranchInfo,
    pub base: BranchInfo,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub mergeable: bool,
    #[serde(rename = "merged_at")]
    pub merged_at: Option<String>,
    #[serde(default)]
    pub merged: bool,
    #[serde(rename = "updated_at", deserialize_with = "deserialize_forgejo_datetime")]
    pub updated_at: DateTime<Utc>,
}

#[derive(serde::Deserialize, Debug)]
pub struct Comment {
    pub user: User,
    #[serde(default)]
    pub body: String,
    #[serde(rename = "created_at")]
    pub created_at: String,
}

#[derive(serde::Deserialize, Debug)]
pub struct Review {
    pub user: User,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub state: String,
    #[serde(rename = "submitted_at", default)]
    pub submitted_at: String,
}

#[derive(serde::Deserialize, Debug)]
pub struct CommitEntry {
    pub sha: CommitId,
}

impl PullRequest {
    /// Translate a Forgejo `PullRequest` (plus separately-fetched commits / comments / reviews)
    /// into a `gh::PrInfo`, so callers can treat `fj` and `gh` interchangeably.
    pub fn into_gh_pr_info(
        self,
        commits: Vec<CommitId>,
        comments: Vec<Comment>,
        reviews: Vec<Review>,
    ) -> gh::PrInfo {
        let closed = matches!(self.state.as_str(), "closed") && !self.merged;
        let mergeable = if self.mergeable { "MERGEABLE".to_string() } else { String::new() };
        gh::PrInfo {
            title: self.title,
            body: self.body,
            number: self.number,
            author: gh::serde_types::Author { login: self.user.login },
            commits: commits
                .into_iter()
                .map(|oid| gh::serde_types::Commit { oid })
                .collect(),
            comments: comments
                .into_iter()
                .map(|c| gh::serde_types::Comment {
                    author: gh::serde_types::Author { login: c.user.login },
                    body: c.body,
                    created_at: c.created_at,
                })
                .collect(),
            reviews: reviews
                .into_iter()
                .map(|r| gh::serde_types::Review {
                    author: gh::serde_types::Author { login: r.user.login },
                    body: r.body,
                    state: r.state,
                    submitted_at: r.submitted_at,
                })
                .collect(),
            head_commit: self.head.sha,
            base_ref: self.base.ref_name,
            state: self.state,
            mergeable,
            merge_state_status: String::new(),
            is_draft: self.draft,
            closed,
            merged_at: self.merged_at,
        }
    }

    pub fn updated_at(&self) -> DateTime<Utc> { self.updated_at }
}
