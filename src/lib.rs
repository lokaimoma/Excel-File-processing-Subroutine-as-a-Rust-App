use axum::Router;

use data::{sqlite_data_Source::SqliteDataSource, DataSource};
use rusqlite::Connection;
use tower_http::trace::TraceLayer;

pub mod data;
pub mod error;
pub mod web;

pub type Result<T> = std::result::Result<T, error::Error>;

pub async fn get_app_router() -> Result<Router> {
    let sqlite_con = Connection::open(".\\data\\db.sqlite").unwrap();
    let datasource = SqliteDataSource::new(sqlite_con);
    datasource.init_database().await?;

    Ok(Router::new()
        .merge(crate::web::get_routes(datasource))
        .layer(TraceLayer::new_for_http()))
}
