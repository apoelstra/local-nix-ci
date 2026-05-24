// SPDX-License-Identifier: GPL-3.0-or-later

mod args;
mod commit;
mod daemon;
mod pr;
mod repo_info;
mod terminal;

use anyhow::Context as _;
use args::{Action, Target};
use lcilib::Db;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = args::parse_cli();
    let mut db = Db::connect().await.context("connecting to database")?;

    if !args.enable_color {
        terminal::disable_terminal_color();
    }

    // Check and set GitHub username if not configured
    db.with_transaction(|tx| async move {
        let username = tx.get_github_username().await?;
        
        if username.is_none() {
            println!("GitHub username not configured. Please enter your GitHub username:");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).context("reading GitHub username")?;
            let username = input.trim();
            
            if username.is_empty() {
                anyhow::bail!("GitHub username cannot be empty");
            }

            tx.set_github_username(username).await?;
            println!("GitHub username set to: {}", username);
        }
        
        Ok(())
    }).await
    // Need to panic on this error rather than returning it because of std::Error bugs
    // (as manifested in anyhow)
    .expect("configuring GitHub username");

    match (args.action, args.target) {
        (Action::Info, Target::None) => {
            repo_info::overview(&mut db)
                .await
                .context("getting repository overview")?;
        }
        (Action::Info, Target::Pr(pr_number)) => {
            pr::info(pr_number, &mut db)
                .await
                .context("getting PR info")?;
        }
        (Action::Refresh, Target::Pr(pr_number)) => {
            pr::refresh_from_cli(pr_number, &mut db)
                .await
                .context("refreshing PR")?;
        }
        (Action::Log, Target::Pr(pr_number)) => {
            let log_options = args.log_options.as_ref().unwrap();
            pr::log(
                pr_number,
                log_options.since.as_deref(),
                log_options.until.as_deref(),
                &mut db,
            )
            .await
            .context("getting PR logs")?;
        }
        (Action::Info, Target::Commit(commit_ref)) => {
            commit::info(&commit_ref, &mut db)
                .await
                .context("getting commit info")?;
        }
        (Action::Refresh, Target::Commit(commit_ref)) => {
            commit::refresh(&commit_ref, &mut db)
                .await
                .context("refreshing commit")?;
        }
        (Action::Log, Target::Commit(commit_ref)) => {
            let log_options = args.log_options.as_ref().unwrap();
            commit::log(
                &commit_ref,
                log_options.since.as_deref(),
                log_options.until.as_deref(),
                &mut db,
            )
            .await
            .context("getting commit logs")?;
        }
        (Action::Review, Target::Commit(commit_ref)) => {
            commit::review(&commit_ref, &mut db)
                .await
                .context("reviewing commit")?;
        }
        (Action::Review, Target::Pr(pr_number)) => {
            pr::review(pr_number, &mut db)
                .await
                .context("reviewing PR")?;
        }
        (Action::Next, Target::Pr(pr_number)) => {
            pr::next(pr_number, &mut db)
                .await
                .context("finding next action for PR")?;
        }
        (Action::Next, Target::Commit(commit_ref)) => {
            commit::next(&commit_ref, &mut db)
                .await
                .context("finding next action for commit")?;
        }
        (Action::Run, Target::None) => {
            daemon::run(&mut db).await.context("running daemon")?;
        }
        (Action::Run, _) => {
            eprintln!(
                "The 'run' action does not accept any target. Use 'local-ci run' with no arguments."
            );
            std::process::exit(1);
        }
        (Action::Next, Target::None) => {
            println!("Use 'local-ci info' to see a list of work that needs to be done.");
        }
        _ => {
            eprintln!("Action not yet implemented");
            std::process::exit(1);
        }
    }

    Ok(())
}
