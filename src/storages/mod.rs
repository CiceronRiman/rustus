use std::collections::HashMap;
use std::str::FromStr;

use actix_files::NamedFile;
use async_trait::async_trait;
use chrono::serde::ts_seconds;
use chrono::{DateTime, Utc};
use derive_more::{Display, From};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::errors::RustusResult;
use crate::RustusConf;

pub mod file_storage;
pub mod sqlite_file_storage;

/// Enum of available Storage implementations.
#[derive(PartialEq, From, Display, Clone, Debug)]
pub enum AvailableStores {
    #[display(fmt = "FileStorage")]
    FileStorage,
    #[display(fmt = "SqliteFileStorage")]
    SqliteFileStorage,
}

impl FromStr for AvailableStores {
    type Err = String;

    /// This function converts string to the `AvailableStore` item.
    /// This function is used by structopt to parse CLI parameters.
    ///
    /// # Params
    /// `input` - input string.
    fn from_str(input: &str) -> Result<AvailableStores, Self::Err> {
        match input {
            "file_storage" => Ok(AvailableStores::FileStorage),
            "sqlite_file_storage" => Ok(AvailableStores::SqliteFileStorage),
            _ => Err(String::from("Unknown storage type")),
        }
    }
}

impl AvailableStores {
    /// Convert `AvailableStores` to the Storage.
    ///
    /// # Params
    /// `config` - Rustus configuration.
    ///
    pub fn get(&self, config: &RustusConf) -> Box<dyn Storage + Send + Sync> {
        match self {
            Self::FileStorage => Box::new(file_storage::FileStorage::new(config.clone())),
            Self::SqliteFileStorage => {
                Box::new(sqlite_file_storage::SQLiteFileStorage::new(config.clone()))
            }
        }
    }
}

/// Information about file.
/// It has everything about stored file.
#[derive(Clone, Debug, Serialize, Deserialize, FromRow)]
pub struct FileInfo {
    pub id: String,
    pub offset: usize,
    pub length: usize,
    pub path: String,
    #[serde(with = "ts_seconds")]
    pub created_at: DateTime<Utc>,
    pub deferred_size: bool,
    pub metadata: HashMap<String, String>,
}

impl FileInfo {
    /// Creates new `FileInfo`.
    ///
    /// # Params
    ///
    /// File info takes
    /// `file_id` - Unique file identifier;
    /// `file_size` - Size of a file if it's known;
    /// `path` - local path of a file;
    /// `initial_metadata` - meta information, that could be omitted.
    pub fn new(
        file_id: &str,
        file_size: Option<usize>,
        path: String,
        initial_metadata: Option<HashMap<String, String>>,
    ) -> FileInfo {
        let id = String::from(file_id);
        let mut length = 0;
        let mut deferred_size = true;
        if let Some(size) = file_size {
            length = size;
            deferred_size = false;
        }
        let metadata = match initial_metadata {
            Some(meta) => meta,
            None => HashMap::new(),
        };

        FileInfo {
            id,
            path,
            length,
            metadata,
            deferred_size,
            offset: 0,
            created_at: chrono::Utc::now(),
        }
    }
}

#[async_trait]
pub trait Storage {
    /// Prepare storage before starting up server.
    ///
    /// Function to check if configuration is correct
    /// and prepare storage E.G. create connection pool,
    /// or directory for files.
    async fn prepare(&mut self) -> RustusResult<()>;

    /// Get file information.
    ///
    /// This method returns all information about file.
    ///
    /// # Params
    /// `file_id` - unique file identifier.
    async fn get_file_info(&self, file_id: &str) -> RustusResult<FileInfo>;

    /// Set file info
    ///
    /// This method will save information about current upload.
    ///
    /// # Params
    /// `file_info` - information about current upload.
    async fn set_file_info(&self, file_info: &FileInfo) -> RustusResult<()>;

    /// Get contents of a file.
    ///
    /// This method must return NamedFile since it
    /// is compatible with ActixWeb files interface.
    ///
    /// # Params
    /// `file_id` - unique file identifier.
    async fn get_contents(&self, file_id: &str) -> RustusResult<NamedFile>;

    /// Add bytes to the file.
    ///
    /// This method is used to append bytes to some file.
    /// It returns new offset.
    ///
    /// # Params
    /// `file_id` - unique file identifier;
    /// `request_offset` - offset from the client.
    /// `bytes` - bytes to append to the file.
    async fn add_bytes(
        &self,
        file_id: &str,
        request_offset: usize,
        bytes: &[u8],
    ) -> RustusResult<usize>;

    /// Create file in storage.
    ///
    /// This method is used to generate unique file id, create file and store information about it.
    ///
    /// # Params
    /// `file_size` - Size of a file. It may be None if size is deffered;
    /// `metadata` - Optional file metainformation;
    async fn create_file(
        &self,
        file_size: Option<usize>,
        metadata: Option<HashMap<String, String>>,
    ) -> RustusResult<String>;

    /// Remove file from storage
    ///
    /// This method removes file and all associated
    /// object if any.
    ///
    /// # Params
    /// `file_id` - unique file identifier;
    async fn remove_file(&self, file_id: &str) -> RustusResult<()>;
}
