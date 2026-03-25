// SPDX-License-Identifier: GPL-3.0-or-later

pub mod models;
pub mod operations;
mod schema;
mod util;

use core::fmt;
use core::pin::Pin;
use tokio_postgres::{Client, NoTls};

pub use self::models::{AckStatus, CiStatus, Log, MergeStatus, ReviewStatus};
pub use self::schema::SchemaError;
pub use self::util::EntityType;

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
        let (mut client, connection) =
            tokio_postgres::connect("host=localhost user=postgres dbname=local-ci", NoTls)
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
    pub async fn transaction(&mut self) -> Result<Transaction<'_>, DbTransactionError> {
        self.client.transaction().await.map(|inner| Transaction { inner })
        .map_err(DbTransactionError::Construct)
    }

    /// Execute a function within a transaction, automatically committing or rolling back.
    ///
    /// The function is expected to be async, but its returned future must be boxed and
    /// pinned. For example, see the implementation of [`Self::get_schema_version`].
    /// This is ultimately due to limitations of Rust's lifetime syntax.
    ///
    /// # Errors
    ///
    /// Returns an error if the transaction fails or if the function returns an error.
    pub async fn with_transaction<R, F>(&mut self, f: F) -> Result<R, DbTransactionError>
    where
        R: Send + 'static,
        F: for<'tx> FnOnce(
            &'tx Transaction<'tx>,
        ) -> Pin<Box<dyn Future<Output = Result<R, DbQueryError>> + Send + 'tx>>,
    {
        let tx = self.transaction().await?;
        match f(&tx).await {
            Ok(result) => {
                tx.commit().await?;
                Ok(result)
            }
            Err(e) => {
                let _ = tx.rollback().await; // Ignore rollback errors
                Err(DbTransactionError::Query(e))
            }
        }
    }

    /// Get the current schema version
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub async fn get_schema_version(&mut self) -> Result<i32, DbTransactionError> {
        self.with_transaction(|tx| Box::pin(util::get_schema_version(tx))).await
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

pub struct Transaction<'db> {
    inner: tokio_postgres::Transaction<'db>,
}

// FIXME this allows direct database access to work. We want to remove all
// these accesses then delete this. Will do over the coming commits.
impl<'db> core::ops::Deref for Transaction<'db> {
    type Target = tokio_postgres::Transaction<'db>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Transaction<'_> {
    /// Commits the transaction to the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the commitment fails.
    pub async fn commit(self) -> Result<(), DbTransactionError> {
        self.inner.commit().await.map_err(DbTransactionError::Commit)
    }

    /// Cancels the transaction, rolling back any changes it made.
    ///
    /// # Errors
    ///
    /// Returns an error if the rollback fails.
    pub async fn rollback(self) -> Result<(), DbTransactionError> {
        self.inner.rollback().await.map_err(DbTransactionError::Rollback)
    }
}

#[derive(Debug)]
pub enum DbTransactionError {
    Construct(tokio_postgres::Error),
    Commit(tokio_postgres::Error),
    Rollback(tokio_postgres::Error),
    Query(DbQueryError),
}

impl fmt::Display for DbTransactionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Construct(..) => f.write_str("failed to open transaction"),
            Self::Commit(..) => f.write_str("failed to commit transaction"),
            Self::Rollback(..) => f.write_str("failed to rollback transaction"),
            Self::Query(..) => f.write_str("failed to execute query within transaction"),
        }
        
    }
}

impl std::error::Error for DbTransactionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            Self::Construct(ref e) => Some(e),
            Self::Commit(ref e) => Some(e),
            Self::Rollback(ref e) => Some(e),
            Self::Query(ref e) => Some(e),
        }
        
    }
}

#[derive(Debug)]
pub struct DbQueryError {
    action: &'static str,
    entity_type: EntityType,
    raw_id: Option<i32>,
    clauses: Vec<String>,
    error: tokio_postgres::Error,
}

impl fmt::Display for DbQueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(id) = self.raw_id {
            write!(
                f,
                "failed to {} {} {} (clauses {:?}): {}",
                self.action, self.entity_type, id, self.clauses, self.error
            )
        } else {
            write!(
                f,
                "failed to {} {} (clauses {:?}): {}",
                self.action, self.entity_type, self.clauses, self.error
            )
        }
    }
}

impl std::error::Error for DbQueryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}
