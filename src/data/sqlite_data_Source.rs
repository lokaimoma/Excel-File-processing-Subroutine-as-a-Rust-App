use std::path::Path;

use super::{model::UploadFileEntry, DataSource};
use crate::{
    error::{self, Error},
    Result,
};
use async_trait::async_trait;
use rusqlite::Connection;
use uuid::Uuid;

pub struct SqliteDataSource(Connection);

#[async_trait]
impl DataSource for SqliteDataSource {
    async fn init_database(self) -> Result<()> {
        let stmt = format!("CREATE TABLE IF NOT EXISTS {t_name} ({id_col} TEXT PRIMARY KEY, {fname_col} TEXT NOT NULL);", t_name=Self::UPLOAD_TABLE_NAME, id_col=Self::UPLOAD_T_ID_COL, fname_col=Self::UPLOAD_T_FILE_NAME_COL);

        if let Err(e) = self.0.execute(&stmt, ()) {
            return Err(error::Error::DatabaseOperationFailed(e.to_string()));
        };

        Ok(())
    }

    async fn add_file_entry(self, file_path: &Path) -> Result<()> {
        let id_val = Uuid::now_v7();
        let stmt = format!(
            "INSERT INTO {t_name} ({id_col}, {fname_col}) values ({id_val}, {file_path});",
            t_name = Self::UPLOAD_TABLE_NAME,
            id_col = Self::UPLOAD_T_ID_COL,
            fname_col = Self::UPLOAD_T_FILE_NAME_COL,
            file_path = file_path.display()
        );

        if let Err(e) = self.0.execute(&stmt, ()) {
            return Err(error::Error::DatabaseOperationFailed(e.to_string()));
        }

        Ok(())
    }

    async fn remove_file_entry(self, id: String) -> Result<()> {
        let stmt = format!(
            "DELETE FROM {t_name} where {id_col} = {id}",
            t_name = Self::UPLOAD_TABLE_NAME,
            id_col = Self::UPLOAD_T_ID_COL
        );
        if let Err(e) = self.0.execute(&stmt, ()) {}

        Ok(())
    }

    async fn get_file_entry(self, id: String) -> Result<Option<UploadFileEntry>> {
        todo!()
    }
}
