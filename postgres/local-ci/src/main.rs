
mod args;
mod pr;

use anyhow::Context as _;
use args::{Action, Target};
use lcilib::Db;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = args::parse_cli();
    let mut db = Db::connect().await
        .context("connecting to database")?;

    match (args.action, args.target) {
        (Action::Info, Target::Pr(pr_number)) => {
            pr::info(pr_number, &mut db).await
                .context("getting PR info")?;
        }
        (Action::Refresh, Target::Pr(pr_number)) => {
            pr::refresh(pr_number, &mut db).await
                .context("refreshing PR")?;
        }
        _ => {
            eprintln!("Action not yet implemented");
            std::process::exit(1);
        }
    }

    Ok(())
}
