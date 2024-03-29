use axum::Router;
use std::path::MAIN_SEPARATOR;

use axum::{extract::DefaultBodyLimit, http::Method};
use data::{sqlite_ds::SqliteDataSource, DataSource};
use rusqlite::Connection;
use tokio::fs;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
};

mod colors;
mod data;
pub mod error;
mod web;

const DATA_DIR_NAME: &str = "data_";

pub type Result<T> = std::result::Result<T, error::Error>;

pub async fn get_app_router() -> Result<Router> {
    fs::create_dir_all(format!(".{MAIN_SEPARATOR}{DATA_DIR_NAME}"))
        .await
        .unwrap();
    let sqlite_con = Connection::open(format!(
        ".{MAIN_SEPARATOR}{DATA_DIR_NAME}{MAIN_SEPARATOR}db.sqlite"
    ))
    .unwrap();
    let datasource = SqliteDataSource::new(sqlite_con);
    datasource.init_database().await?;
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(Any);

    Ok(Router::new()
        .merge(crate::web::get_routes(datasource))
        .nest_service("/", ServeDir::new(format!(".{MAIN_SEPARATOR}frontend")))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(cors)
                .layer(DefaultBodyLimit::max(10000000)),
        ))
}
