use crate::{
    data::{
        model::{RowsPayload, UploadFileEntry},
        sqlite_ds::SqliteDataSource,
        DataSource,
    },
    error::Error,
    Result,
};
use axum::{
    body::Bytes,
    extract::{Multipart, State},
    routing::post,
    Json, Router,
};
use calamine::{open_workbook_auto, DataType, Reader};
use serde_json::json;
use serde_json::Value;
use std::{collections::HashMap, path::PathBuf};
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

enum SortInfo {
    ASC { column_index: u32 },
    DESC { column_index: u32 },
}

impl SortInfo {
    const ASC: &'static str = "asc";
    const DESC: &'static str = "desc";
}

struct JobDetails {
    file_id: String,
    contraction_file: Option<Bytes>,
    search_terms: Vec<String>,
    check_date_cols: Vec<u32>,
    sort_cols_info: Vec<SortInfo>,
}

impl JobDetails {
    const FILE_ID_FIELD_N: &'static str = "fileId";
    const CONTRACTION_F_FIELD_N: &'static str = "contractionFile";
    const SEARCH_TERMS_FIELD_N: &'static str = "searchTerms";
    const CHECK_DATE_FIELD_N: &'static str = "checkDate";
    const SORT_COL_FIELD_N: &'static str = "sortCol";
    const SEARCH_TERM_COUNTER_LIMIT: usize = 5;

    fn sort_infos(&self) -> &Vec<SortInfo> {
        &self.sort_cols_info
    }

    fn file_id(&self) -> &str {
        &self.file_id
    }

    fn contraction_file(&self) -> &Option<Bytes> {
        &self.contraction_file
    }

    fn search_terms(&self) -> &Vec<String> {
        &self.search_terms
    }

    fn check_date_cols(&self) -> &Vec<u32> {
        &self.check_date_cols
    }

    async fn try_from(mut value: Multipart) -> Result<Self> {
        let mut file_id: Option<String> = None;
        let mut contraction_file: Option<Bytes> = None;
        let mut search_terms: Vec<String> = Vec::with_capacity(5);
        let mut check_date_cols: Vec<u32> = Vec::new();
        let mut sor_infos: Vec<SortInfo> = Vec::new();

        let mut search_t_counter = 0;

        while let Some(field) = value.next_field().await? {
            let name = field.name();
            if name.is_none() {
                continue;
            }

            let name = name.unwrap();
            match name {
                JobDetails::FILE_ID_FIELD_N => file_id = Some(name.to_owned()),
                JobDetails::CONTRACTION_F_FIELD_N => {
                    let bytes = field.bytes().await?;
                    contraction_file = Some(bytes);
                }
                JobDetails::SEARCH_TERMS_FIELD_N => {
                    if search_t_counter < JobDetails::SEARCH_TERM_COUNTER_LIMIT {
                        let text = field.text().await?;
                        search_terms.insert(search_t_counter, text.into());
                        search_t_counter += 1;
                    }
                }
                JobDetails::CHECK_DATE_FIELD_N => {
                    let text = field.text().await?;
                    let text = text.trim();
                    let number = text.parse::<u32>();
                    if number.is_err() {
                        return Err(Error::Generic(format!("Invalid column index: {}", text)));
                    }
                    check_date_cols.push(number.unwrap());
                }
                JobDetails::SORT_COL_FIELD_N => {
                    // payload has to be of format ORDER,index
                    // order can be asc / desc (lowercase)
                    let text = field.text().await?;
                    let text = text.trim();
                    let text_parts: Vec<&str> = text.split(",").collect();
                    if text_parts.len() < 2 {
                        return Err(Error::Generic(format!("sortCol data has to be of form order,index Where order can take as value either asc or desc. Got: {}", text)));
                    }
                    let order = text_parts[0];
                    let order = order.to_lowercase();
                    let sort_info: SortInfo;
                    let index = text_parts[1];
                    let index_val = index.parse::<u32>();
                    if index_val.is_err() {
                        return Err(Error::Generic(format!(
                            "Invalid value passed as column index. Got {}, expected a valid number",
                            index
                        )));
                    }
                    match order.as_str() {
                        SortInfo::ASC => {
                            sort_info = SortInfo::ASC {
                                column_index: index_val.unwrap(),
                            }
                        }
                        SortInfo::DESC => {
                            sort_info = SortInfo::DESC {
                                column_index: index_val.unwrap(),
                            }
                        }
                        _ => {
                            return Err(Error::Generic(format!(
                                "Invalid sort order value: Got {}, Expected: asc / desc",
                                order
                            )));
                        }
                    }
                    sor_infos.push(sort_info);
                }
                _ => {}
            }
        }
        if file_id.is_none() {
            return Err(Error::MultipartFormError(
                "fileId not present in formdata".to_string(),
            ));
        }
        Ok(Self {
            file_id: file_id.unwrap(),
            contraction_file,
            check_date_cols,
            search_terms,
            sort_cols_info: sor_infos,
        })
    }
}

async fn run_job(
    State(datasource): State<SqliteDataSource>,
    mut multipart: Multipart,
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
