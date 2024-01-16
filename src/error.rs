use axum::{
    body::Body,
    http::{HeaderName, Response, StatusCode},
    response::IntoResponse,
};
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
    #[error("No entry found with the id {0}")]
    NoEntryFound(String),
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .header("Content-Type", "text/plain")
            .body(Body::from(format!("{self}")))
            .unwrap()
    }
}
