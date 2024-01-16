use serde::Serialize;

#[derive(Serialize)]
pub struct UploadFileEntry {
    pub id: String,
    #[serde(skip)]
    pub file_path: String,
}
