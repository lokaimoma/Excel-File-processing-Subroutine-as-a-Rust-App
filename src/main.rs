use std::path::MAIN_SEPARATOR;

#[tokio::main]
async fn main() {
    let f_appender = tracing_appender::rolling::hourly(format!(".{MAIN_SEPARATOR}"), "server.log");
    let (non_blk, _guard) = tracing_appender::non_blocking(f_appender);
    tracing_subscriber::fmt::fmt()
        .with_env_filter("axum_starter=trace")
        .event_format(tracing_subscriber::fmt::format().pretty())
        .with_writer(non_blk)
        .init();
    let app = axum_starter::get_app_router().await.unwrap();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();

    // let file = reader::xlsx::read(r#".\data\0329 giao hang.xlsx"#).unwrap();
    // let sheet = file.get_sheet(&0usize).unwrap();
    // let a = sheet.get_cell_collection()[0];
    // println!("{:?}", sheet.get_highest_column());
    // println!("{:?}", sheet.get_highest_column_and_row());
    // println!("{:?}", sheet.get_value((10, 1)));
    // println!("{:?}", a.get_cell_value());
    // println!("{}", "01".parse::<u32>().unwrap());
    // println!("{}", "hello".find("llo").unwrap());
    // println!("{}", &"hello".to_string()[0..=0]);
    // for i in 1..0 {
    // println!("{}", i);
    // }
    // println!("{}", (1..=3).contains(&3));
    // let mut a = String::from("hello");
    // a.replace_range(1..=1, "k");
    // println!("{}", a);
    // let html_string = format!(
    // r##"<font color="{clr}">{txt}</font><font color="#ff0000">Another text</font>"##,
    // clr = "#000000",
    // txt = "The text"
    // );
    // println!(
    // "{:?}",
    // umya_spreadsheet::helper::html::html_to_richtext(&html_string).unwrap()
    // );
}
