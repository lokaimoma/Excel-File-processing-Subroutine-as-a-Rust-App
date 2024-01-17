use crate::{
    data::{model::UploadFileEntry, sqlite_ds::SqliteDataSource, DataSource},
    error::Error,
};
use axum::{
    extract::{Multipart, State},
    routing::post,
    Json, Router,
};
use serde_json::json;
use serde_json::Value;
use std::path::PathBuf;
use tokio::fs;
use xlsxwriter;

pub fn get_routes(datasource: SqliteDataSource) -> Router {
    Router::new()
        .route("/upload", post(upload_file))
        .with_state(datasource)
}

async fn upload_file(
    datasource: State<SqliteDataSource>,
    mut multipart: Multipart,
) -> Result<Json<Value>, Error> {
    let result = multipart.next_field().await;

    if result.is_err() {
        return Err(Error::UploadFailed(result.err().unwrap().body_text()));
    }

    let field = result.unwrap();

    if field.is_none() {
        return Err(Error::NoFileUploaded);
    }

    let field = field.unwrap();

    let fname = field.file_name();
    if fname.is_none() {
        return Err(Error::NoFileUploaded);
    }
    let fname = fname.unwrap().to_string();
    let bytes = field.bytes().await.unwrap();

    let mut file_path = PathBuf::from(".\\data");
    if !file_path.exists() {
        let _ = fs::create_dir_all(&file_path).await;
    }
    file_path.push(&fname);

    if let Err(e) = fs::write(&file_path, bytes).await {
        println!("Error writing file");
        eprintln!("{} : {:?}", e, file_path);
        return Err(Error::WritingToDisk(fname));
    };

    let id = datasource.add_file_entry(&file_path).await;

    if id.is_err() {
        return Err(id.err().unwrap());
    }

    let f_entry = UploadFileEntry {
        id: id.unwrap().into(),
        file_path: file_path.to_string_lossy().to_string(),
    };

    if let Err(e) = xlsxwriter::Workbook::new(file_path) {
        fs::remove_file(file_path).await;
        return Err(Error::InValidXLSXFIle(e.to_string()));
    };

    Ok(Json(json!(f_entry)))
}
