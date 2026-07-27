// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Context as _;
use lcilib::{gh, jj, Db};
use lcilib::db::models::{Repository, Stack};
use std::collections::HashSet;
use std::io::{self, Write as _};
use xshell::cmd;

use crate::args::StackId;

/// Refreshes all the commits in a stack, first offering to push them if they're all signed.
///
/// Takes ownership of the database transaction because 'refresh' commits stuff before posting
/// anything on Github.
///
/// # Errors
///
/// Returns an error if:
/// - Database transaction or operations fail
/// - PR has no commits
pub async fn refresh(
    stack_id: StackId,
    db: &mut Db,
) -> anyhow::Result<()> {
    let tx = db
        .transaction()
        .await
        .context("failed to start database transaction")?;

    let id = stack_id.to_db_stack_id();
    let stack = Stack::find_by_id(&tx, id)
        .await
        .context("finding stack")?;
    let repo = Repository::find_by_id(&tx, stack.repository_id)
        .await
        .context("finding repository for stack")?
        .expect("repo referenced by database should exist");
    let commits = id.get_commits(&tx)
        .await
        .context("getting commits in stack")?;

    // Commit the transaction since we're done with it. Shouldn't possibly fail since
    // we only used it for SELECT queries, but if it does, better to fail here before
    // we do anything heavy.
    tx.commit().await.context("committing read-only tx")?;

    let tip_commit_id = commits.first().map(|commit| &commit.git_commit_id);
    let mut all_signed = true;
    for commit in &commits {
        if !jj::is_commit_gpg_signed(&repo.repo_shell, &commit.jj_change_id)
            .await
            .context("quering gpg-signed status of commit")?
        {
            all_signed = false;
        }
    }

    if let (Some(tip_commit_id), true) = (tip_commit_id, all_signed) {
        loop {
            print!("All commits in stack {} are signed. Do you want to attempt to push them? (y/n) ", stack_id);
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let choice = input.trim().to_ascii_lowercase();
            match choice.as_str() {
                "y" | "yes" => {
                    let target_branch = &stack.target_branch;
                    repo.repo_shell.with_lock_blocking(|shell| {
                        cmd!(shell, "git push origin {tip_commit_id}:{target_branch}")
                            .run()
                            .context("calling git-push")
                    }).await??;
                    break;
                }
                "n" | "no" => break,
                _ => {},
            }
        }
    }

    let mut to_refresh = HashSet::new();
    for commit in commits {
        for (pr, _) in &commit.prs {
            to_refresh.insert(usize::try_from(pr.pr_number).unwrap());
        }
    }

    for pr_number in to_refresh {
        let pr_info = gh::get_pr_info(&repo.repo_shell, pr_number)
            .await
            .context("failed to fetch PR from GitHub")?;
        crate::pr::refresh(&repo, &pr_info, db)
            .await
            .context("refreshing PR")?;
    }

    Ok(())
}
