use std::path::MAIN_SEPARATOR;

#[tokio::main]
async fn main() {
    let f_appender = tracing_appender::rolling::hourly(format!(".{MAIN_SEPARATOR}"), "server.log");
    let (non_blk, _guard) = tracing_appender::non_blocking(f_appender);
    tracing_subscriber::fmt::fmt()
        .with_env_filter("excel_app=error")
        .event_format(tracing_subscriber::fmt::format().pretty())
        .with_writer(non_blk)
        .init();
    let app = excel_app::get_app_router().await.unwrap();

    println!("Running on http://127.0.0.1:6070");
    println!("Swagger ui at http://127.0.0.1:6070/swagger-ui");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:6070")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
