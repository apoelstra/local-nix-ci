// SPDX-License-Identifier: GPL-3.0-or-later

mod change_id;

use crate::git::CommitId;
use std::ffi::OsStr;
use std::fmt;
use xshell::{Cmd, Shell, cmd};

pub use change_id::ChangeId;
pub use change_id::Error as ChangeIdError;

#[derive(Debug)]
pub enum Error {
    Shell(xshell::Error),
    ChangeId(ChangeIdError),
    ParseOutput(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Shell(_) => f.write_str("failed to invoke jj"),
            Self::ChangeId(_) => f.write_str("failed to parse change ID"),
            Self::ParseOutput(s) => write!(f, "failed to parse output {s}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Shell(e) => Some(e),
            Self::ChangeId(e) => Some(e),
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
fn jj_new<P: AsRef<OsStr>>(
    shell: &Shell,
    parents: &[P],
    description: Option<&str>,
) -> Result<ChangeId, Error> {
    let mut jj = jj(shell).arg("new").arg("--no-edit");
    for p in parents {
        jj = jj.arg("-r").arg(p);
    }
    if let Some(desc) = description{
        jj = jj.arg("-m").arg(desc);
    }

    let jj_new_output = jj.read_stderr().map_err(Error::Shell)?;
    for line in jj_new_output.lines() {
        if line.contains("Created new commit") {
            // Extract change ID from the line - jj change IDs use letters 'k' through 'z'
            for word in line.split_whitespace() {
                if let Ok(change_id) = word.parse() {
                    return Ok(change_id);
                }
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
    let mut cmd = jj(shell).arg("log").arg("--no-graph").arg("-r").arg(revset);
    if !template.is_empty() {
        cmd = cmd.arg("-T").arg(template);
    }
    cmd.read()
        .map_err(Error::Shell)
        .map(|s| s.trim().to_string())
}

/// Get the jj change ID for a git commit hash.
///
/// # Errors
///
/// Returns an error if the jj command fails to execute or if the commit is not found.
pub fn get_change_id_for_commit(
    shell: &Shell,
    git_commit_id: &CommitId,
) -> Result<ChangeId, Error> {
    jj_log(shell, "change_id", git_commit_id).and_then(|s| s.parse().map_err(Error::ChangeId))
}

/// Check if a commit is GPG signed using jj
///
/// # Errors
///
/// Returns an error if the jj command fails or if we can't determine the repository path.
pub fn is_commit_gpg_signed(shell: &Shell, change_id: &ChangeId) -> Result<bool, Error> {
    let output = jj_log(shell, "if(signature, \"true\", \"false\")", change_id)?;
    Ok(output.trim() == "true")
}

/// Check if a commit has conflicts using jj
///
/// # Errors
///
/// Returns an error if the jj command fails to execute.
pub fn has_conflicts(shell: &Shell, change_id: &ChangeId) -> Result<bool, Error> {
    let output = jj_log(shell, "if(conflict,\"x\",\"\")", change_id)?;
    Ok(!output.trim().is_empty())
}

/// Create a merge commit using jj
///
/// # Errors
///
/// Returns an error if the jj command fails to execute or if the merge has conflicts.
pub fn create_merge_commit(
    shell: &Shell,
    pr_tip_commit: &str,
    target_branch: &str,
    description: Option<&str>,
) -> Result<ChangeId, Error> {
    // Create new merge commit
    let change_id = jj_new(shell, &[target_branch, pr_tip_commit], description)?;
    // Check for conflicts
    if has_conflicts(shell, &change_id)? {
        return Err(Error::ParseOutput(format!(
            "Merge commit {} has conflicts",
            change_id
        )));
    }

    Ok(change_id)
}

/// Get the current git commit ID for a jj change ID
///
/// # Errors
///
/// Returns an error if the jj command fails to execute.
///
/// # Panics
///
/// Panics if 'jj' outputs something that can't be parsed as a git commit ID.
pub fn get_current_git_commit_for_change_id(
    shell: &Shell,
    change_id: &ChangeId,
) -> Result<CommitId, Error> {
    jj_log(shell, "commit_id", change_id)
        .map(|res| res.parse().expect("jj to output a valid commit ID"))
}

/// Update the description of a commit using jj
///
/// # Errors
///
/// Returns an error if the jj command fails to execute.
pub fn update_commit_description(
    shell: &Shell,
    change_id: &ChangeId,
    description: &str,
) -> Result<(), Error> {
    jj(shell)
        .arg("describe")
        .arg("--quiet")
        .arg("-r")
        .arg(dbg!(change_id))
        .arg("-m")
        .arg(description)
        .ignore_stdout()
        .quiet()
        .run()
        .map_err(Error::Shell)?;

    Ok(())
}
