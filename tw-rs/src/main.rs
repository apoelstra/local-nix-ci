// SPDX-License-Identifier: GPL-3.0-or-later

mod args;
mod gh;
mod git;
mod jj;
mod repo;
mod run;
mod tw;

use anyhow::Context;
use core::fmt;
use std::io::{self, Write};
use xshell::{Shell, cmd};

use self::args::{Action, Target};
use self::git::GitCommit;
use self::tw::CommitTask;
use self::tw::serde_types::{CiStatus, ReviewStatus};

fn check_required_tools() -> xshell::Result<()> {
    let sh = Shell::new()?;
    let tools = ["gh", "git", "jj", "nix", "task"];

    for tool in &tools {
        if cmd!(sh, "{tool} --version")
            .quiet()
            .ignore_stdout()
            .run()
            .is_err()
        {
            eprintln!(
                "Error: Required tool '{}' is not available or not in PATH",
                tool
            );
            std::process::exit(1);
        }
    }

    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Next {
    ReviewPr(usize),
    ReviewCommit(GitCommit),
    Failed,
    NothingToDo,
}

impl fmt::Display for Next {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReviewPr(num) => write!(f, "local-ci pr {num} review"),
            Self::ReviewCommit(commit) => write!(f, "local-ci commit {commit} review"),
            Self::Failed => f.write_str("Nothing to do (failed)."),
            Self::NothingToDo => f.write_str("Nothing to do."),
        }
    }
}

impl Next {
    fn update_from_commit(&mut self, commit: &CommitTask) {
        match commit.review_status() {
            ReviewStatus::Unreviewed => {
                self.update(Next::ReviewCommit(commit.commit_id().clone()));
            }
            ReviewStatus::NeedsChange | ReviewStatus::Nacked => {
                self.update(Next::Failed);
            }
            ReviewStatus::Approved => {
                if *commit.ci_status() == CiStatus::Failed {
                    self.update(Next::Failed);
                }
            }
        }
    }

    fn update(&mut self, new: Self) {
        if *self != Self::Failed {
            *self = new;
        }
    }

    fn do_it(&self, shell: &Shell, tasks: &mut tw::TaskCollection) -> Result<(), anyhow::Error> {
        match self {
            Self::ReviewPr(num) => real_main(shell, tasks, Action::Review, Target::Pr(*num)),
            Self::ReviewCommit(commit) => real_main(
                shell,
                tasks,
                Action::Review,
                Target::Commit(commit.to_string()),
            ),
            Self::NothingToDo => {
                eprintln!("Nothing to do.");
                Ok(())
            }
            Self::Failed => {
                eprintln!("Nothing to do (PR or commit is marked failed).");
                Ok(())
            }
        }
    }
}

