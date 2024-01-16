use axum::{http::StatusCode, response::IntoResponse};
use thiserror::Error;

#[derive(Debug, Error, Clone)]
pub enum Error {
    #[error("Error parsing multipart data: {0}")]
    UploadFailed(String),
    #[error("Expected a file but none was uploaded")]
    NoFileUploaded,
    #[error("Error writing uploaded file {0} to disk")]
    WritingToDisk(String),
    #[error("Database Error: {0}")]
    DatabaseOperationFailed(String),
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        let mut response = StatusCode::INTERNAL_SERVER_ERROR.into_response();
        response.extensions_mut().insert(self);
        response
    }
}
