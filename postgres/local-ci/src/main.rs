
mod args;

use anyhow::Context as _;
use lcilib::Db;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Will abort on error
    let _args = args::parse_cli();

    let _db = Db::connect().await
        .context("connecting to database")?;

    Ok(())
}
