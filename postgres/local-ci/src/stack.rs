// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Context as _;
use lcilib::{jj, Db};
use lcilib::db::models::{Repository, Stack};
use std::collections::HashSet;
use xshell::cmd;

use crate::args::StackId;
use crate::terminal::ColorFormat;

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
    // This dumb "let mut <var>; loop {}; use <var>" construction would be better written
    // by just recursing in the `if did_something` block below, but we can't recurse in an
    // async function so instead we just loop.
    let mut stack;
    let mut repo;
    let mut commits;
    let mut tip_commit_id;
    let mut all_signed;
    loop {
        let id = stack_id.to_db_stack_id();
        stack = db.with_transaction(async move |tx| Stack::find_by_id(&tx, id).await)
            .await
            .context("finding stack")?;
        crate::daemon::process_stack_updates(db, stack)
            .await
            .context("updating stack state against the database and local repo")?;

        // The update might've deleted the stack, so we have to re-fetch it.
        let tx = db
            .transaction()
            .await
            .context("failed to start database transaction")?;
        stack = Stack::find_by_id(&tx, id)
            .await
            .context("finding stack")?;
        repo = Repository::find_by_id(&tx, stack.repository_id)
            .await
            .context("finding repository for stack")?
            .expect("repo referenced by database should exist");
        commits = id.get_commits(&tx)
            .await
            .context("getting commits in stack")?;

        // Commit the transaction since we're done with it. Shouldn't possibly fail since
        // we only used it for SELECT queries, but if it does, better to fail here before
        // we do anything heavy.
        tx.commit().await.context("committing read-only tx")?;

        tip_commit_id = commits.last().map(|commit| &commit.git_commit_id);
        all_signed = true;
        let mut did_something = false;
        for commit in &commits {
            if !jj::is_commit_gpg_signed(&repo.repo_shell, &commit.jj_change_id)
                .await
                .context("quering gpg-signed status of commit")?
            {
                if let Some((pr, _)) = commit.prs.first() {
                    if crate::ask_yes_no(format_args!(
                        "Merge for {} PR {} is not signed. Invoke check-and-sign.sh on it?",
                        ColorFormat::white(&repo.name), ColorFormat::white(pr.pr_number),
                    )) {
                        let pr_number = pr.pr_number.to_string();
                        let change_id = &commit.jj_change_id;

                        repo.repo_shell.with_lock_blocking(|shell| {
                            cmd!(shell, "check-and-sign.sh {pr_number} {change_id}")
                                .run()
                                .context("calling check-and-sign.sh")
                        }).await??;
                        did_something = true;
                    }
                }
                all_signed = false;
            }
        }

        if did_something {
            crate::daemon::process_stack_updates(db, stack)
                .await
                .context("updating stack state against the database and local repo")?;
            println!();
            println!("Invoked check-and-sign.sh on some commits. The stack may have changed.");
            println!("Re-running refresh command.");
            println!();
        } else {
            break;
        }
    }

    if let (Some(tip_commit_id), true) = (tip_commit_id, all_signed) {
        if crate::ask_yes_no(format_args!(
            "All {} commits in stack {stack_id} (repo {}, target branch {}) are signed.\n\
             Do you want to attempt to push them?",
             ColorFormat::white(commits.len()), ColorFormat::white(&repo.name),
             ColorFormat::white(&stack.target_branch),
        )) {
            let target_branch = &stack.target_branch;
            repo.repo_shell.with_lock_blocking(|shell| {
                cmd!(shell, "git push origin {tip_commit_id}:{target_branch}")
                    .run()
                    .context("calling git-push")
            }).await??;

            // After `git push`ing, Github needs a moment to update the pull requests.
            println!("Sleeping 5 seconds before refreshing pull requests.");
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    }

    let mut to_refresh = HashSet::new();
    for commit in commits {
        for (pr, _) in &commit.prs {
            to_refresh.insert(usize::try_from(pr.pr_number).unwrap());
        }
    }

    for pr_number in to_refresh {
        let pr_info = repo.repo_shell.get_pr_info(pr_number)
            .await
            .context("failed to fetch PR from GitHub")?;
        crate::pr::refresh(&repo, &pr_info, db)
            .await
            .context("refreshing PR")?;
    }

    // After refreshing the PRs, it may be that we can delete or otherwise reduce the stack,
    // since some PRs have been merged.
    crate::daemon::process_stack_updates(db, stack)
        .await
        .context("updating stack state against the database and local repo")?;

    Ok(())
}
