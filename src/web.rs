use crate::{
    data::{
        model::{JobDetails, RowsPayload, UploadFileEntry},
        sqlite_ds::SqliteDataSource,
        DataSource,
    },
    error::Error,
    Result,
};
use aho_corasick::AhoCorasick;
use axum::{
    extract::{Multipart, State},
    routing::post,
    Json, Router,
};
use calamine::{open_workbook_auto, DataType, Reader};
use serde_json::json;
use serde_json::Value;
use std::{
    ops::{Range, RangeInclusive},
    path::PathBuf,
};
use tokio::fs;
use tracing::{event, instrument, Level};
use umya_spreadsheet::reader;

const DATA_DIR: &str = ".\\data";

pub fn get_routes(datasource: SqliteDataSource) -> Router {
    Router::new()
        .route("/upload", post(upload_file))
        .route("/getHeader", post(get_header_row))
        .route("/runJob", post(run_job))
        .with_state(datasource)
}

#[instrument]
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
    let mut spreedsheet = main_file.unwrap();
    let first_sheet = spreedsheet.get_sheet_mut(&0usize);
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

    let mut contraction_f_path = PathBuf::from(DATA_DIR);
    let contraction_f_name = format!("contraction_{}.xlsx", uuid::Uuid::now_v7());
    contraction_f_path.push(contraction_f_name);
    let contraction_str: Vec<String> =
        get_contraction_texts(&job_detail, contraction_f_path).await?;

    let final_file_path = get_contracted_data(
        last_col_idx,
        last_row_idx,
        first_sheet,
        &job_detail,
        &contraction_str,
    )?;

    todo!()
}

#[derive(Clone, Copy, Debug)]
struct FoundSubTextPosInfo {
    start_idx: usize,
    end_idx: usize,
    index_in_search_vec: usize,
}

fn get_contracted_data(
    last_col_idx: u32,
    last_row_idx: u32,
    work_sheet: &mut umya_spreadsheet::Worksheet,
    job_detail: &JobDetails,
    contraction_str: &[String],
) -> Result<PathBuf> {
    let search_terms = job_detail.search_terms();
    for col_idx in 1..=last_col_idx {
        for row_idx in 2..=last_row_idx {
            let cell = work_sheet.get_cell_mut((col_idx, row_idx));
            let cell_text = cell.get_value();

            //  aho_corsick
            let ac = AhoCorasick::new(search_terms).unwrap();
            event!(
                Level::TRACE,
                message = "Searching cell value for search patterns",
                cell_value = cell_text.as_ref(),
                search_patterns = format!("{:?}", search_terms)
            );
            let search_findings: Vec<FoundSubTextPosInfo> = ac
                .find_overlapping_iter(cell_text.as_ref())
                .map(|finding| FoundSubTextPosInfo {
                    start_idx: finding.start(),
                    end_idx: finding.end() - 1,
                    index_in_search_vec: finding.pattern().as_usize(),
                })
                .collect();
            event!(
                Level::DEBUG,
                message = "Done searching",
                findings = format!("{:?}", search_findings)
            );

            // let mut search_findings: Vec<FoundSubTextPosInfo> = Vec::new();
            // for (idx, term) in search_terms.iter().enumerate() {
            //     if cell_text.contains(term) {
            //         let start_idx = cell_text.find(term).unwrap();
            //         let end_idx = term.len() - 1;
            //         search_findings.push(FoundSubTextPosInfo {
            //             start_idx,
            //             end_idx,
            //             index_in_search_vec: idx,
            //         });
            //     }
            // }
            // finding overlapping

            event!(
                Level::TRACE,
                "Searching for overlaps in search findings, and recalculating their start and end"
            );
            let new_search_findings = Vec::clone(&search_findings);
            for i in 0..search_findings.len() {
                let f1 = search_findings[i];
                for j in i + 1..search_findings.len() {
                    let f2 = search_findings[j];
                    let mut new_f2 = new_search_findings[j];
                    let range = f1.start_idx..=f1.end_idx;
                    if range.contains(&f2.start_idx) {
                        if range.contains(&f2.end_idx) {
                            new_f2.end_idx = 0;
                            new_f2.start_idx = 0;
                            continue;
                        }
                        let new_start = f1.end_idx + 1;
                        if new_start > new_f2.start_idx {
                            new_f2.start_idx = new_start;
                        }
                    }
                }
                event!(
                    Level::DEBUG,
                    message = format!("Iter {}", i),
                    new_search_findings = format!("{:?}", new_search_findings)
                );
            }
            event!(Level::TRACE, "Removing findings with start and end = 0");
            let new_search_findings: Vec<FoundSubTextPosInfo> = new_search_findings
                .into_iter()
                .filter(|finding| finding.start_idx != 0 && finding.end_idx != 0)
                .collect();
            event!(
                Level::DEBUG,
                message = "Final findings",
                findings = format!("{:?}", new_search_findings)
            );

            let mut cell_text = cell_text.to_string();
            for finding in new_search_findings {
                cell_text.replace_range(
                    finding.start_idx..=finding.end_idx,
                    &format!(
                        r##"<font color="{txtcolor}">{value}</font>"##,
                        txtcolor = "RED",
                        value = search_terms[finding.index_in_search_vec]
                    ),
                );
            }
        }
    }
    todo!()
}

async fn get_contraction_texts(
    job_detail: &JobDetails,
    contraction_f_path: PathBuf,
) -> Result<Vec<String>> {
    let mut contraction_str: Vec<String> = Vec::new();
    if job_detail.contraction_file().is_some() {
        if let Err(e) = fs::write(
            &contraction_f_path,
            job_detail.contraction_file().as_ref().unwrap().as_ref(),
        )
        .await
        {
            return Err(Error::IOError(format!(
                "Error writing contraction file to disk, {}",
                e.to_string()
            )));
        };

        let contraction_wkbook = reader::xlsx::read(&contraction_f_path);

        if contraction_wkbook.is_err() {
            return Err(Error::IOError(
                contraction_wkbook.err().unwrap().to_string(),
            ));
        }

        let contraction_wkbook = contraction_wkbook.unwrap();
        let first_contra_sheet = contraction_wkbook.get_sheet(&0usize);
        if first_contra_sheet.is_err() {
            return Err(Error::Generic(format!(
                "Contraction file contains no sheet: {}",
                first_contra_sheet.err().unwrap()
            )));
        }
        let first_contra_sheet = first_contra_sheet.unwrap();
        let (max_col, max_row) = first_contra_sheet.get_highest_column_and_row();
        for col_idx in 1..=max_col {
            for row_idx in 2..=max_row {
                let cell_text = first_contra_sheet.get_value((col_idx, row_idx));
                if !cell_text.trim().is_empty() {
                    contraction_str.push(cell_text.trim().into());
                }
            }
        }
        let _ = fs::remove_file(contraction_f_path).await;
    }
    Ok(contraction_str)
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
