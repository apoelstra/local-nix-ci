// SPDX-License-Identifier: GPL-3.0-or-later

use tokio_postgres::{Client, NoTls};

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
    pub async fn connect() -> Result<Self, tokio_postgres::Error> {
        // This connect-and-spawn logic is directly from the `tokio_postgres` front-page docs,
        // except that I'm holding the joinhandle (for future proofing, e.g. to handle shutdown
        // more gracefully or something) rather than just dropping it.
        let (client, connection) = tokio_postgres::connect("host=localhost user=postgres", NoTls).await?;
        let driver = tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        Ok(Self {
            client,
            _driver: driver,
        })
    }
}

