
use lcilib::Db;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db = Db::connect().await?;

    Ok(())
}
