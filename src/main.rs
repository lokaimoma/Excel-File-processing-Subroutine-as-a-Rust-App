use umya_spreadsheet::reader;

#[tokio::main]
async fn main() {
    // tracing_subscriber::fmt::init();
    // let app = axum_starter::get_app_router().await.unwrap();

    // let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
    //     .await
    //     .unwrap();
    // axum::serve(listener, app).await.unwrap();
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
    println!("{}", (1..=3).contains(&3));
}
