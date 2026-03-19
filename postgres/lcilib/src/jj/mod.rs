// SPDX-License-Identifier: GPL-3.0-or-later

use std::ffi::OsStr;
use std::fmt;
use xshell::{Cmd, Shell, cmd};

#[derive(Debug)]
pub enum Error {
    Shell(xshell::Error),
    ParseOutput(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Shell(_) => f.write_str("failed to invoke jj"),
            Self::ParseOutput(s) => write!(f, "failed to parse output {s}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Shell(e) => Some(e),
            Self::ParseOutput(..) => None,
        }
    }
}

/// A generic jj invocation.
pub fn jj(shell: &Shell) -> Cmd<'_> {
    cmd!(
        shell,
        "jj --config signing.behavior=drop --color never --no-pager --ignore-working-copy"
    )
}

/// Invokes `jj new` on the given shell and returns the created change ID.
///
/// # Errors
///
/// Returns an error if the jj command fails to execute or if the output cannot be parsed
/// to extract the change ID.
pub fn jj_new<P: AsRef<OsStr>>(shell: &Shell, parents: &[P]) -> Result<String, Error> {
    let mut jj = jj(shell).arg("new").arg("--no-edit");
    for p in parents {
        jj = jj.arg("-r").arg(p);
    }

    let jj_new_output = jj.read_stderr().map_err(Error::Shell)?;
    for line in jj_new_output.lines() {
        if line.contains("Created new commit") {
            // Extract change ID from the line - jj change IDs use letters 'k' through 'z'
            if let Some(change_id_match) = line
                .split_whitespace()
                .find(|word| word.len() >= 8 && word.chars().all(|c| ('k'..='z').contains(&c)))
            {
                return Ok(change_id_match.to_string());
            }
        }
    }

    // The above should always succeed. If jj becomes unreliable, we can add a fallback
    // to `jj log -T change_id -r latest(parent1+ & parent2+ & ...)` which will almost
    // certainly work, though it's inherently racy. For now don't bother.
    Err(Error::ParseOutput(jj_new_output))
}

/// Invokes `jj log` with the given template and revset. Returns stdout with whitespace trimmed.
///
/// # Errors
///
/// Returns an error if the jj command fails to execute.
pub fn jj_log<R: AsRef<OsStr>>(shell: &Shell, template: &str, revset: R) -> Result<String, Error> {
    jj(shell)
        .arg("log")
        .arg("--no-graph")
        .arg("-T")
        .arg(template)
        .arg("-r")
        .arg(revset)
        .read()
        .map_err(Error::Shell)
        .map(|s| s.trim().to_string())
}
