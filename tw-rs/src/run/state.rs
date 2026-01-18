// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Context as _;
use std::{env, fs};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use chrono::Utc;
use xshell::{Shell, cmd};

pub struct CiState {
    local_ci_path: PathBuf,
    local_ci_commit_id: String,
    temp_nix_dir: xshell::TempDir,
    last_ci_check: Instant,
}

impl CiState {
    pub fn new() -> anyhow::Result<Self> {
        let sh = Shell::new()?;
        
        // Get LOCAL_CI_PATH or default to git toplevel from binary location
        let local_ci_path = if let Ok(path) = env::var("LOCAL_CI_PATH") {
            PathBuf::from(path)
        } else {
            let binary_dir = env::current_exe()?
                .parent()
                .context("Failed to get binary directory")?
                .to_path_buf();
            
            let _push_dir = sh.push_dir(&binary_dir);
            let git_toplevel = cmd!(sh, "git rev-parse --show-toplevel")
                .read()
                .context("Failed to get git toplevel from binary directory")?;
            PathBuf::from(git_toplevel.trim())
        };

        eprintln!("[{}] Using LOCAL_CI_PATH: {}", 
                 Utc::now().format("%Y-%m-%d %H:%M:%S"), 
                 local_ci_path.display());

        // Get commit ID of CI repo
        let _push_dir = sh.push_dir(&local_ci_path);
        let mut local_ci_commit_id = cmd!(sh, "git rev-parse HEAD")
            .read()
            .context("Failed to get LOCAL_CI commit ID")?
            .trim()
            .to_string();

        // Check if repo is dirty
        let is_dirty = cmd!(sh, "git diff-index --quiet HEAD")
            .quiet()
            .run()
            .is_err();

        if is_dirty {
            eprintln!("[{}] WARNING: LOCAL_CI repo is dirty, appending -dirty to commit tasks", 
                     Utc::now().format("%Y-%m-%d %H:%M:%S"));
            local_ci_commit_id.push_str("-dirty");
        };

        eprintln!("[{}] LOCAL_CI commit ID: {}", 
                 Utc::now().format("%Y-%m-%d %H:%M:%S"), 
                 local_ci_commit_id);

        // Create temp directory and copy *.check-pr.nix files
        let temp_nix_dir = sh.create_temp_dir()?;
        let nix_files = cmd!(sh, "find . -maxdepth 2 -name '*.nix' -type f")
            .read()
            .context("Failed to find .check-pr.nix files")?;

        for nix_file in nix_files.lines() {
            if !nix_file.trim().is_empty() {
                let src = local_ci_path.join(nix_file.trim().trim_start_matches("./"));
                let dst = temp_nix_dir.path().join(nix_file.trim().trim_start_matches("./"));
                
                if let Some(parent) = dst.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(&src, &dst)
                    .with_context(|| format!("Failed to copy {} to temp dir", src.display()))?;
            }
        }

        eprintln!("[{}] Copied .check-pr.nix files to temp directory: {}", 
                 Utc::now().format("%Y-%m-%d %H:%M:%S"), 
                 temp_nix_dir.path().display());

        Ok(CiState {
            local_ci_path,
            local_ci_commit_id,
            temp_nix_dir,
            last_ci_check: Instant::now(),
        })
    }

    pub fn check_ci_repo_status(&mut self) -> anyhow::Result<()> {
        // Only check every 15 minutes
        if self.last_ci_check.elapsed() < Duration::from_secs(15 * 60) {
            return Ok(());
        }
        
        self.last_ci_check = Instant::now();
        
        let sh = Shell::new()?;
        let _push_dir = sh.push_dir(&self.local_ci_path);
        
        let current_commit = cmd!(sh, "git rev-parse HEAD")
            .read()
            .context("Failed to get current LOCAL_CI commit ID")?
            .trim()
            .to_string();

        let (already_dirty, expected_commit) = match self.local_ci_commit_id.strip_suffix("-dirty") {
            Some(stripped) => (true, stripped),
            None => (false, self.local_ci_commit_id.as_str()),
        };

        if current_commit != expected_commit {
            eprintln!("[{}] WARNING: LOCAL_CI commit ID changed from {} to {}. Please restart the program.", 
                     Utc::now().format("%Y-%m-%d %H:%M:%S"), 
                     expected_commit, 
                     current_commit);
        }

        let is_dirty = cmd!(sh, "git diff-index --quiet HEAD")
            .quiet()
            .run()
            .is_err();

        if is_dirty && !already_dirty {
            eprintln!("[{}] WARNING: LOCAL_CI repo became dirty. Please restart the program.", 
                     Utc::now().format("%Y-%m-%d %H:%M:%S"));
        } else if !is_dirty && already_dirty {
            eprintln!("[{}] WARNING: LOCAL_CI repo is no longer dirty but was dirty on startup. Please restart the program.", 
                     Utc::now().format("%Y-%m-%d %H:%M:%S"));
        }

        Ok(())
    }

    pub fn temp_nix_dir(&self) -> &Path {
        self.temp_nix_dir.path()
    }

    pub fn local_ci_commit_id(&self) -> &str {
        &self.local_ci_commit_id
    }
}
