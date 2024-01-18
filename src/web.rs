use crate::{
    data::{
        model::{JobDetails, RowsPayload, UploadFileEntry},
        sqlite_ds::SqliteDataSource,
        DataSource,
    },
    error::Error,
    Result,
};
use axum::{
    extract::{Multipart, State},
    routing::post,
    Json, Router,
};
use calamine::{open_workbook_auto, DataType, Reader};
use serde_json::json;
use serde_json::Value;
use std::path::PathBuf;
use tokio::fs;
use umya_spreadsheet::reader;

const DATA_DIR: &str = ".\\data";

pub fn get_routes(datasource: SqliteDataSource) -> Router {
    Router::new()
        .route("/upload", post(upload_file))
        .route("/getHeader", post(get_header_row))
        .route("/runJob", post(run_job))
        .with_state(datasource)
}

async fn run_job(
    State(datasource): State<SqliteDataSource>,
    multipart: Multipart,
) -> Result<Json<Value>> {
    let job_detail = JobDetails::try_from(multipart).await?;
    let main_file = datasource
        .get_file_entry(job_detail.file_id().to_owned())
        .await?;
    let main_file = reader::xlsx::read(main_file.file_path);
    if main_file.is_err() {
        return Err(Error::IOError(main_file.err().unwrap().to_string()));
    }
    let spreedsheet = main_file.unwrap();
    let first_sheet = spreedsheet.get_sheet(&0usize);
    if first_sheet.is_err() {
        return Err(Error::InValidExcelFile(first_sheet.err().unwrap().into()));
    }
    let first_sheet = first_sheet.unwrap();

    let (first_col_idx, first_row_idx) = (1, 1);
    let (last_col_idx, last_row_idx) = first_sheet.get_highest_column_and_row();

    validate_sheet(
        first_col_idx,
        last_col_idx,
        first_sheet,
        first_row_idx,
        &job_detail,
        last_row_idx,
    )?;

    todo!()
}

fn validate_sheet(
    first_col_idx: u32,
    last_col_idx: u32,
    first_sheet: &umya_spreadsheet::Worksheet,
    first_row_idx: u32,
    job_detail: &JobDetails,
    last_row_idx: u32,
) -> Result<()> {
    // verify header row has no empty values
    for col_idx in first_col_idx..=last_col_idx {
        let row_val = first_sheet.get_value((col_idx, first_row_idx));
        if row_val.trim().is_empty() {
            return Err(Error::Generic("Incomplete title bar".into()));
        }
    }

    // verify cols with date
    for col_idx in job_detail.check_date_cols() {
        for row_idx in first_row_idx + 1..=last_row_idx {
            let value = first_sheet.get_value((col_idx, &row_idx));
            if value.len() < 6 {
                return Err(Error::Generic(format!(
                    "Invalid date value at column: {}, row: {}",
                    col_idx, row_idx
                )));
            }
            let month = &value[0..2];
            let month_val = month.parse::<u32>();
            if month_val.is_err() {
                return Err(Error::Generic(format!(
                    "Invalid month value for date field. Value = {}, column: {}, row: {}",
                    month, col_idx, row_idx
                )));
            }
            let month_val = month_val.unwrap();
            if month_val < 1 || month_val > 12 {
                return Err(Error::Generic(format!(
                    "Invalid month value for date field. Value = {}, column: {}, row: {}",
                    month, col_idx, row_idx
                )));
            }
            let day = &value[2..4];
            let day_val = day.parse::<u32>();
            if day_val.is_err() {
                return Err(Error::Generic(format!(
                    "Invalid day value for date field. Value = {}, column: {}, row: {}",
                    day, col_idx, row_idx
                )));
            }
            let day_val = day_val.unwrap();
            if day_val < 1 || day_val > 31 {
                return Err(Error::Generic(format!(
                    "Invalid day value for date field. Value = {}, column: {}, row: {}",
                    day, col_idx, row_idx
                )));
            }
            let year = &value[4..6];
            let year_val = year.parse::<u32>();
            if year_val.is_err() {
                return Err(Error::Generic(format!(
                    "Invalid year value for date field. Value = {}, column: {}, row: {}",
                    year, col_idx, row_idx
                )));
            }
        }
    }
    Ok(())
}

async fn get_header_row(
    State(datasource): State<SqliteDataSource>,
    Json(partial_f_entry): Json<UploadFileEntry>,
) -> Result<Json<Value>> {
    let result = match datasource.get_file_entry(partial_f_entry.id).await {
        Ok(r) => r,
        Err(e) => return Err(Error::DatabaseOperationFailed(e.to_string())),
    };

    let mut workbook = match open_workbook_auto(result.file_path) {
        Err(e) => return Err(Error::IOError(e.to_string())),
        Ok(wb) => wb,
    };

    let first_sheet = workbook.worksheet_range_at(0);

    if first_sheet.is_none() {
        return Err(Error::Generic("No sheet found in excel file".into()));
    }

    let rows_data = match first_sheet.unwrap() {
        Err(e) => return Err(Error::Generic(e.to_string())),
        Ok(v) => v,
    };

    let rows: Vec<String> = rows_data
        .rows()
        .take(1)
        .flat_map(|r| {
            let mut data = Vec::new();
            for c in r {
                let s = match c {
                    DataType::String(s) => s.to_owned(),
                    DataType::Int(i) => i.to_string(),
                    DataType::Float(f) => f.to_string(),
                    DataType::Bool(b) => b.to_string(),
                    DataType::DateTime(d) => d.to_string(),
                    DataType::Duration(d) => d.to_string(),
                    DataType::DateTimeIso(dt) => dt.to_string(),
                    DataType::DurationIso(dt) => dt.to_string(),
                    DataType::Error(e) => e.to_string(),
                    DataType::Empty => "".to_owned(),
                };
                data.push(s);
            }
            data
        })
        .collect();

    let rows = RowsPayload { rows };
    let rows = json!(rows);

    Ok(Json(rows))
}

async fn upload_file(
    State(datasource): State<SqliteDataSource>,
    mut multipart: Multipart,
) -> Result<Json<Value>> {
    let result = multipart.next_field().await;

    if result.is_err() {
        return Err(Error::MultipartFormError(result.err().unwrap().body_text()));
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

    let mut file_path = PathBuf::from(DATA_DIR);
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

    if let Err(e) = open_workbook_auto(&file_path) {
        let _ = fs::remove_file(file_path).await;
        return Err(Error::InValidExcelFile(e.to_string()));
    };

    Ok(Json(json!(f_entry)))
}
