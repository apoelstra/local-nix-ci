// SPDX-License-Identifier: GPL-3.0-or-later

mod schema;
mod util;
pub mod models;
pub mod operations;

use tokio_postgres::{Client, NoTls, Transaction};

pub use self::schema::SchemaError;
pub use self::util::{EntityType, log_action_simple};
pub use self::models::{AckStatus, CiStatus, MergeStatus, ReviewStatus, Log};

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
        let (mut client, connection) = tokio_postgres::connect("host=localhost user=postgres dbname=local-ci", NoTls)
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

    /// Get a reference to the underlying client for direct queries
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Start a new transaction
    ///
    /// # Errors
    ///
    /// Returns an error if the transaction cannot be started.
    pub async fn transaction(&mut self) -> Result<Transaction<'_>, tokio_postgres::Error> {
        self.client.transaction().await
    }

    /// Execute a function within a transaction, automatically committing or rolling back
    ///
    /// # Errors
    ///
    /// Returns an error if the transaction fails or if the function returns an error.
    pub async fn with_transaction<F, R, E>(&mut self, f: F) -> Result<R, Error>
    where
        F: for<'a> FnOnce(&'a Transaction<'a>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<R, E>> + Send + 'a>>,
        E: Into<Error>,
    {
        let tx = self.transaction().await.map_err(Error::Connect)?;
        match f(&tx).await {
            Ok(result) => {
                tx.commit().await.map_err(Error::Connect)?;
                Ok(result)
            }
            Err(e) => {
                let _ = tx.rollback().await; // Ignore rollback errors
                Err(e.into())
            }
        }
    }

    /// Get the current schema version
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub async fn get_schema_version(&mut self) -> Result<i32, Error> {
        let tx = self.transaction().await.map_err(Error::Connect)?;
        let version = util::get_schema_version(&tx).await.map_err(Error::Connect)?;
        tx.commit().await.map_err(Error::Connect)?;
        Ok(version)
    }

    /// Check if a repository exists by path
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub async fn repository_exists_by_path(&mut self, path: &str) -> Result<bool, Error> {
        let tx = self.transaction().await.map_err(Error::Connect)?;
        let exists = util::repository_exists_by_path(&tx, path).await.map_err(Error::Connect)?;
        tx.commit().await.map_err(Error::Connect)?;
        Ok(exists)
    }

    /// Get repository ID by path
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub async fn get_repository_id_by_path(&mut self, path: &str) -> Result<Option<i32>, Error> {
        let tx = self.transaction().await.map_err(Error::Connect)?;
        let id = util::get_repository_id_by_path(&tx, path).await.map_err(Error::Connect)?;
        tx.commit().await.map_err(Error::Connect)?;
        Ok(id)
    }

    /// Check if a commit exists by git commit ID
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub async fn commit_exists_by_git_id(&mut self, repository_id: i32, git_commit_id: &str) -> Result<bool, Error> {
        let tx = self.transaction().await.map_err(Error::Connect)?;
        let exists = util::commit_exists_by_git_id(&tx, repository_id, git_commit_id).await.map_err(Error::Connect)?;
        tx.commit().await.map_err(Error::Connect)?;
        Ok(exists)
    }

    /// Check if a pull request exists
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub async fn pull_request_exists(&mut self, repository_id: i32, pr_number: i32) -> Result<bool, Error> {
        let tx = self.transaction().await.map_err(Error::Connect)?;
        let exists = util::pull_request_exists(&tx, repository_id, pr_number).await.map_err(Error::Connect)?;
        tx.commit().await.map_err(Error::Connect)?;
        Ok(exists)
    }

    /// Log an action (convenience method)
    ///
    /// # Errors
    ///
    /// Returns an error if the logging fails.
    pub async fn log_action(
        &mut self,
        entity_type: EntityType,
        entity_id: i32,
        action: &str,
        description: Option<&str>,
        reason: Option<&str>,
    ) -> Result<(), Error> {
        log_action_simple(&mut self.client, entity_type, entity_id, action, description, reason)
            .await
            .map_err(Error::Connect)
    }
}

#[derive(Debug)]
pub enum Error {
    Connect(tokio_postgres::Error),
    Schema(SchemaError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connect(..) => f.write_str("database connection error"),
            Self::Schema(..) => f.write_str("database schema error"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Connect(e) => Some(e),
            Self::Schema(e) => Some(e),
        }
    }
}
