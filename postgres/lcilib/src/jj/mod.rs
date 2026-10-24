// SPDX-License-Identifier: GPL-3.0-or-later

mod change_id;

use crate::git::CommitId;
use crate::repo::{RepoShell, RepoShellLock};
use std::ffi::OsStr;
use std::fmt;
use xshell::{Cmd, cmd};

pub use change_id::ChangeId;
pub use change_id::Error as ChangeIdError;

#[derive(Debug)]
pub enum Error {
    Shell(xshell::Error),
    ShellLock(tokio::task::JoinError),
    ChangeId(ChangeIdError),
    ParseOutput(String),
    HasConflicts(ChangeId),
    AlreadyConflicted {
        conflicted_changes: Vec<ChangeId>,
        parents: Vec<String>,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Shell(_) => f.write_str("failed to invoke jj"),
            Self::ShellLock(_) => f.write_str("panic while holding shell lock"),
            Self::ChangeId(_) => f.write_str("failed to parse change ID"),
            Self::ParseOutput(s) => write!(f, "failed to parse output {s}"),
            Self::HasConflicts(s) => write!(f, "newly created change {s} is conflicted"),
            Self::AlreadyConflicted {
                conflicted_changes,
                parents,
            } => {
                write!(
                    f,
                    "conflicted merge(s) already exist for parents {}: {}",
                    parents.join(", "),
                    conflicted_changes
                        .iter()
                        .map(ChangeId::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Shell(e) => Some(e),
            Self::ShellLock(e) => Some(e),
            Self::ChangeId(e) => Some(e),
            Self::ParseOutput(..) => None,
            Self::HasConflicts(..) => None,
            Self::AlreadyConflicted { .. } => None,
        }
    }
}

/// A generic jj invocation.
fn jj<'sh>(shell: &'sh RepoShellLock<'_>) -> Cmd<'sh> {
    cmd!(
        shell,
        "jj --config signing.behavior=drop --no-pager --ignore-working-copy"
    )
}

/// Invokes `jj new` on the given shell and returns the created change ID.
///
/// # Errors
///
/// Returns an error if the jj command fails to execute or if the output cannot be parsed
/// to extract the change ID.
async fn jj_new<P: AsRef<OsStr> + Sync>(
    shell: &RepoShell,
    parents: &[P],
    description: Option<&str>,
) -> Result<ChangeId, Error> {
    let jj_new_output = shell
        .with_lock_blocking(|shell| {
            let mut jj = jj(&shell)
                .arg("new")
                .arg("--color")
                .arg("never")
                .arg("--no-edit");
            for p in parents {
                jj = jj.arg("-r").arg(p);
            }
            if let Some(desc) = description {
                jj = jj.arg("-m").arg(desc);
            }

            jj.read_stderr().map_err(Error::Shell)
        })
        .await
        .map_err(Error::ShellLock)??;

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
pub async fn jj_log<R, S>(
    shell: &RepoShell,
    template: Option<S>,
    revset: R,
) -> Result<String, Error>
where
    R: AsRef<OsStr> + Send + Sync,
    S: AsRef<OsStr> + Send + Sync,
{
    let stdout = shell
        .with_lock_blocking(|shell| {
            let mut cmd = jj(&shell).arg("log").arg("-r").arg(revset);
            if let Some(template) = template {
                cmd = cmd
                    .arg("--color")
                    .arg("never")
                    .arg("--no-graph")
                    .arg("-T")
                    .arg(template);
            }
            cmd.read().map_err(Error::Shell)
        })
        .await
        .map_err(Error::ShellLock)??;

    Ok(stdout.trim().to_string())
}

/// Get the jj change ID for a git commit hash.
///
/// # Errors
///
/// Returns an error if the jj command fails to execute or if the commit is not found.
pub async fn get_change_id_for_commit(
    shell: &RepoShell,
    git_commit_id: &CommitId,
) -> Result<ChangeId, Error> {
    jj_log(shell, Some("change_id"), git_commit_id)
        .await
        .and_then(|s| s.parse().map_err(Error::ChangeId))
}

/// Check if a commit is GPG signed using jj
///
/// # Errors
///
/// Returns an error if the jj command fails or if we can't determine the repository path.
pub async fn is_commit_gpg_signed(shell: &RepoShell, change_id: &ChangeId) -> Result<bool, Error> {
    let output = jj_log(shell, Some("if(signature, \"true\", \"false\")"), change_id).await?;
    Ok(output.trim() == "true")
}

/// Batch-check whether each of the given change IDs is GPG signed.
///
/// # Returns
///
/// Returns a hashset of all the change IDs that -were- signed, of the ones
/// passed in.
///
/// # Errors
///
/// Returns an error if the jj command fails or if jj's output cannot be
/// parsed as change IDs.
pub async fn count_gpg_signed_commits(
    shell: &RepoShell,
    change_ids: &[ChangeId],
) -> Result<usize, Error> {
    if change_ids.is_empty() {
        return Ok(0)
    }
    // Build a revset "c1|c2|..." selecting exactly our change IDs. Rust makes us construct
    // a Vec to use join <.<.
    let cid_strs = change_ids.iter().map(ChangeId::to_string).collect::<Vec<_>>();
    let revset = format!("({}) & signed()", cid_strs.join("|"));
    let output = jj_log(shell, Some("change_id"), &revset).await?;

    let mut count = 0;
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // parse as a sanity check that we're not getting garbage from jj
        line.parse::<ChangeId>().map_err(Error::ChangeId)?;
        count += 1;
    }
    Ok(count)
}

/// Check if a commit has conflicts using jj
///
/// # Errors
///
/// Returns an error if the jj command fails to execute.
pub async fn has_conflicts(shell: &RepoShell, change_id: &ChangeId) -> Result<bool, Error> {
    let output = jj_log(shell, Some("if(conflict,\"x\",\"\")"), change_id).await?;
    Ok(!output.trim().is_empty())
}

/// Check for existing conflicted merges of the given parents.
///
/// Only merges whose number of parents exactly matches `parents.len()` are returned,
/// so merges with additional parents beyond the given set are ignored.
///
/// # Errors
///
/// Returns an error if the jj command fails to execute.
async fn check_existing_conflicted_merges(
    shell: &RepoShell,
    parents: &[&str],
) -> Result<Vec<ChangeId>, Error> {
    assert!(
        !parents.is_empty(),
        "should not have a merge without parents"
    );

    // Look for all commits which are children of the given parents.
    let revset = format!("({}+) & conflicts()", parents.join(")+ & ("));
    let template = "change_id.short() ++ \" \" ++ parents.len() ++ \"\\n\"";
    let output = jj_log(shell, Some(template), &revset).await?;
    if (output.trim()).is_empty() {
        return Ok(Vec::new());
    }

    // ...but there may be changes (e.g. the user's megamerges) which are children of the
    // given parents but also children of other parents. These might be conflicted, but
    // since local-ci didn't create them, we should ignore them.
    let expected_parent_count = parents.len();
    let mut conflicted_changes = Vec::new();
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some((change_id_str, parent_count_str)) = line.split_once(' ') else {
            eprintln!("[warning] failed to parse jj log line {}", line);
            continue;
        };
        let Ok(parent_count) = parent_count_str.parse::<usize>() else {
            eprintln!("[warning] failed to parse parent count in {}", line);
            continue;
        };
        if parent_count != expected_parent_count {
            continue;
        }
        if let Ok(change_id) = change_id_str.parse() {
            conflicted_changes.push(change_id);
        } else {
            eprintln!("[warning] failed to parse {} as change ID", change_id_str);
        }
    }

    Ok(conflicted_changes)
}

/// Create a merge commit using jj
///
/// # Errors
///
/// Returns an error if the jj command fails to execute, if existing conflicted merges exist,
/// or if the merge has conflicts.
pub async fn create_merge_commit(
    shell: &RepoShell,
    pr_tip_commit: &str,
    target_branch: &str,
    description: Option<&str>,
) -> Result<ChangeId, Error> {
    // Check for existing conflicted merges
    let parents = [target_branch, pr_tip_commit];
    let conflicted_changes = check_existing_conflicted_merges(shell, &parents).await?;

    if !conflicted_changes.is_empty() {
        return Err(Error::AlreadyConflicted {
            conflicted_changes,
            parents: parents.iter().map(<&str>::to_string).collect(),
        });
    }

    // Create new merge commit
    let change_id = jj_new(shell, &parents, description).await?;
    // Check for conflicts
    if has_conflicts(shell, &change_id).await? {
        return Err(Error::HasConflicts(change_id));
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
pub async fn get_current_git_commit_for_change_id(
    shell: &RepoShell,
    change_id: &ChangeId,
) -> Result<CommitId, Error> {
    jj_log(shell, Some("commit_id"), change_id)
        .await
        .map(|res| res.parse().expect("jj to output a valid commit ID"))
}

/// Update the description of a commit using jj.
///
/// Does nothing if the commit already has exactly this description, modulo
/// trailing newlines (which jj normalizes to a single one anyway): `jj
/// describe` rewrites the commit even when the text is unchanged, which
/// changes its commit ID and drops any signature, leaving the commit ID
/// recorded in the database stale for no reason.
///
/// The read-compare-write sequence runs under the shell's repository lock, so
/// concurrent callers cannot undo each other's skips.
///
/// # Errors
///
/// Returns an error if the jj command fails to execute.
pub async fn update_commit_description(
    shell: &RepoShell,
    change_id: &ChangeId,
    description: &str,
) -> Result<(), Error> {
    shell
        .with_lock_blocking(|shell| {
            let current_description = jj(&shell)
                .arg("log")
                .arg("-r")
                .arg(change_id)
                .arg("--no-graph")
                .arg("--color")
                .arg("never")
                .arg("-T")
                .arg("description")
                .read()
                .map_err(Error::Shell)?;
            if current_description.trim_end_matches('\n') == description.trim_end_matches('\n') {
                return Ok(());
            }

            jj(&shell)
                .arg("describe")
                .arg("--quiet")
                .arg("-r")
                .arg(change_id)
                .arg("-m")
                .arg(description)
                .ignore_stdout()
                .quiet()
                .run()
                .map_err(Error::Shell)
        })
        .await
        .map_err(Error::ShellLock)??;

    Ok(())
}

/// A git+jj colocated repository, whose origin URL is resolvable enough
/// for `RepoShell::new`. `JJ_CONFIG` is set on the shell so the tests are
/// hermetic with respect to host jj/git identity configuration.
#[cfg(test)]
pub(super) fn test_repo() -> (tempfile::TempDir, RepoShell) {
    let dir = tempfile::tempdir().expect("failed to create temporary directory");
    let dir_path = dir.path().to_owned();
    let shell = xshell::Shell::new().expect("failed to create shell");

    let config_path = dir_path.join("jj-config.toml");
    std::fs::write(
        &config_path,
        "[user]\nname = \"local-ci-test\"\nemail = \"test@example.com\"\n",
    )
    .expect("failed to write jj config");
    shell.set_var("JJ_CONFIG", &config_path);

    cmd!(shell, "git init --initial-branch=main {dir_path}")
        .quiet()
        .run()
        .expect("git init failed");
    cmd!(shell, "git -C {dir_path} remote add origin https://github.com/example/example.git")
        .quiet()
        .run()
        .expect("git remote add failed");
    cmd!(shell, "git -C {dir_path} -c user.name=local-ci-test -c user.email=test@example.com commit --allow-empty -m initial")
        .quiet()
        .run()
        .expect("git commit failed");
    cmd!(shell, "jj git init --colocate {dir_path}")
        .quiet()
        .run()
        .expect("jj init failed");

    let repo_shell = RepoShell::new(&dir_path).expect("failed to create RepoShell");
    (dir, repo_shell)
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::future::Future;
    use xshell::cmd;

    /// Runs `f` on a dedicated thread with its own current-thread tokio runtime
    /// and joins it, panicking on the test thread if the task panicked. Used
    /// instead of `#[tokio::test]` because lcilib's tokio features do not
    /// include the macros for it.
    fn run_task<F>(f: impl FnOnce() -> F + Send + 'static) -> F::Output
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .build()
                .expect("failed to build tokio runtime");
            rt.block_on(f())
        })
        .join()
        .unwrap_or_else(|e| std::panic::resume_unwind(e))
    }

    /// If the commit already carries the wanted description, the update must
    /// be skipped entirely. An actual `jj describe` invocation errors out on
    /// an immutable commit even when the description is identical, so reaching
    /// an `Ok` result proves the skip path was taken -- while a changed
    /// description must still be written, which the immutable commit then
    /// refuses.
    #[test]
    fn update_commit_description_skips_unchanged() {
        let (_dir, shell) = test_repo();

        run_task(move || async move {
            let change_id: ChangeId = jj_log(&shell, Some("change_id"), "@")
                .await
                .expect("jj log failed")
                .parse()
                .expect("valid change id");

            update_commit_description(&shell, &change_id, "stack merge description")
                .await
                .expect("initial describe failed");

            // Make the working-copy commit immutable: 'jj describe' on it now
            // fails even if the description is unchanged.
            shell
                .with_lock_blocking(|sh| {
                    let key = r#"revset-aliases."immutable_heads()""#;
                    let value = r#""@ | remote_bookmarks()""#;
                    cmd!(sh, "jj config set --repo {key} {value}").quiet().run()
                })
                .await
                .expect("with_lock_blocking failed")
                .expect("config set failed");

            update_commit_description(&shell, &change_id, "stack merge description")
                .await
                .expect("identical description must be skipped, not re-described");

            let err = update_commit_description(&shell, &change_id, "different description")
                .await
                .expect_err("different description must actually run 'jj describe'");
            assert!(
                matches!(err, Error::Shell(_)),
                "expected 'jj describe' failure, got {err:?}"
            );
        });
    }

    /// Descriptions are compared modulo trailing newlines (jj stores exactly
    /// one, or none for the empty description), so an update differing only in
    /// trailing newlines must not rewrite the commit.
    #[test]
    fn update_commit_description_ignores_trailing_newlines() {
        let (_dir, shell) = test_repo();

        run_task(move || async move {
            let change_id: ChangeId = jj_log(&shell, Some("change_id"), "@")
                .await
                .expect("jj log failed")
                .parse()
                .expect("valid change id");

            update_commit_description(&shell, &change_id, "some description")
                .await
                .expect("initial describe failed");
            let commit_id =
                get_current_git_commit_for_change_id(&shell, &change_id).await.expect("jj log failed");

            for description in ["some description\n", "some description\n\n"] {
                update_commit_description(&shell, &change_id, description)
                    .await
                    .unwrap_or_else(|e| panic!("update with {description:?} should be skipped: {e:?}"));
                assert_eq!(
                    get_current_git_commit_for_change_id(&shell, &change_id)
                        .await
                        .expect("jj log failed"),
                    commit_id,
                    "description differing only in trailing newlines must not rewrite the commit",
                );
            }

            update_commit_description(&shell, &change_id, "some other description")
                .await
                .expect("describe with changed text failed");
            assert_eq!(
                jj_log(&shell, Some("description"), &change_id)
                    .await
                    .expect("jj log failed"),
                "some other description",
            );
        });
    }
}
