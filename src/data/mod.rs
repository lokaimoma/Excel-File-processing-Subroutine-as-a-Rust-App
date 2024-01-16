use std::path::Path;

use crate::Result;
use async_trait::async_trait;

use self::model::UploadFileEntry;

pub mod model;
pub mod sqlite_data_Source;

#[async_trait]
pub trait DataSource {
    const UPLOAD_TABLE_NAME: &'static str = "UploadEntriesTable";
    const UPLOAD_T_ID_COL: &'static str = "ID";
    const UPLOAD_T_FILE_NAME_COL: &'static str = "FILE_NAME";
    async fn init_database(self) -> Result<()>;
    async fn add_file_entry(self, file_path: &Path) -> Result<()>;
    async fn remove_file_entry(self, id: String) -> Result<()>;
    async fn get_file_entry(self, id: String) -> Result<Option<UploadFileEntry>>;
}
