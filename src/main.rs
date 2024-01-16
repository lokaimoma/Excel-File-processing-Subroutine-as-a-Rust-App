#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let app = axum_starter::get_app_router().await.unwrap();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
