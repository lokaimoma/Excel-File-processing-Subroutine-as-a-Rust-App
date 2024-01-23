use axum::{
    body::Body,
    http::{Response, StatusCode},
    response::IntoResponse,
};
use thiserror::Error;
use utoipa::ToSchema;

#[derive(Debug, Error, Clone, ToSchema)]
pub enum Error {
    #[error("Error parsing multipart data: {0}")]
    MultipartFormError(String),
    #[error("Expected a file but none was uploaded or file was corrupted")]
    NoFileUploaded,
    #[error("Error writing uploaded file {0} to disk")]
    WritingToDisk(String),
    #[error("Database Error: {0}")]
    DatabaseOperationFailed(String),
    #[error("No entry found with the id {0}")]
    NoEntryFound(String),
    #[error("Invalid Excel file: {0}")]
    InValidExcelFile(String),
    #[error("Invalid payload: {0}")]
    InvalidPayload(String),
    #[error("IO Error: {0}")]
    IOError(String),
    #[error("{0}")]
    Generic(String),
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

impl From<axum::extract::multipart::MultipartError> for Error {
    fn from(value: axum::extract::multipart::MultipartError) -> Self {
        Self::MultipartFormError(
            format!("Error parsing multipart formdata: {}", value.to_string()).to_string(),
        )
    }
}
