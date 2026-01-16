// SPDX-License-Identifier: GPL-3.0-or-later

use core::{fmt, str::FromStr};
use std::ffi::OsStr;
use xshell::{cmd, Shell};

/// A representation of a git commit.
///
/// When deserialized, validated to be 20 hex digits, but stored
/// as a string to allow efficient use with xshell.
#[derive(Clone, Default, PartialEq, Eq, Debug, Hash)]
pub struct GitCommit(String);

impl<'de> serde::Deserialize<'de> for GitCommit {
    fn deserialize<D: serde::Deserializer<'de>>(des: D) -> Result<Self, D::Error> {
        use bitcoin_hashes::Sha1;

        let sha1 = Sha1::deserialize(des)?;
        Ok(Self(sha1.to_string()))
    }
}

impl FromStr for GitCommit {
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

impl fmt::Display for GitCommit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<OsStr> for GitCommit {
    fn as_ref(&self) -> &OsStr {
        self.0.as_ref()
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
            Self::CommitNotFound(commit) => write!(f, "commit {commit} not found after fetching from all remotes"),
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
pub fn list_parents<C: AsRef<OsStr>>(
    shell: &Shell,
    commit: C,
) -> Result<Vec<GitCommit>, Error> {
    let output = cmd!(shell, "git rev-list --parents -n 1 {commit}")
        .read()
        .map_err(Error::Shell)?;
    
    output
        .trim()
        .split_whitespace()
        .skip(1) // first element is the commit itself
        .map(GitCommit::from_str)
        .collect()
}

/// Checks whether a commit is available locally; failing that tries to fetch it from origin;
/// failing that tries to fetch it from upstream; and failing that returns an error.
pub fn fetch_commit<C: AsRef<OsStr>>(
    shell: &Shell,
    commit: C,
) -> Result<(), Error> {
    fn now_have_commit<C: AsRef<OsStr>>(shell: &Shell, commit: C) -> bool {
        cmd!(shell, "git cat-file -e {commit}").quiet().run().is_ok()
    }

    let commit = &commit; // stupid Rust
    
    // First check if commit is available locally
    if now_have_commit(shell, commit) {
        return Ok(());
    }

    // Then try to fetch it from origin then upstream.    
    if cmd!(shell, "git fetch origin {commit}").run().is_ok() {
        if now_have_commit(shell, commit) {
            return Ok(());
        }
    }
    if cmd!(shell, "git fetch upstream {commit}").run().is_ok() {
        if now_have_commit(shell, commit) {
            return Ok(());
        }
    }
    
    // All attempts failed
    Err(Error::CommitNotFound(
        commit.as_ref().to_string_lossy().to_string()
    ))
}
