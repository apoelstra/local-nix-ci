// SPDX-License-Identifier: GPL-3.0-or-later

mod args;
mod repo;
mod tw;

use anyhow::Context;
use std::io::BufRead as _; // for lines
use xshell::{Shell, cmd};

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

    if args.target == args::Target::None {
        // Handle "global" actions, which don't need to be in an active repo
        // and which can avoid invoking gh (except refresh, which does on
        // purpose).
        match args.action {
            args::Action::Approve | args::Action::Nack | args::Action::Review => {
                eprintln!("Nothing to review. (Did you mean to provide a PR number or commit ID?");
                eprintln!();
                args::usage();
                std::process::exit(1);
            },
            args::Action::Info => {
                let shell = tw::task_shell()
                    .context("creating task shell")?;
                let tasks = tw::TaskCollection::new(&shell)
                    .context("loading tasks from taskwarrior")?;

                for (_, pull) in tasks.pulls() {
                    println!("{} PR #{}: {}", pull.project(), pull.number(), pull.title());
                }
            },
            args::Action::Refresh => {
                eprintln!("[invoking gh here]");
            },
            args::Action::Run => {
                eprintln!("[run loop here]");
            },
        }
        return Ok(());
    } else {
        // Error out tfor actions which don't have any target.
        if args.action == args::Action::Run {
            eprintln!("'run' cannot be invoked with any target.");
            eprintln!();
            args::usage();
            std::process::exit(1);
        }
    }

    // Orient ourselves.
    let repo = match repo::current_repo() {
        Err(e) => {
            eprintln!("Failed to locate repo: {e}");
            eprintln!("Are you in a Github-connected git repository (that 'gh' understands)?");
            std::process::exit(1);
        }
        Ok(repo) => repo,
    };

    Ok(())
}
