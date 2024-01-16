use std::{path::Path, sync::Arc};

use super::{model::UploadFileEntry, DataSource};
use crate::{
    error::{self, Error},
    Result,
};
use async_trait::async_trait;
use rusqlite::Connection;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Clone)]
pub struct SqliteDataSource(Arc<Mutex<Connection>>);

impl SqliteDataSource {
    const UPLOAD_TABLE_NAME: &'static str = "UploadEntriesTable";
    const UPLOAD_T_ID_COL: &'static str = "ID";
    const UPLOAD_T_FILE_NAME_COL: &'static str = "FILE_NAME";

    pub fn new(c: Connection) -> Self {
        Self(Arc::new(Mutex::from(c)))
    }
}

#[async_trait]
impl DataSource for SqliteDataSource {
    async fn init_database(&self) -> Result<()> {
        let stmt = format!("CREATE TABLE IF NOT EXISTS {t_name} ({id_col} TEXT PRIMARY KEY, {fname_col} TEXT NOT NULL);", t_name=Self::UPLOAD_TABLE_NAME, id_col=Self::UPLOAD_T_ID_COL, fname_col=Self::UPLOAD_T_FILE_NAME_COL);

        if let Err(e) = self.0.lock().await.execute(&stmt, ()) {
            return Err(error::Error::DatabaseOperationFailed(e.to_string()));
        };

        Ok(())
    }

    async fn add_file_entry(&self, file_path: &Path) -> Result<Uuid> {
        let id_val = Uuid::now_v7();
        let stmt = format!(
            "INSERT INTO {t_name} ({id_col}, {fname_col}) values (?1, ?2);",
            t_name = Self::UPLOAD_TABLE_NAME,
            id_col = Self::UPLOAD_T_ID_COL,
            fname_col = Self::UPLOAD_T_FILE_NAME_COL,
        );

        if let Err(e) = self
            .0
            .lock()
            .await
            .execute(&stmt, (id_val.to_string(), file_path.to_string_lossy()))
        {
            return Err(error::Error::DatabaseOperationFailed(e.to_string()));
        }

        Ok(id_val)
    }

    async fn remove_file_entry(&self, id: String) -> Result<()> {
        let stmt = format!(
            "DELETE FROM {t_name} where {id_col} = ?1",
            t_name = Self::UPLOAD_TABLE_NAME,
            id_col = Self::UPLOAD_T_ID_COL
        );
        if let Err(e) = self.0.lock().await.execute(&stmt, (id,)) {}

        Ok(())
    }

    async fn get_file_entry(&self, id: String) -> Result<UploadFileEntry> {
        let stmt = format!(
            "SELECT {id_col}, {fname_col} FROM {t_name} where id = ?1;",
            t_name = Self::UPLOAD_TABLE_NAME,
            id_col = Self::UPLOAD_T_ID_COL,
            fname_col = Self::UPLOAD_T_FILE_NAME_COL
        );

        return match self.0.lock().await.query_row(&stmt, (&id,), |row| {
            Ok((row.get::<usize, String>(0), row.get::<usize, String>(1)))
        }) {
            Err(e) => Err(Error::DatabaseOperationFailed(e.to_string())),
            Ok(v) => {
                if v.0.is_err() || v.1.is_err() {
                    Err(Error::NoEntryFound(id))
                } else {
                    let id = v.0.unwrap();
                    let file_path = v.1.unwrap();
                    Ok(UploadFileEntry { id, file_path })
                }
            }
        };
    }
}
