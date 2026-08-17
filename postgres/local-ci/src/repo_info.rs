// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Context as _;
use lcilib::{
    Db,
    db::models::{Ack, CommitToTest, DbRepositoryId, PullRequest, Repository, ReviewStatus, Stack},
    repo,
};
use std::collections::{BTreeMap, BTreeSet};
use xshell::{Shell, cmd};

use crate::daemon::util::calculate_stack_priority;
use crate::terminal::{ColorFormat, Colorable as _};

/// Show overview of all PRs and commits in the current repository
///
/// # Errors
///
/// Returns an error if:
/// - Failed to get current repository information
/// - Database transaction fails
/// - Repository lookup fails
pub async fn overview(db: &mut Db) -> anyhow::Result<()> {
    let repo = repo::current_repo(db)
        .await
        .context("failed to get current repository")?;

    let tx = db
        .transaction()
        .await
        .context("failed to start database transaction")?;

    println!(
        "{}",
        ColorFormat::white(format_args!("Repository: {} ({})", repo.name, repo.path))
    );
    println!("Nixfile: {}", repo.nixfile_path);
    println!("Created: {}", repo.created_at);
    println!();

    // Get all PRs for this repository
    let all_prs = repo
        .id
        .get_current_pull_requests(&tx)
        .await
        .context("failed to get PRs for repository")?;

    // Get all stacks for this repository
    let all_stacks = Stack::get_all(&tx)
        .await
        .context("failed to get stacks for repository")?;

    // Get all commits for this repository
    let mut all_commits = vec![];
    // ...all PR commits
    for pr in &all_prs {
        let commits = pr
            .get_current_non_merge_commits(&tx)
            .await
            .with_context(|| format!("getting commits for PR {}", pr.pr_number))?;
        all_commits.extend(commits);
    }
    // ...all single commits (TODO)
    // ...all commits from stacks
    for stack in &all_stacks {
        let commits = stack
            .id
            .get_commits(&tx)
            .await
            .with_context(|| format!("getting commits for stack {}", stack.id))?;
        all_commits.extend(commits);
    }

    // Get all ACKs for PRs in this repository
    let mut all_acks = Vec::new();
    for pr in &all_prs {
        let pr_acks = Ack::find_by_pull_request(&tx, pr.id)
            .await
            .context("failed to get ACKs for PR")?;
        all_acks.extend(pr_acks);
    }

    show_prs(&all_prs);
    show_daemon_work(&tx).await?;
    show_stacks(&tx, &all_stacks).await?;

    tx.commit().await.context("failed to commit transaction")?;

    Ok(())
}

/// Display PRs organized by status
fn show_prs(prs: &[PullRequest]) {
    println!(
        "{}",
        ColorFormat::white(format_args!("\n=== Pull Requests by Status ==="))
    );

    // Ready to merge
    let ready_to_merge: Vec<_> = prs
        .iter()
        .filter(|pr| pr.review_status == ReviewStatus::Approved && pr.ok_to_merge)
        .collect();

    if !ready_to_merge.is_empty() {
        println!("Ready to Merge ({}):", ready_to_merge.len());
        for pr in ready_to_merge {
            println!(
                "  PR #{}: {} (priority: {})",
                pr.pr_number, pr.title, pr.priority
            );
        }
        println!();
    }

    // .await review
    let needs_review: Vec<_> = prs
        .iter()
        .filter(|pr| pr.review_status == ReviewStatus::Unreviewed)
        .collect();

    if !needs_review.is_empty() {
        println!("Needs Review ({}):", needs_review.len());
        for pr in needs_review {
            println!("  PR #{}: {}", pr.pr_number, pr.title);
        }
        println!();
    }

    // Rejected
    let rejected: Vec<_> = prs
        .iter()
        .filter(|pr| pr.review_status == ReviewStatus::Rejected)
        .collect();

    if !rejected.is_empty() {
        println!("Rejected/Needs Changes ({}):", rejected.len());
        for pr in rejected {
            println!("  PR #{}: {}", pr.pr_number, pr.title);
        }
        println!();
    }

    // Approved but not ready to merge
    let approved_not_ready: Vec<_> = prs
        .iter()
        .filter(|pr| pr.review_status == ReviewStatus::Approved && !pr.ok_to_merge)
        .collect();

    if !approved_not_ready.is_empty() {
        println!(
            "Approved but Not Ready to Merge ({}):",
            approved_not_ready.len()
        );
        for pr in approved_not_ready {
            println!("  PR #{}: {}", pr.pr_number, pr.title);
        }
    }
}

