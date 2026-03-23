// SPDX-License-Identifier: GPL-3.0-or-later

use core::ops;
use std::sync::{Arc, Mutex, MutexGuard};

/// A shell which enforces exclusive access and which is constructed
/// to have its CWD set to the path of a repository.
///
/// All repo operations should go through this shell.
#[derive(Debug)]
pub struct RepoShell {
    inner: Arc<Mutex<xshell::Shell>>,
}

impl RepoShell {
    /// Constructs a new repo shell with CWD set to the given path.
    ///
    /// Does not validate that the path exists.
    ///
    /// # Errors
    ///
    /// Errors if `std::env::current_dir` fails, because
    /// `xshell::Shell::new` calls this, even though
    /// we don't actually need the current directory. :/
    pub fn new(path: &str) -> Result<Self, xshell::Error> {
        let shell = xshell::Shell::new()?;
        shell .change_dir(path);
        Ok(Self {
            inner: Arc::new(Mutex::new(shell)),
         })
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