fn real_main(
    shell: &Shell,
    tasks: &mut tw::TaskCollection,
    action: Action,
    target: Target,
) -> Result<(), anyhow::Error> {
    if target == Target::None {
        // Handle "global" actions, which don't need to be in an active repo
        // and which can avoid invoking gh (except refresh, which does on
        // purpose).
        match action {
            Action::Review => {
                eprintln!("Nothing to review. (Did you mean to provide a PR number or commit ID?");
                eprintln!();
                args::usage();
                std::process::exit(1);
            }
            Action::TaskEdit | Action::TaskInfo => {
                eprintln!(
                    "Nothing to query taskwarrior for. (Did you mean to provide a PR number or commit ID?"
                );
                eprintln!();
                args::usage();
                std::process::exit(1);
            }
            Action::Info => {
                for (_, pull) in tasks.pulls() {
                    println!("{} PR #{}: {}", pull.project(), pull.number(), pull.title());
                }
            }
            Action::Next => {
                eprintln!("No 'next' action without a PR number or commit ID.");
                eprintln!();
                args::usage();
                std::process::exit(1);
            }
            Action::Refresh => {
                eprintln!("[invoking gh here]");
            }
            Action::Run => run::run(tasks)?,
        }
        return Ok(());
    } else {
        // Error out tfor actions which don't have any target.
        if action == Action::Run {
            eprintln!("'run' cannot be invoked with any target.");
            eprintln!();
            args::usage();
            std::process::exit(1);
        }
    }

    // Orient ourselves. (In practice this is the slowest step but unfortunately
    // there is no real way to cache it.)
    let repo = match repo::current_repo() {
        Err(e) => {
            eprintln!("Failed to locate repo: {e}");
            eprintln!("Are you in a Github-connected git repository (that 'gh' understands)?");
            std::process::exit(1);
        }
        Ok(repo) => repo,
    };

    println!("In repository: {}", repo.project_name);
    println!("Repo root: {}", repo.repo_root.display());
    println!();

    // Look up PR.
    match target {
        Target::Pr(num) => {
            let lookup = tasks.pull_by_number(&repo.project_name, num);
            let just_created = lookup.is_none();
            let pull = match lookup {
                Some(task) => task,
                None => tasks
                    .insert_or_refresh_pr(shell, &repo, num)
                    .context("adding new PR")?,
            };
            // Clone the pull to end the mutable borrow of `tasks`.
            let pull = pull.clone();

            match action {
                Action::Info | Action::Next => {
                    let mut next = Next::NothingToDo;

                    println!("=== PR #{}: {} ===", pull.number(), pull.title());
                    println!("Author: {}", pull.author());
                    println!("PR review status: {}", pull.review_status());
                    println!("PR merge status: {}", pull.merge_status());
                    println!();
                    let merge_commit = pull.merge_commit(tasks);
                    next.update_from_commit(merge_commit);
                    print!(
                        "PR merge commit: {} (review: {}",
                        merge_commit.commit_id(),
                        merge_commit.review_status()
                    );
                    if matches!(merge_commit.review_status(), ReviewStatus::Approved) {
                        print!(", ci: {}", merge_commit.ci_status());
                    }
                    if merge_commit.is_merge_commit() {
                        if merge_commit.is_clean_merge() {
                            print!(", CLEAN_MERGE");
                        } else {
                            print!(", MERGE");
                        }
                    }
                    println!(")");

                    if !pull.merge_change_id().is_empty() {
                        println!("  JJ change ID: {}", pull.merge_change_id());
                    }
                    println!("  (Review the merge commit like any other commit to trigger CI)");
                    println!();

                    match pull.review_status() {
                        ReviewStatus::Unreviewed => {
                            next.update(Next::ReviewPr(num));
                        }
                        ReviewStatus::NeedsChange | ReviewStatus::Nacked => {
                            next.update(Next::Failed);
                        }
                        ReviewStatus::Approved => { /* done */ }
                    }

                    println!("=== Commits ===");
                    let commits: Vec<_> = pull.commits(tasks).collect();

                    for commit in &commits {
                        next.update_from_commit(commit);

                        print!(
                            "  {} (review: {}",
                            commit.commit_id(),
                            commit.review_status()
                        );
                        if matches!(commit.review_status(), ReviewStatus::Approved) {
                            print!(", ci: {}", commit.ci_status());
                        }

                        if commit.is_tip() {
                            print!(", TIP");
                        }

                        if commit.is_merge_commit() {
                            if commit.is_clean_merge() {
                                print!(", CLEAN_MERGE");
                            } else {
                                print!(", MERGE");
                            }
                        }

                        println!(")");
                    }

                    println!();
                    println!("Commit graph:");
                    let mut revset_str =
                        format!("{} | {}", pull.base_commit(), merge_commit.commit_id());
                    for commit in &commits {
                        revset_str.push_str(&format!(" | {}", commit.commit_id()));
                    }
                    let _ = cmd!(
                        shell,
                        "jj log --no-pager --ignore-working-copy -r {revset_str}"
                    )
                    .quiet()
                    .run();
                    println!();
                    println!("Next action: {next}");

                    if action == Action::Next {
                        next.do_it(shell, tasks)?;
                    }
                }
                Action::Refresh => {
                    if !just_created {
                        tasks
                            .insert_or_refresh_pr(shell, &repo, num)
                            .context("refreshing PR")?;
                    }
                }
                Action::Review => {
                    review_pr_interactive(shell, tasks, &repo, &pull)?;
                }
                Action::Run => unreachable!("checked above"),
                Action::TaskEdit => {
                    let uuid = pull.uuid().to_string();
                    cmd!(shell, "task edit {uuid}")
                        .run()
                        .context("executing task edit")?;
                }
                Action::TaskInfo => {
                    let uuid = pull.uuid().to_string();
                    cmd!(shell, "task info {uuid}")
                        .run()
                        .context("executing task info")?;
                }
            }
        }
        Target::Commit(commit_str) => {
            // Resolve the git ref to a commit ID
            let commit_id = git::resolve_ref(shell, &commit_str).context("resolving git commit")?;

            let lookup = tasks.commit_by_id(&repo.project_name, &commit_id);
            let just_created = lookup.is_none();
            let commit = match lookup {
                Some(task) => task,
                None => tasks
                    .insert_or_refresh_commit(
                        shell,
                        &repo.project_name,
                        &repo.repo_root,
                        &commit_id,
                    )
                    .context("adding new commit")?,
            };

            match action {
                Action::Info | Action::Next => {
                    println!("{}: {}", commit.project(), commit.description());

                    let mut next = Next::NothingToDo;
                    next.update_from_commit(commit);
                    println!();
                    println!("Next action: {next}");

                    if action == Action::Next {
                        next.do_it(shell, tasks)?;
                    }
                }
                Action::Refresh => {
                    if !just_created {
                        tasks
                            .insert_or_refresh_commit(
                                shell,
                                &repo.project_name,
                                &repo.repo_root,
                                &commit_id,
                            )
                            .context("refreshing commit")?;
                    }
                }
                Action::Review => {
                    review_commit_interactive(shell, tasks, &repo, &commit_id)?;
                }
                Action::Run => unreachable!("checked above"),
                Action::TaskEdit => {
                    let uuid = commit.uuid().to_string();
                    cmd!(shell, "task edit {uuid}")
                        .run()
                        .context("executing task edit")?;
                }
                Action::TaskInfo => {
                    let uuid = commit.uuid().to_string();
                    cmd!(shell, "task info {uuid}")
                        .run()
                        .context("executing task info")?;
                }
            }
        }
        Target::None => unreachable!("this case handled above, before current_repo()"),
    }

    Ok(())
}

