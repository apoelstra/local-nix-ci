// SPDX-License-Identifier: GPL-3.0-or-later

use core::fmt;

use xshell::cmd;

#[derive(Debug)]
pub enum LoadError {
    GitConfigMissing(&'static str),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GitConfigMissing(k) => write!(f, "missing git config value `{}`", k),
        }
    }
}

impl std::error::Error for LoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::GitConfigMissing(..) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForgejoRepoData {
    token: String,
    username: String,
    https_url: String,
}

fn read_git_config(sh: &xshell::Shell, key: &'static str) -> Option<String> {
    match cmd!(sh, "git config --get {key}").read() {
        Ok(v) => Some(v.trim().to_string()),
        // xshell returns an error on nonzero exit; treat that as "unset".
        Err(_) => None,
    }
}

impl ForgejoRepoData {
    /// Loads Forgejo data from a shell already cd'd into the correct directory.
    ///
    /// The base HTTPS URL is derived from the origin remote by way of [`repo::detect_upstream`],
    /// unless `forgejo.httpsUrl` is set in git config, in which case that value takes precedence.
    pub(crate) fn load_from_shell(
        shell: &xshell::Shell,
        default_https_url: String,
    ) -> Result<Self, LoadError> {
        let token = read_git_config(shell, "forgejo.token")
            .ok_or(LoadError::GitConfigMissing("forgejo.token"))?;
        let username = read_git_config(shell, "forgejo.username")
            .ok_or(LoadError::GitConfigMissing("forgejo.username"))?;
        let override_https_url = read_git_config(shell, "forgejo.httpsUrl");

        let https_url = override_https_url.unwrap_or(default_https_url);
        Ok(Self { token, username, https_url })
    }
}
