use axum::{
    extract::Multipart,
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};

use tokio::fs;
use tower_http::trace::TraceLayer;

use axum_starter::error::Error;
use std::path::PathBuf;

async fn upload_file(mut multipart: Multipart) -> Result<Json<String>, Error> {
    println!("ENtered herer");
    let result = multipart.next_field().await;

    if result.is_err() {
        return Err(Error::UploadFailed(result.err().unwrap().body_text()));
    }

    let field = result.unwrap();

    println!("Successfully unwraped field");

    if field.is_none() {
        return Err(Error::NoFileUploaded);
    }

    let field = field.unwrap();

    let fname = field.file_name().unwrap_or("no_file_name.xlsx").to_string();
    let bytes = field.bytes().await.unwrap();

    println!("Successfully got file name and bytes");

    let mut path = PathBuf::from(".\\data");
    if !path.exists() {
        let _ = fs::create_dir_all(&path).await;
    }
    path.push(&fname);

    if let Err(e) = fs::write(&path, bytes).await {
        println!("Error writing file");
        eprintln!("{} : {:?}", e, path);
        return Err(Error::WritingToDisk(fname));
    };

    Ok(Json("{'success': 'File stored successfully'}".into()))
}

async fn hello() -> impl IntoResponse {
    Html("Hello welcome to this server")
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let app = Router::new()
        .route("/upload", post(upload_file))
        .route("/hello", get(hello))
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
