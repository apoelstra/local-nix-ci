// SPDX-License-Identifier: GPL-3.0-or-later

use crate::git::GitCommit;

/// An element of the `commits` array returned by `gh pr view --json`.
#[derive(serde::Deserialize)]
pub struct Commit {
    pub oid: GitCommit,
}

/// An "author" as returned by Github.
#[derive(Default, serde::Deserialize)]
pub struct Author {
    pub login: String,
}

/// The output of `gh pr view --json`
#[derive(serde::Deserialize)]
pub struct PrInfo {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub author: Author,
    #[serde(default)]
    pub commits: Vec<Commit>,
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