fn review_commit_interactive(
    shell: &Shell,
    tasks: &mut tw::TaskCollection,
    repo: &repo::Repository,
    commit_id: &GitCommit,
) -> Result<(), anyhow::Error> {
    let commit_task = tasks
        .commit_by_id(&repo.project_name, commit_id)
        .context("commit not found in task collection")?;

    loop {
        println!();
        println!("=== Reviewing commit {} ===", commit_id);
        println!("Current review status: {}", commit_task.review_status());
        println!();

        // Show git diff
        println!("--- Git diff ---");
        let git_show_result = cmd!(shell, "git show {commit_id}").run();
        if let Err(e) = git_show_result {
            eprintln!("Warning: Failed to show git diff: {}", e);
        }
        println!();

        // Show PRs containing this commit
        println!("--- PRs containing this commit ---");
        let mut found_prs = false;
        for (_, pr_task) in tasks.pulls() {
            let contains_commit = pr_task.commits(tasks).any(|c| c.commit_id() == commit_id);

            if contains_commit {
                found_prs = true;
                println!(
                    "  PR #{}: {} (by {})",
                    pr_task.number(),
                    pr_task.title(),
                    pr_task.author()
                );

                if commit_task.is_tip() {
                    println!("  ⚠️  This is a tip commit for at least one of the above PR(s)");
                }
                if commit_task.is_merge_commit() {
                    println!("  ⚠️  This is a merge commit for at least one of the above PR(s)");
                }
            }
        }

        if !found_prs {
            println!("  No PRs found containing this commit.");
        }
        println!();
        println!("  Note: Remember to review the PR(s) separately from individual commits.");
        println!();

        // Prompt for action
        println!("What would you like to do?");
        println!("1) Approve");
        println!("2) NACK");
        println!("3) Needs Change");
        println!("4) Erase review (mark unreviewed)");
        println!("5) Re-view diff");
        println!("6) Cancel");
        print!("Choice (1-6): ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let choice = input.trim();

        let new_status = match choice {
            "1" => Some(ReviewStatus::Approved),
            "2" => Some(ReviewStatus::Nacked),
            "3" => Some(ReviewStatus::NeedsChange),
            "4" => Some(ReviewStatus::Unreviewed),
            "5" => {
                // Continue loop to re-show diff
                continue;
            }
            "6" => {
                println!("Review cancelled.");
                break;
            }
            _ => {
                println!("Invalid choice. Please select 1-6.");
                continue;
            }
        };

        if let Some(status) = new_status {
            // Get review notes from user
            let review_notes = if status == ReviewStatus::Unreviewed {
                String::new()
            } else {
                get_review_notes_from_editor(shell, commit_task, &status)?
            };

            // Update the task
            let uuid = commit_task.uuid().to_string();
            let status_str = match status {
                ReviewStatus::Approved => "approved",
                ReviewStatus::Nacked => "nacked",
                ReviewStatus::NeedsChange => "needschange",
                ReviewStatus::Unreviewed => "unreviewed",
            };

            cmd!(
                shell,
                "task {uuid} modify review_status:{status_str} review_notes:{review_notes}"
            )
            .run()
            .context("updating task review status")?;

            println!("Commit {} review status updated to: {}", commit_id, status);
            if !review_notes.is_empty() {
                println!("Review notes saved.");
            }

            break;
        }
    }

    Ok(())
}

fn review_pr_interactive(
    shell: &Shell,
    tasks: &mut tw::TaskCollection,
    repo: &repo::Repository,
    pull: &tw::PrTask,
) -> Result<(), anyhow::Error> {
    loop {
        println!();
        println!("=== Reviewing PR #{}: {} ===", pull.number(), pull.title());
        println!("Author: {}", pull.author());
        println!("Current PR Review Status: {}", pull.review_status());
        println!();

        // Show commit status summary
        println!("=== Commit Status Summary ===");
        let mut all_commits_approved = true;
        let mut all_commits_ci_success = true;
        let mut tip_commit_id = None;

        let commits: Vec<_> = pull.commits(tasks).collect();
        for commit in &commits {
            println!(
                "  {}: review={}, ci={}{}",
                commit.commit_id(),
                commit.review_status(),
                commit.ci_status(),
                if commit.is_tip() { " (TIP)" } else { "" }
            );

            if commit.is_tip() {
                tip_commit_id = Some(commit.commit_id().clone());
            }

            if *commit.review_status() != ReviewStatus::Approved {
                all_commits_approved = false;
            }
            if *commit.ci_status() != CiStatus::Success {
                all_commits_ci_success = false;
            }
        }

        println!();
        println!("All commits approved: {}", all_commits_approved);
        println!("All commits CI success: {}", all_commits_ci_success);
        println!();

        // Prompt for action
        println!("What would you like to do?");
        println!("1) Approve PR");
        println!("2) NACK PR");
        println!("3) Request changes");
        println!("4) View total diff");
        println!("5) Cancel");
        print!("Choice (1-5): ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let choice = input.trim();

        let new_status = match choice {
            "1" => Some(ReviewStatus::Approved),
            "2" => Some(ReviewStatus::Nacked),
            "3" => Some(ReviewStatus::NeedsChange),
            "4" => {
                println!("--- Total diff for PR #{} ---", pull.number());
                let _push_dir = shell.push_dir(&repo.repo_root);

                // Get base commit and tip commit for the diff
                let base_commit = pull.base_commit();
                if let Some(tip_commit) = tip_commit_id.as_ref() {
                    let _ = cmd!(shell, "git diff {base_commit}..{tip_commit}").run();
                } else {
                    eprintln!("Warning: No tip commit found, cannot show diff");
                }
                continue;
            }
            "5" => {
                println!("Review cancelled.");
                break;
            }
            _ => {
                println!("Invalid choice. Please select 1-5.");
                continue;
            }
        };

        if let Some(status) = new_status {
            // Get tip commit for the approval message
            let tip_commit = match tip_commit_id {
                Some(ref commit_id) => commit_id,
                None => {
                    eprintln!(
                        "Warning: PR appears to have no tip commit set; please manually fix this. Cannot review."
                    );
                    break;
                }
            };

            // Get review notes from user
            let review_notes =
                get_pr_review_notes_from_editor(shell, pull, &status, tip_commit, &commits)?;

            // Update the task
            let uuid = pull.uuid().to_string();
            let status_str = match status {
                ReviewStatus::Approved => "approved",
                ReviewStatus::Nacked => "nacked",
                ReviewStatus::NeedsChange => "needschange",
                ReviewStatus::Unreviewed => "unreviewed",
            };

            cmd!(
                shell,
                "task {uuid} modify review_status:{status_str} review_notes:{review_notes}"
            )
            .run()
            .context("updating PR review status")?;

            println!("PR #{} review status updated to: {}", pull.number(), status);
            if !review_notes.is_empty() {
                println!("Review notes saved.");
            }

            // If approved, check if we should post GitHub approval and handle merge commit
            if status == ReviewStatus::Approved {
                post_github_approval_if_ready(shell, tasks, repo, pull)?;

                // Handle merge commit auto-approval if it's clean
                let merge_commit = pull.merge_commit(tasks);
                if merge_commit.is_clean_merge()
                    && *merge_commit.review_status() == ReviewStatus::Unreviewed
                {
                    println!("Merge commit is clean, automatically approving it...");
                    let merge_uuid = merge_commit.uuid().to_string();
                    let auto_review_notes =
                        format!("Auto-approved clean merge commit for PR #{}", pull.number());
                    cmd!(shell, "task {merge_uuid} modify review_status:approved review_notes:{auto_review_notes}")
                        .run()
                        .context("auto-approving clean merge commit")?;
                    println!("Merge commit automatically approved.");
                } else if merge_commit.is_merge_commit() && !merge_commit.is_clean_merge() {
                    eprintln!("Warning: Merge commit is not clean and needs manual review.");
                    eprintln!(
                        "Please review the merge commit separately: tw-rs commit {} review",
                        merge_commit.commit_id()
                    );
                }
            }

            break;
        }
    }

    Ok(())
}

fn get_pr_review_notes_from_editor(
    shell: &Shell,
    pull: &tw::PrTask,
    status: &ReviewStatus,
    tip_commit: &GitCommit,
    commits: &[&tw::CommitTask],
) -> Result<String, anyhow::Error> {
    // Create temporary directory and file
    let temp_dir = shell.create_temp_dir()?;
    let temp_file = temp_dir
        .path()
        .join(format!("local-ci-pr-review-{}.txt", pull.number()));

    // Populate temp file with template
    let mut template = String::new();
    template.push_str(&format!(
        "# Enter your PR review here. Updated PR #{} review status: {}\n",
        pull.number(),
        status
    ));

    if *status == ReviewStatus::Approved {
        template.push_str(
            "# This will be posted as a Github approval as soon as all CI runs have passed\n",
        );
        template.push_str("# and all commits are approved.\n");
        template.push_str(&format!(
            "ACK {}; successfully ran local tests\n",
            tip_commit
        ));
    }

    template.push_str("# Commit Review Information:\n");
    // Add commit review information
    for commit in commits {
        template.push_str(&format!("# {}", commit.commit_id()));
        if commit.is_tip() {
            template.push_str(" (TIP)");
        }
        template.push_str(":\n");
        let review_notes = commit.review_notes();
        if !review_notes.is_empty() {
            template.push_str(&format!("#   Review: {}\n", review_notes));
        } else {
            template.push_str("#   Review: (none)\n");
        }
    }
    template
        .push_str("# Edit the approval message above. Lines starting with # will be removed.\n");

    shell.write_file(&temp_file, template)?;

    // Get editor command
    let editor = shell.var("EDITOR").unwrap_or_else(|_| "vim".to_string());

    println!("Opening {} for review notes...", editor);

    // Run editor
    let exit_status = cmd!(shell, "{editor} {temp_file}").run();

    if let Err(e) = exit_status {
        return Err(anyhow::anyhow!("Editor failed: {}. Review cancelled.", e));
    }

    // Read review notes from temp file and remove comment lines
    let content = shell.read_file(&temp_file)?;
    let review_notes: Vec<&str> = content
        .lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .collect();

    let result = review_notes.join("\n");

    // temp_dir is automatically cleaned up when dropped
    Ok(result)
}

fn post_github_approval_if_ready(
    shell: &Shell,
    tasks: &tw::TaskCollection,
    repo: &repo::Repository,
    pull: &tw::PrTask,
) -> Result<(), anyhow::Error> {
    // Only proceed if PR is approved
    if *pull.review_status() != ReviewStatus::Approved {
        return Ok(());
    }

    // Check if all commits are approved and CI successful
    let mut all_commits_approved_and_ci = true;
    for commit in pull.commits(tasks) {
        // Skip merge commits for this check
        if commit.is_merge_commit() {
            continue;
        }
        if *commit.review_status() != ReviewStatus::Approved
            || *commit.ci_status() != CiStatus::Success
        {
            all_commits_approved_and_ci = false;
            break;
        }
    }

    // If all commits successful and PR is approved, post approval on GitHub
    if all_commits_approved_and_ci {
        println!(
            "All commits in PR #{} are successful and PR is approved. Checking if approval already posted...",
            pull.number()
        );

        // Get PR review notes
        let pr_review_notes = pull.review_notes();
        let pr_num = pull.number().to_string();

        let _push_dir = shell.push_dir(&repo.repo_root);

        // Get current user's GitHub username
        let current_user_result = cmd!(shell, "gh api user --jq '.login'").read();
        let current_user = match current_user_result {
            Ok(username) => username.trim().to_string(),
            Err(e) => {
                println!(
                    "Warning: Could not get current GitHub username ({}), proceeding with approval",
                    e
                );
                return Ok(());
            }
        };

        // Check if we've already posted this exact approval message
        let existing_reviews_result = cmd!(shell, "gh pr view {pr_num} --json reviews --jq '.reviews[] | select(.state == \"APPROVED\" and .author.login == \"'{current_user}'\") | .body'")
            .read();

        let should_post_approval = match existing_reviews_result {
            Ok(existing_reviews) => {
                let already_posted = existing_reviews
                    .lines()
                    .any(|review_body| review_body.trim() == pr_review_notes.trim());

                if already_posted {
                    println!(
                        "Approval with same message already posted for PR #{}",
                        pull.number()
                    );
                    false
                } else {
                    true
                }
            }
            Err(e) => {
                println!(
                    "Warning: Could not check existing reviews ({}), proceeding with approval",
                    e
                );
                true
            }
        };

        if should_post_approval {
            println!("Posting GitHub approval for PR #{}...", pull.number());
            let approval_result =
                cmd!(shell, "gh pr review {pr_num} -a -b {pr_review_notes}").run();

            match approval_result {
                Ok(_) => {
                    println!("Successfully posted approval for PR #{}", pull.number());
                }
                Err(_) => {
                    println!(
                        "Failed to post approval for PR #{} - posting comment instead",
                        pull.number()
                    );
                    let _ = cmd!(shell, "gh pr review {pr_num} -c -b {pr_review_notes}").run();
                }
            }
        }
    } else {
        println!(
            "PR #{} is approved but not all commits are approved and CI successful yet",
            pull.number()
        );
    }

    Ok(())
}

fn get_review_notes_from_editor(
    shell: &Shell,
    commit: &CommitTask,
    status: &ReviewStatus,
) -> Result<String, anyhow::Error> {
    // Create temporary directory and file
    let temp_dir = shell.create_temp_dir()?;
    let temp_file = temp_dir
        .path()
        .join(format!("local-ci-review-{}.txt", commit.commit_id()));

    // Populate temp file with template
    let mut template = commit.review_notes().to_owned();
    if template.is_empty() {
        template.push_str(&format!(
            "# Enter your review here. Updated commit {} review status: {}\n",
            commit.commit_id(),
            status
        ));
        template
            .push_str("# Edit the review message above. Lines starting with # will be removed.\n");
    }

    shell.write_file(&temp_file, template)?;

    // Get editor command
    let editor = shell.var("EDITOR").unwrap_or_else(|_| "vim".to_string());

    println!("Opening {} for review notes...", editor);

    // Run editor
    let exit_status = cmd!(shell, "{editor} {temp_file}").run();

    if let Err(e) = exit_status {
        return Err(anyhow::anyhow!("Editor failed: {}. Review cancelled.", e));
    }

    // Read review notes from temp file and remove comment lines
    let content = shell.read_file(&temp_file)?;
    let review_notes: Vec<&str> = content
        .lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .collect();

    let result = review_notes.join("\n");

    // temp_dir is automatically cleaned up when dropped
    Ok(result)
}

fn main() -> Result<(), anyhow::Error> {
    // Parse CLI arguments -- if this fails it will just terminate the program
    // with a usage message.
    let args = args::parse_cli();

    // Check that all required tools are available
    if let Err(e) = check_required_tools() {
        eprintln!("Error checking required tools: {}", e);
        std::process::exit(1);
    }

    // Load task database. When this gets too slow to invoke on every command we
    // we will move it into the daemon that runs when we do 'local-ci run' and
    // then do RPC calls against that (and add some sort of "reload" command
    // hooked to TW adds and modifies). But for now we just query taskwarrior
    // every time.
    let shell = tw::task_shell().context("creating task shell")?;
    let mut tasks = tw::TaskCollection::new(&shell).context("loading tasks from taskwarrior")?;

    real_main(&shell, &mut tasks, args.action, args.target)
}
