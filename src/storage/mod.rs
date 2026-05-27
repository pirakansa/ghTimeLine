pub mod items;
pub mod queries;
pub mod schema;

use std::path::Path;

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use thiserror::Error;

use crate::models::HostConfig;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("invalid local filter: {0}")]
    InvalidFilter(String),
}

pub type Result<T> = std::result::Result<T, StorageError>;

pub struct Storage {
    connection: Connection,
}

impl Storage {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| {
                StorageError::Database(rusqlite::Error::ToSqlConversionFailure(Box::new(err)))
            })?;
        }
        let connection = Connection::open(path)?;
        Self::from_connection(connection)
    }

    pub fn in_memory() -> Result<Self> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(connection: Connection) -> Result<Self> {
        connection.pragma_update(None, "foreign_keys", "ON")?;
        schema::migrate(&connection)?;
        Ok(Self { connection })
    }

    pub fn connection(&self) -> &Connection {
        &self.connection
    }

    pub fn with_immediate_transaction<T>(
        &self,
        action: impl FnOnce(&Self) -> Result<T>,
    ) -> Result<T> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(StorageError::from)?;
        let result = action(self);
        match result {
            Ok(value) => {
                transaction.commit().map_err(StorageError::from)?;
                Ok(value)
            }
            Err(err) => Err(err),
        }
    }

    pub fn ensure_host(&self, host: &HostConfig) -> Result<i64> {
        let fingerprint = host.fingerprint();
        let existing = self
            .connection
            .query_row(
                "SELECT id FROM hosts WHERE fingerprint = ?1",
                params![fingerprint],
                |row| row.get::<_, i64>(0),
            )
            .optional()?;

        let now = Utc::now().to_rfc3339();
        if let Some(id) = existing {
            self.connection.execute(
                "UPDATE hosts
                 SET name = ?1, kind = ?2, scheme = ?3, hostname = ?4,
                     rest_api_base_path = ?5, updated_at = ?6
                 WHERE id = ?7",
                params![
                    host.name,
                    host.kind.to_string(),
                    host.scheme.to_string(),
                    host.hostname,
                    host.rest_api_base_path,
                    now,
                    id
                ],
            )?;
            return Ok(id);
        }

        self.connection.execute(
            "INSERT INTO hosts (
                fingerprint, name, kind, scheme, hostname, rest_api_base_path,
                created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
            params![
                host.fingerprint(),
                host.name,
                host.kind.to_string(),
                host.scheme.to_string(),
                host.hostname,
                host.rest_api_base_path,
                now
            ],
        )?;
        Ok(self.connection.last_insert_rowid())
    }
}