/// Display stacks organized by repository
async fn show_daemon_work(tx: &lcilib::Transaction<'_>) -> anyhow::Result<()> {
    println!(
        "{}",
        ColorFormat::white("\n=== Available Work for Daemon ===\n")
    );

    // This was just copied straight out of `daemon/ci_cyle.rs` `find_next_commit_to_test`
    let standalone_commits = CommitToTest::get_standalone_approved_commits(tx)
        .await
        .context("finding standalone approved commits")?;
    let (high_priority_stacks, low_priority_stacks) = crate::daemon::find_stacks(tx)
        .await
        .context("finding stacks")?;
    let prs_needing_testing = PullRequest::find_needing_testing_prioritized(tx)
        .await
        .context("finding PRs needing testing")?;

    crate::daemon::print_work_summary(
        tx,
        &standalone_commits,
        &high_priority_stacks,
        &prs_needing_testing,
        &low_priority_stacks,
    )
    .await
    .context("printing work summary")?;

    Ok(())
}

/// Display stacks organized by repository
async fn show_stacks(tx: &lcilib::Transaction<'_>, stacks: &[Stack]) -> anyhow::Result<()> {
    /// Key type to allow using repositories as a [`BTreeMap`] key sorted by name.
    struct RepoKey {
        name: String,
        id: DbRepositoryId,
        repo: Repository,
    }
    impl PartialEq for RepoKey {
        fn eq(&self, other: &Self) -> bool {
            self.id == other.id
        }
    }
    impl Eq for RepoKey {}
    impl PartialOrd for RepoKey {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }
    impl Ord for RepoKey {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            (&self.name, self.id.bare_i32()).cmp(&(&other.name, other.id.bare_i32()))
        }
    }

    /// Same but for sorting stacks by reverse priority
    struct StackKey<'s> {
        stack: &'s Stack,
        commits: Vec<CommitToTest>,
        target: String,
        prio: f64,
    }
    impl PartialEq for StackKey<'_> {
        fn eq(&self, other: &Self) -> bool {
            self.stack.id == other.stack.id
        }
    }
    impl Eq for StackKey<'_> {}
    impl PartialOrd for StackKey<'_> {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }
    impl Ord for StackKey<'_> {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            self.target
                .cmp(&other.target)
                .then(self.prio.total_cmp(&other.prio).reverse())
        }
    }

    if stacks.is_empty() {
        println!("\nNo stacks.");
        return Ok(());
    }
    println!(
        "{}",
        ColorFormat::white(format_args!("\n=== Merge Stacks ==="))
    );

    // Group stacks by repository
    let mut stacks_by_repo: BTreeMap<RepoKey, BTreeSet<StackKey<'_>>> = BTreeMap::new();
    for stack in stacks {
        let repo = Repository::get_by_id(tx, stack.repository_id).await?;
        // Sort the stack by priority.
        let commits = stack.id.get_commits(tx).await?;
        let prio = calculate_stack_priority(&commits, tx).await?;
        let target = stack.target_branch.clone();

        // Then insert into a list ordered by name.
        stacks_by_repo
            .entry(RepoKey {
                name: repo.name.clone(),
                id: repo.id,
                repo,
            })
            .or_default()
            .insert(StackKey {
                stack,
                commits,
                target,
                prio,
            });
    }

    for (RepoKey { repo, .. }, repo_stacks) in stacks_by_repo {
        // Display repository heading
        println!(
            "{}",
            ColorFormat::white("\n***** ***** ***** ***** ***** ***** ***** *****")
        );
        println!(
            "{}",
            ColorFormat::white(format_args!("***** {:35} *****", repo.name))
        );
        println!(
            "{}",
            ColorFormat::white("***** ***** ***** ***** ***** ***** ***** *****")
        );

        let mut last_target = None;
        for StackKey {
            stack,
            commits,
            target: _,
            prio,
        } in repo_stacks
        {
            let commit_ids: Vec<_> = commits
                .iter()
                .map(|commit| commit.git_commit_id.as_str())
                .collect();
            let revset = commit_ids.join("|");

            let color = if last_target != Some(&stack.target_branch) {
                ColorFormat::light_green
            } else {
                ColorFormat::very_dull_green
            };
            last_target = Some(&stack.target_branch);

            print!("\n{}", color(format_args!("Stack {}: ", stack.id)));
            println!(
                "prio {:1.3}, target {}, {} commits",
                prio,
                stack.target_branch,
                commits.len()
            );
            for commit in &commits {
                let pr = &commit.prs[0].0;
                let acks = Ack::find_by_pull_request(tx, pr.id)
                    .await
                    .context("failed to find ACKs for PR")?;

                println!(
                    "    PR {} {} ({}): {} (prio {}, by {}, ACKs: {})",
                    ColorFormat::white(pr.pr_number),
                    ColorFormat::white(commit.jj_change_id.prefix8()),
                    commit.git_commit_id.prefix8(),
                    commit.ci_status.with_color(),
                    pr.priority,
                    pr.author_login,
                    acks.into_iter()
                        .map(|a| a.reviewer_name)
                        .collect::<Vec<_>>()
                        .join(", "),
                );
            }
            println!();
            let repo_path = repo.path.clone();
            tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
                let shell = Shell::new()?;
                let _guard = shell.push_dir(&repo_path);

                cmd!(shell, "jj log --no-pager -r {revset}").quiet().run()?;
                Ok(())
            })
            .await??;
        }
    }

    Ok(())
}
