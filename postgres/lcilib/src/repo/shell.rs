// SPDX-License-Identifier: GPL-3.0-or-later

use core::{fmt, ops};
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use super::Upstream;

#[derive(Debug)]
pub enum RepoShellError {
    CreateShell(xshell::Error),
    GitRevParse(xshell::Error),
    GitRemote {
        err: xshell::Error,
        remote: &'static str,
    },
    UnknownUpstream {
        origin_url: String,
    },
}

impl fmt::Display for RepoShellError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::CreateShell(..) => f.write_str("failed to create xshell shell"),
            Self::GitRevParse(..) => f.write_str("failed to run 'git rev-parse --show-toplevel'"),
            Self::GitRemote { remote, .. } => write!(f, "failed to run 'git remote get-url {remote}'"),
            Self::UnknownUpstream { ref origin_url } => {
                write!(f, "unknown upstream type for origin URL {origin_url}")
            },
        }
    }
}

impl std::error::Error for RepoShellError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            Self::CreateShell(ref e) => Some(e),
            Self::GitRevParse(ref e) => Some(e),
            Self::GitRemote { ref err, .. } => Some(err),
            Self::UnknownUpstream { .. } => None,
        }
    }
}

/// A shell which enforces exclusive access and which is constructed
/// to have its CWD set to the path of a repository.
///
/// All repo operations should go through this shell.
#[derive(Debug)]
pub struct RepoShell {
    inner: Arc<Mutex<xshell::Shell>>,
    upstream: Upstream,
    // The project name in the form 'owner/repo' that the remote API wants.
    project_name: String,

}

/// Attempts to parse a git remote URL and returns `(upstream, "owner/repo")` on success.
///
/// Recognizes GitHub URLs (ssh and https) and Forgejo URLs on `gitea.bitcoin.ninja`. The
/// returned project path uses `/` as its separator.
fn parse_remote_url(url: &str) -> Result<(Upstream, String), RepoShellError> {
    let candidates: &[(&str, fn() -> Upstream)] = &[
        ("git@github.com:", || Upstream::Github),
        ("https://github.com/", || Upstream::Github),
        ("https://www.github.com/", || Upstream::Github),
        ("ssh://git@github.com/", || Upstream::Github),
        ("git@gitea.bitcoin.ninja:", Upstream::gitea_bitcoin_ninja),
        ("https://gitea.bitcoin.ninja/", Upstream::gitea_bitcoin_ninja), 
        ("ssh://git@gitea.bitcoin.ninja/", Upstream::gitea_bitcoin_ninja), 
    ];

    for (prefix, make_upstream) in candidates {
        if let Some(rest) = url.strip_prefix(prefix) {
            let mut repo_part = rest;
            for _ in 0..2 {
                repo_part = repo_part.strip_suffix(".git").unwrap_or(repo_part);
                repo_part = repo_part.strip_suffix("/").unwrap_or(repo_part);
            }
            return Ok((make_upstream(), repo_part.to_string()));
        }
    }

    Err(RepoShellError::UnknownUpstream { origin_url: url.to_owned() })
}

impl RepoShell {
    /// Constructs a new repo shell from the current directory.
    #[inline]
    pub fn new_at_cwd() -> Result<Self, RepoShellError> {
        let shell = xshell::Shell::new().map_err(RepoShellError::CreateShell)?;
        let repo_root = xshell::cmd!(shell, "git rev-parse --show-toplevel")
            .read()
            .map_err(RepoShellError::GitRevParse)?;
        Self::new_inner(shell, repo_root)
    }

    /// Constructs a new repo shell with CWD set to the given path.
    ///
    /// Does not validate that the path exists.
    ///
    /// # Errors
    ///
    /// Errors if we cannot figure out the upstream type of the repository in the given path.
    ///
    /// Also errors if `std::env::current_dir` fails, because
    /// `xshell::Shell::new` calls this, even though
    /// we don't actually need the current directory. :/
    #[inline]
    pub fn new(path: impl AsRef<Path>) -> Result<Self, RepoShellError> {
        let shell = xshell::Shell::new().map_err(RepoShellError::CreateShell)?;
        Self::new_inner(shell, path)
    }

    fn new_inner(shell: xshell::Shell, path: impl AsRef<Path>) -> Result<Self, RepoShellError> {
        shell .change_dir(path);

        let url = match xshell::cmd!(&shell, "git remote get-url origin").read() {
            Ok(url) => url,
            Err(err) => {
                // Try 'upstream' rather than origin, then give up.
                match xshell::cmd!(&shell, "git remote get-url upstream").read() {
                    Ok(url) => url,
                    Err(_) => return Err(RepoShellError::GitRemote { err, remote: "origin" }),
                }
            }
        };

        let (upstream, project_name) = parse_remote_url(url.trim())?;
        Ok(Self {
            inner: Arc::new(Mutex::new(shell)),
            upstream,
            project_name,
        })
    }

    /// Accessor for the repo root directory.
    pub fn repo_root(&self) -> std::path::PathBuf {
        let lock = self.inner.lock().unwrap();
        lock.current_dir()
    }

    /// Accessor for the upstream.
    pub fn upstream(&self) -> &Upstream {
        &self.upstream
    }

    /// Accessor for the upstream.
    pub fn project_name(&self) -> &str {
        &self.project_name
    }

    /// Takes an exclusive lock to the shell and runs the given closure.
    ///
    /// # Async
    ///
    /// This function internally locks a sync mutex before calling the closure,
    /// which it does inside of `async_scoped::spawn_blocking`. Therefore bad things
    /// will happen if the closure tries to do async things, e.g. with `block_on`.
    /// Just don't do it.
    ///
    /// # Errors
    ///
    /// See "panics" section. TBH I don't know whether a panic in the closure will
    /// cause a panic or `JoinError` here because the docs on `scope_and_block`
    /// are lacking.
    ///
    /// # Panics
    ///
    /// Panics if the closure panics, or if any other closure passed to
    /// this function with this lock has panicked.
    /// 
    pub async fn with_lock_blocking<T: Send + 'static>(
        &self,
        op: impl FnOnce(RepoShellLock<'_>) -> T + Send,
    ) -> Result<T, tokio::task::JoinError> {
        let arc = Arc::clone(&self.inner);
        // SAFETY: we immediately await the future and do not drop it.
        // See docs on `scope_and_collect` https://docs.rs/async-scoped/latest/async_scoped/struct.Scope.html#method.scope_and_collect
        // and this Reddit post https://old.reddit.com/r/rust/comments/ee3vsu/asyncscoped_spawn_non_static_futures_with_asyncstd/fbpis3c/
        // for more information -- it appears there is only unsoundness if you really abuse this function.
        unsafe {
            let ((), mut res) = async_scoped::TokioScope::scope_and_collect(|scope| {
                scope.spawn_blocking(|| {
                    let lock = RepoShellLock { inner: arc.lock().unwrap() };
                    op(lock)
                });
            }).await;
            assert_eq!(res.len(), 1, "exactly one future spawned");
            res.pop().unwrap()
        }
    }
}

/// An exclusive lock of a [`RepoShell`].
#[derive(Debug)]
pub struct RepoShellLock<'sh> {
    inner: MutexGuard<'sh, xshell::Shell>
}

impl ops::Deref for RepoShellLock<'_> {
    type Target = xshell::Shell;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
