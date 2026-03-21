// SPDX-License-Identifier: GPL-3.0-or-later

use core::{fmt, str::FromStr};
use std::collections::HashMap;
use std::ffi::OsStr;
use postgres_types::{FromSql, ToSql};
use xshell::{Shell, cmd};

/// Information about a git commit
#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub author: String,
    pub date: String,
    pub message: String,
    pub diffstat: String,
}

/// A representation of a git commit ID.
///
/// When deserialized, validated to be 20 hex digits, but stored
/// as a string to allow efficient use with xshell and postgres.
#[derive(Clone, Default, PartialEq, Eq, Debug, Hash, FromSql, ToSql)]
#[postgres(transparent)]
pub struct CommitId(String);

impl<'de> serde::Deserialize<'de> for CommitId {
    fn deserialize<D: serde::Deserializer<'de>>(des: D) -> Result<Self, D::Error> {
        use bitcoin_hashes::Sha1;

        let sha1 = Sha1::deserialize(des)?;
        Ok(Self(sha1.to_string()))
    }
}

impl FromStr for CommitId {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use bitcoin_hashes::Sha1;

        let s = s.to_owned();
        if let Err(e) = Sha1::from_str(&s) {
            Err(Error::CommitParse(s, e))
        } else {
            Ok(Self(s))
        }
    }
}

impl fmt::Display for CommitId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<OsStr> for CommitId {
    fn as_ref(&self) -> &OsStr {
        self.0.as_ref()
    }
}

impl CommitId {
    /// A string representation of the commit ID
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Populates a map from string prefixes (length 7 and greater) to database commit IDs
    pub fn populate_prefix_map<X: Copy>(&self, map: &mut HashMap<String, X>, target: X) {
        for len in 7..=self.0.len() {
            let prefix = self.0[..len].to_owned();
            map.insert(prefix, target);
        }
    }

    /// Returns the 8-character prefix of the commit ID.
    pub fn prefix8(&self) -> &str {
        &self.0[..8]
    }
}

#[derive(Debug)]
pub enum Error {
    Shell(xshell::Error),
    CommitParse(String, bitcoin_hashes::hex::HexToArrayError),
    CommitNotFound(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Shell(_) => f.write_str("failed to invoke git"),
            Self::CommitParse(s, _) => write!(f, "failed to parse {s} as git commit"),
            Self::CommitNotFound(commit) => write!(
                f,
                "commit {commit} not found after fetching from all remotes"
            ),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Shell(e) => Some(e),
            Self::CommitParse(_, e) => Some(e),
            Self::CommitNotFound(..) => None,
        }
    }
}

/// Returns the list of parent commit IDs for the given commit.
///
/// # Errors
///
/// Returns an error if the git command fails to execute or if any of the parent commit IDs
/// cannot be parsed as valid git commits.
pub fn list_parents<C: AsRef<OsStr>>(shell: &Shell, commit: C) -> Result<Vec<CommitId>, Error> {
    let output = cmd!(shell, "git rev-list --parents -n 1 {commit}")
        .read()
        .map_err(Error::Shell)?;

    output
        .split_whitespace()
        .skip(1) // first element is the commit itself
        .map(CommitId::from_str)
        .collect()
}

/// Resolves a git reference (branch, tag, commit ID, etc.) to a full commit ID.
///
/// # Errors
///
/// Returns an error if the git command fails to execute or if the resolved commit ID
/// cannot be parsed as a valid git commit.
pub fn resolve_ref<R: AsRef<OsStr>>(shell: &Shell, git_ref: R) -> Result<CommitId, Error> {
    let output = cmd!(shell, "git rev-parse {git_ref}")
        .read()
        .map_err(Error::Shell)?;

    CommitId::from_str(output.trim())
}

/// Checks whether a commit is available locally; failing that tries to fetch it from origin;
/// failing that tries to fetch it from upstream; and failing that returns an error.
///
/// # Errors
///
/// Returns an error if the commit cannot be found locally and all fetch attempts from
/// origin and upstream remotes fail.
pub fn fetch_commit<C: AsRef<OsStr>>(shell: &Shell, commit: C) -> Result<(), Error> {
    fn now_have_commit<C: AsRef<OsStr>>(shell: &Shell, commit: C) -> bool {
        cmd!(shell, "git cat-file -e {commit}")
            .quiet()
            .run()
            .is_ok()
    }

    let commit = &commit; // stupid Rust

    // First check if commit is available locally
    if now_have_commit(shell, commit) {
        return Ok(());
    }

    // Then try to fetch it from origin then upstream.
    if cmd!(
        shell,
        "git fetch --force origin +{commit}:refs/heads/local-ci/last-fetch"
    )
    .quiet()
    .ignore_stderr()
    .run()
    .is_ok()
        && now_have_commit(shell, commit)
    {
        let _ = cmd!(shell, "jj git import").quiet().run();
        return Ok(());
    }
    if cmd!(
        shell,
        "git fetch --force upstream +{commit}:refs/heads/local-ci/last-fetch"
    )
    .quiet()
    .ignore_stderr()
    .run()
    .is_ok()
        && now_have_commit(shell, commit)
    {
        let _ = cmd!(shell, "jj git import").quiet().run();
        return Ok(());
    }

    // All attempts failed
    Err(Error::CommitNotFound(
        commit.as_ref().to_string_lossy().to_string(),
    ))
}

/// Always tries to fetch a commit or ref from Github, regardless if we have it locally.
///
/// # Errors
///
/// Returns an error if the fetch operation fails for both origin and upstream remotes,
/// or if the resolved commit ID cannot be parsed as a valid git commit.
pub fn fetch_resolve_ref(shell: &Shell, remote_ref: &str) -> Result<CommitId, Error> {
    cmd!(shell, "git fetch origin {remote_ref}")
        .quiet()
        .ignore_stderr()
        .run()
        .map_err(Error::Shell)
        .and_then(|()| resolve_ref(shell, format!("origin/{remote_ref}")))
        .or_else(|e| {
            // Attempt 'upstream' on error, but failing that just return the error we got for 'origin'
            if cmd!(shell, "git fetch upstream {remote_ref}")
                .quiet()
                .ignore_stdout()
                .run()
                .is_ok()
            && let Ok(r) = resolve_ref(shell, format!("upstream/{remote_ref}")) {
                return Ok(r);
            }
            Err(e)
        })
}

/// Get detailed information about a commit
///
/// # Errors
///
/// Returns an error if the git command fails to execute.
pub fn get_commit_info<C: AsRef<OsStr>>(shell: &Shell, commit: C) -> Result<CommitInfo, Error> {
    // Get author and date
    let author_date = cmd!(shell, "git show --no-patch '--format=%an <%ae>%n%ai' {commit}")
        .read()
        .map_err(Error::Shell)?;
    let mut lines = author_date.lines();
    let author = lines.next().unwrap_or("Unknown").to_string();
    let date = lines.next().unwrap_or("Unknown").to_string();

    // Get commit message
    let message = cmd!(shell, "git show --no-patch --format=%B {commit}")
        .read()
        .map_err(Error::Shell)?
        .trim()
        .to_string();

    // Get diffstat
    let diffstat = cmd!(shell, "git show --stat --format= {commit}")
        .read()
        .map_err(Error::Shell)?
        .trim()
        .to_string();

    Ok(CommitInfo {
        author,
        date,
        message,
        diffstat,
    })
}
