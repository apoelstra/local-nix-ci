// SPDX-License-Identifier: GPL-3.0-or-later

use core::ops;

/// A shell which enforces exclusive access and which is constructed
/// to have its CWD set to the path of a repository.
///
/// All repo operations should go through this shell.
#[derive(Debug)]
pub struct RepoShell {
    inner: tokio::sync::Mutex<xshell::Shell>,
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
            inner: tokio::sync::Mutex::new(shell),
         })
    }

    pub async fn lock(&self) -> RepoShellLock<'_> {
        RepoShellLock {
            inner: self.inner.lock().await
        }
    }
}

/// An exclusive lock of a [`RepoShell`].
#[derive(Debug)]
pub struct RepoShellLock<'sh> {
    inner: tokio::sync::MutexGuard<'sh, xshell::Shell>
}

impl ops::Deref for RepoShellLock<'_> {
    type Target = xshell::Shell;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
