use std::path::Path;

use crate::Result;
use async_trait::async_trait;
use uuid::Uuid;

use self::model::UploadFileEntry;

pub mod model;
pub mod sqlite_ds;

#[async_trait]
pub trait DataSource: Clone {
    async fn init_database(&self) -> Result<()>
    where
        Self: Sized + Clone;
    async fn add_file_entry(&self, file_path: &Path) -> Result<Uuid>
    where
        Self: Sized + Clone;
    async fn remove_file_entry(&self, id: String) -> Result<()>
    where
        Self: Sized + Clone;
    async fn get_file_entry(&self, id: String) -> Result<UploadFileEntry>
    where
        Self: Sized + Clone;
}
