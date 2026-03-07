// SPDX-License-Identifier: GPL-3.0-or-later

mod schema;
mod util;

use tokio_postgres::{Client, NoTls};

pub use self::schema::SchemaError;

pub struct Db {
    client: Client, 
    _driver: tokio::task::JoinHandle<()>,
}

impl Db {
    /// Connect to the database and return a connection handle.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection fails (e.g. the database is not running or the default
    /// user doesn't work).
    pub async fn connect() -> Result<Self, Error> {
        // This connect-and-spawn logic is directly from the `tokio_postgres` front-page docs,
        // except that I'm holding the joinhandle (for future proofing, e.g. to handle shutdown
        // more gracefully or something) rather than just dropping it.
        let (mut client, connection) = tokio_postgres::connect("host=localhost user=postgres", NoTls)
            .await
            .map_err(Error::Connect)?;
        let driver = tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        // This "ensure schema" business is from ChatGPT, which recommends "refinery" which will
        // encapsulate this, at the cost of 160 dependencies.
        schema::ensure_schema(&mut client)
            .await
            .map_err(Error::Schema)?;

        Ok(Self {
            client,
            _driver: driver,
        })
    }
}

pub enum Error {
    Connect(tokio_postgres::Error),
    Schema(SchemaError),
}
