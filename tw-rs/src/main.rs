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
            let pull = match tasks.pull_by_number(&repo.project_name, num) {
                Some(task) => task,
                None => tasks.insert_or_refresh_pr(&shell, &repo, num)
                    .context("adding new PR")?
            };

            match args.action {
                Action::Info => {
                    println!("{} #{}: {}", pull.project(), pull.number(), pull.title());
                }
                Action::Refresh => todo!(),
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
        Target::Commit(_) => {
            todo!()
        },
        Target::None => unreachable!("this case handled above, before current_repo()"),
    }

    Ok(())
}
