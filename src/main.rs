use std::path::MAIN_SEPARATOR;

#[tokio::main]
async fn main() {
    let f_appender = tracing_appender::rolling::hourly(format!(".{MAIN_SEPARATOR}"), "server.log");
    let (non_blk, _guard) = tracing_appender::non_blocking(f_appender);
    tracing_subscriber::fmt::fmt()
        .with_env_filter("axum_starter=error")
        .event_format(tracing_subscriber::fmt::format().pretty())
        .with_writer(non_blk)
        .init();
    let app = axum_starter::get_app_router().await.unwrap();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:6070")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
