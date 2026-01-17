// SPDX-License-Identifier: GPL-3.0-or-later

mod args;
mod gh;
mod git;
mod jj;
mod repo;
mod tw;

use anyhow::Context;
use xshell::{Shell, cmd};

use self::args::{Action, Target};
use self::tw::serde_types::ReviewStatus;

fn check_required_tools() -> xshell::Result<()> {
    let sh = Shell::new()?;
    let tools = ["gh", "git", "jj", "nix", "task"];
    
    for tool in &tools {
        if cmd!(sh, "{tool} --version").quiet().ignore_stdout().run().is_err() {
            eprintln!("Error: Required tool '{}' is not available or not in PATH", tool);
            std::process::exit(1);
        }
    }
    
    Ok(())
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
    let shell = tw::task_shell()
        .context("creating task shell")?;
    let mut tasks = tw::TaskCollection::new(&shell)
        .context("loading tasks from taskwarrior")?;

    if args.target == Target::None {
        // Handle "global" actions, which don't need to be in an active repo
        // and which can avoid invoking gh (except refresh, which does on
        // purpose).
        match args.action {
            Action::Review => {
                eprintln!("Nothing to review. (Did you mean to provide a PR number or commit ID?");
                eprintln!();
                args::usage();
                std::process::exit(1);
            },
            Action::TaskEdit | Action::TaskInfo => {
                eprintln!("Nothing to query taskwarrior for. (Did you mean to provide a PR number or commit ID?");
                eprintln!();
                args::usage();
                std::process::exit(1);
            },
            Action::Info => {
                for (_, pull) in tasks.pulls() {
                    println!("{} PR #{}: {}", pull.project(), pull.number(), pull.title());
                }
            },
            Action::Refresh => {
                eprintln!("[invoking gh here]");
            },
            Action::Run => {
                eprintln!("[run loop here]");
            },
        }
        return Ok(());
    } else {
        // Error out tfor actions which don't have any target.
        if args.action == Action::Run {
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
    match args.target {
        Target::Pr(num) => {
            let lookup = tasks.pull_by_number(&repo.project_name, num);
            let just_created = lookup.is_none();
            let pull = match lookup {
                Some(task) => task,
                None => tasks.insert_or_refresh_pr(&shell, &repo, num)
                    .context("adding new PR")?
            };
            // Clone the pull to end the mutable borrow of `tasks`.
            let pull = pull.clone();

            match args.action {
                Action::Info => {
                    println!("=== PR #{}: {} ===", pull.number(), pull.title());
                    println!("Author: {}", pull.author());
                    println!("PR Review Status: {}", pull.review_status());
                    println!("PR Merge Status: {}", pull.merge_status());
                    println!();
                    
                    println!("=== Commits ===");
                    let commits: Vec<_> = pull.commits(&tasks).collect();
                    
                    for commit in &commits {
                        print!("  {} (review: {}", commit.commit_id(), commit.review_status());
                        
                        if matches!(commit.review_status(), ReviewStatus::Approved) {
                            print!(", ci: {}", commit.ci_status());
                        }
                        
                        if commit.is_tip() {
                            print!(", TIP");
                        }
                        
                        println!(")");
                    }
                    
                    println!();
                    println!("=== Merge Commit ===");
                    let merge_commit = pull.merge_commit(&tasks);
                    print!("  Merge commit: {} (review: {}", merge_commit.commit_id(), merge_commit.review_status());
                    if matches!(merge_commit.review_status(), ReviewStatus::Approved) {
                        print!(", ci: {}", merge_commit.ci_status());
                    }
                    println!(")");
                    
                    if !pull.merge_change_id().is_empty() {
                        println!("  JJ change ID: {}", pull.merge_change_id());
                    }
                    println!("  (Review the merge commit like any other commit to trigger CI)");
                    
                    println!();
                    println!("Commit graph:");
                    let mut revset_str = format!("{} | {}", pull.base_commit(), merge_commit.commit_id());
                    for commit in &commits {
                        revset_str.push_str(&format!(" | {}", commit.commit_id()));
                    }
                    let _ = cmd!(shell, "jj log --no-pager --ignore-working-copy -r {revset_str}").quiet().run();
                }
                Action::Refresh => {
                    if !just_created {
                        tasks.insert_or_refresh_pr(&shell, &repo, num)
                            .context("refreshing PR")?;
                    }
                },
                Action::Review => todo!(),
                Action::Run => unreachable!("checked above"),
                Action::TaskEdit => {
                    let uuid = pull.uuid().to_string();
                    cmd!(&shell, "task edit {uuid}")
                        .run()
                        .context("executing task edit")?;
                }
                Action::TaskInfo => {
                    let uuid = pull.uuid().to_string();
                    cmd!(&shell, "task info {uuid}")
                        .run()
                        .context("executing task info")?;
                }
            }

        },
        Target::Commit(commit_str) => {
            // Resolve the git ref to a commit ID
            let commit_id = git::resolve_ref(&shell, &commit_str)
                .context("resolving git commit")?;

            let lookup = tasks.commit_by_id(&repo.project_name, &commit_id);
            let just_created = lookup.is_none();
            let commit_uuid = match lookup {
                Some(task) => *task.uuid(),
                None => tasks.insert_or_refresh_commit(&shell, &repo.project_name, &repo.repo_root, &commit_id)
                    .context("adding new commit")?
            };

            match args.action {
                Action::Info => {
                    if let Some(commit_task) = tasks.commit_by_id(&repo.project_name, &commit_id) {
                        println!("{}: {}", commit_task.project(), commit_task.description());
                    } else {
                        println!("Commit {} in project {}", commit_id, repo.project_name);
                    }
                }
                Action::Refresh => {
                    if !just_created {
                        tasks.insert_or_refresh_commit(&shell, &repo.project_name, &repo.repo_root, &commit_id)
                            .context("refreshing commit")?;
                    }
                },
                Action::Review => todo!(),
                Action::Run => unreachable!("checked above"),
                Action::TaskEdit => {
                    let uuid = commit_uuid.to_string();
                    cmd!(&shell, "task edit {uuid}")
                        .run()
                        .context("executing task edit")?;
                }
                Action::TaskInfo => {
                    let uuid = commit_uuid.to_string();
                    cmd!(&shell, "task info {uuid}")
                        .run()
                        .context("executing task info")?;
                }
            }
        },
        Target::None => unreachable!("this case handled above, before current_repo()"),
    }

    Ok(())
}
