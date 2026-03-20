// SPDX-License-Identifier: GPL-3.0-or-later

use crate::git::GitCommit;

/// An element of the `commits` array returned by `gh pr view --json`.
#[derive(serde::Deserialize, Debug)]
pub struct Commit {
    pub oid: GitCommit,
}

/// An "author" as returned by Github.
#[derive(serde::Deserialize, Debug)]
pub struct Author {
    pub login: String,
}

/// A comment on a PR as returned by Github.
#[derive(serde::Deserialize, Debug)]
pub struct Comment {
    pub author: Author,
    #[serde(default)]
    pub body: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

/// A review on a PR as returned by Github.
#[derive(serde::Deserialize, Debug)]
pub struct Review {
    pub author: Author,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub state: String,
    #[serde(rename = "submittedAt")]
    pub submitted_at: String,
}

/// The output of `gh pr view --json`
#[derive(serde::Deserialize, Debug)]
pub struct PrInfo {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub body: String,
    pub author: Author,
    #[serde(default)]
    pub commits: Vec<Commit>,
    #[serde(default)]
    pub comments: Vec<Comment>,
    #[serde(default)]
    pub reviews: Vec<Review>,
    #[serde(rename = "headRefOid")]
    pub head_commit: GitCommit,
    #[serde(rename = "baseRefName")]
    pub base_ref: String,
}

impl PrInfo {
    /// Iterator over all the `oid`s in the `commits` array.
    pub fn commit_ids(&self) -> impl Iterator<Item = &GitCommit> {
        self.commits.iter().map(|c| &c.oid)
    }
}
