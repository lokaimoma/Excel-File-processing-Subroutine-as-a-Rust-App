use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct UploadFileEntry {
    pub id: String,
    #[serde(skip)]
    pub file_path: String,
}

#[derive(Serialize)]
pub struct RowsPayload {
    pub rows: Vec<String>,
}
