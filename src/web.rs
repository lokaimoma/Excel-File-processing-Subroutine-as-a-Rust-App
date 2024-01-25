use crate::{
    colors::{self, CellColorProfile},
    data::{
        model::{JobDetails, RowsPayload, SortInfo, UploadFileEntry, ExcelFileForm, RunJobRequest, RunJobResponse},
        sqlite_ds::SqliteDataSource,
        DataSource,
    },
    error::Error,
    Result as CrateRes, DATA_DIR_NAME,
};
use aho_corasick::AhoCorasick;
use axum::{
    body::{self, Bytes},
    extract::{Multipart, Path, State},
    http::{header::{CONTENT_DISPOSITION, CONTENT_TYPE, HeaderMap}, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use serde_json::Value;
use std::{
    collections::HashMap,
    path::{PathBuf, MAIN_SEPARATOR},
};
use tokio::fs;
use tokio_util::io::ReaderStream;
use tracing::{event, instrument, Level};
use umya_spreadsheet::{reader, writer, Cell};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use chrono::Local;

#[derive(OpenApi)]
#[openapi(
    paths(get_header_row, upload_file, run_job),
    components(
        schemas(UploadFileEntry),
        schemas(RowsPayload),
        schemas(ExcelFileForm),
        schemas(Error),
        schemas(RunJobRequest),
        schemas(RunJobResponse),
    )
)]
pub struct APIDoc;


pub fn get_routes(datasource: SqliteDataSource) -> Router {
    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", APIDoc::openapi()))
        .route("/upload", post(upload_file))
        .route("/getHeader/:entry_uuid", get(get_header_row))
        .route("/runJob", post(run_job))
        .with_state(datasource)
}

fn get_cells(
    work_sheet: &mut umya_spreadsheet::Worksheet,
    last_row_idx: usize,
    last_col_idx: usize,
) -> Vec<Vec<Cell>> {
    let mut result: Vec<Vec<Cell>> = Vec::new();
    for row in 2..=last_row_idx {
        let mut cur_row: Vec<Cell> = Vec::new();
        for col in 1..=last_col_idx {
            cur_row.push(
                work_sheet
                    .get_cell((col as u32, row as u32))
                    .unwrap()
                    .to_owned(),
            )
        }
        result.push(cur_row);
    }
    result
}

fn sort_cells(cells: &mut [Vec<Cell>], sort_infos: &[SortInfo]) {
    event!(Level::TRACE, "Sorting cells");
    if sort_infos.is_empty() {
        event!(Level::TRACE, "No columns to sort");
        return;
    }

    let mut sortable_rows: HashMap<String, Vec<usize>> = HashMap::new();
    let sort_info = &sort_infos[0];
    let mut col_idx: usize = 0;

    sort_cells_by_range(cells, 0..=(cells.len() - 1), sort_info, &mut col_idx);
    event!(Level::TRACE, "First sort done...");

    let sort_infos = sort_infos.iter().skip(1).collect::<Vec<_>>();
    sort_infos.iter().enumerate().for_each(|(i, sort_info)| {
        event!(Level::TRACE, "Finding sortable rows");
        clear_build_sortable_rows(cells, col_idx, &mut sortable_rows);
        event!(Level::TRACE, "Sortable rows found");

        for row_range in sortable_rows
            .values()
            .filter(|r| r.len() > 1)
            .map(|row| row[0]..=row[row.len() - 1])
        {
            event!(Level::TRACE, "Performing sub sort in iter #{}", i);
            sort_cells_by_range(cells, row_range, sort_info, &mut col_idx);
            event!(Level::TRACE, "Sub sort in iter #{} done", i);
        }
    });
    
    event!(Level::TRACE, "Done sorting...");
}

#[inline(always)]
fn sort_cells_by_range(
    cells: &mut [Vec<Cell>],
    row_range: std::ops::RangeInclusive<usize>,
    sort_info: &SortInfo,
    col_idx: &mut usize,
) {
    cells[row_range].sort_unstable_by(|s1, s2| match sort_info {
        crate::data::model::SortInfo::Asc { column_index } => {
            // The column index we are receiving from the user
            // doesn't start counting from 0, hence the -1 here
            *col_idx = (column_index.to_owned() - 1) as usize;
            return s1[*col_idx]
                .get_value()
                .as_ref()
                .partial_cmp(s2[*col_idx].get_value().as_ref())
                .unwrap();
        }
        crate::data::model::SortInfo::Desc { column_index } => {
            // The column index we are receiving from the user
            // doesn't start counting from 0, hence the -1 here
            *col_idx = (column_index.to_owned() - 1) as usize;
            return s2[*col_idx]
                .get_value()
                .as_ref()
                .partial_cmp(s1[*col_idx].get_value().as_ref())
                .unwrap();
        }
    });
}

#[inline(always)]
fn clear_build_sortable_rows(
    cells: &mut [Vec<Cell>],
    col_idx: usize,
    sortable_rows: &mut HashMap<String, Vec<usize>>,
) {
    for (idx, row) in cells.iter().enumerate() {
        let key = row[col_idx].get_value();
        let value = sortable_rows.get_mut(key.as_ref());
        if value.is_some() {
            let value = value.unwrap();
            value.push(idx);
        } else {
            sortable_rows.insert(key.to_string(), vec![idx]);
        }
    }
}

#[utoipa::path(
    post,
    path = "/runJob",
    responses(
        (status = 200, body=RunJobResponse, description="Contraction excel file to download"),
        (status = 501, body=Error, description="An error message")
    ),
    request_body(
        content = RunJobRequest, content_type = "multipart/form-data"
    )
)]
#[instrument]
async fn run_job(
    State(datasource): State<SqliteDataSource>,
    multipart: Multipart,
) -> impl IntoResponse {
    let mut job_detail = JobDetails::try_from(multipart).await?;
    event!(Level::DEBUG, "Job details: {:?}", job_detail);
    let file_entry = datasource
        .get_file_entry(job_detail.file_id().to_owned())
        .await?;
    let spreadsheet = reader::xlsx::read(&file_entry.file_path);
    if spreadsheet.is_err() {
        return Err(Error::IOError(spreadsheet.err().unwrap().to_string()));
    }
    let mut spreadsheet = spreadsheet.unwrap();
    let worksheet = spreadsheet.get_sheet_mut(&0usize);
    if worksheet.is_err() {
        return Err(Error::InValidExcelFile(worksheet.err().unwrap().into()));
    }
    let worksheet = worksheet.unwrap();

    let (first_col_idx, first_row_idx) = (1, 1);
    let (last_col_idx, last_row_idx) = worksheet.get_highest_column_and_row();

    event!(Level::TRACE, "Validating sheet");
    validate_sheet(
        first_col_idx,
        last_col_idx,
        worksheet,
        first_row_idx,
        &job_detail,
        last_row_idx,
    )?;
    event!(Level::TRACE, "Sheet is valid");

    let contraction_f_bytes = job_detail.pop_contraction_file();

    let contraction_task = tokio::spawn(async move {
        let mut contraction_f_path = PathBuf::from(format!(".{MAIN_SEPARATOR}{DATA_DIR_NAME}"));
        let contraction_f_name = format!("contraction_{}.xlsx", uuid::Uuid::now_v7());
        contraction_f_path.push(contraction_f_name);
        get_contraction_texts(contraction_f_bytes, &contraction_f_path).await
    });

    event!(Level::TRACE, "Copying cell values into Vec<Vec<Cell>>");
    let mut cells = get_cells(worksheet, last_row_idx as usize, last_col_idx as usize);
    if cells.len() > 1 {
        event!(
            Level::TRACE,
            "Done copying, Row count: {}, expected: {}. Col count: {}, expected: {}",
            cells.len(),
            // Minus header row
            last_row_idx - 1,
            cells[0].len(),
            last_col_idx
        );
    } else {
        event!(
            Level::TRACE,
            "No values in sheet to copy, Row count: {}, Col count: {}",
            last_row_idx,
            last_col_idx
        );
    }

    sort_cells(cells.as_mut_slice(), job_detail.sort_infos());

    let contraction_str = contraction_task.await.unwrap()?;

    event!(Level::TRACE, "Highlighting search terms and contractions");
    highlight_search_terms_and_contractions(cells.as_mut_slice(), &job_detail, &contraction_str)?;
    event!(
        Level::TRACE,
        "Done highlighting search terms and contractions"
    );

    // We read the cells from row 2 and col 1
    // Hence we have to offset the indexes below
    // by them, to set them at the right place.
    event!(Level::TRACE, "Mutating spreadsheet");
    const ROW_OFFSET: usize = 2;
    const COL_OFFSET: usize = 1;
    cells.into_iter().enumerate().for_each(|(row_idx, row)| {
        row.into_iter()
            .enumerate()
            .for_each(|(col_idx, mut col_cell)| {
                let coordinate = col_cell.get_coordinate_mut();
                coordinate.set_row_num((row_idx + ROW_OFFSET) as u32);
                coordinate.set_col_num((col_idx + COL_OFFSET) as u32);
                worksheet.set_cell(col_cell);
            });
    });
    event!(Level::TRACE, "Done mutating spreadsheet");

    let final_f_name = format!("contraction_{}.xlsx", uuid::Uuid::now_v7());
    let mut contraction_f_path = PathBuf::from(format!(".{MAIN_SEPARATOR}{DATA_DIR_NAME}"));
    contraction_f_path.push(final_f_name);

    event!(Level::TRACE, "Saving final contraction file....");

    if let Err(e) = writer::xlsx::write(&spreadsheet, &contraction_f_path) {
        event!(
            Level::ERROR,
            message = "Error writing contraction file",
            error = e.to_string()
        );
        return Err(Error::IOError(e.to_string()));
    };
    event!(Level::TRACE, "File saved successfully");

    let file = fs::File::open(contraction_f_path).await.unwrap();
    let stream = ReaderStream::new(file);
    let stream = body::Body::from_stream(stream);

    let mut headers = HeaderMap::new();
    let file_name = file_entry.file_path;
    let last_slash_pos = file_name.rfind(MAIN_SEPARATOR);
    let file_name = &file_name[last_slash_pos.unwrap_or(0)+1..];
    let full_stop_pos = file_name.rfind('.');
    let full_stop_pos = full_stop_pos.unwrap_or(file_name.len());
    let file_name = &file_name[0..full_stop_pos];
    let dt = Local::now();
    let formatted_dt = format!("{}", dt.format("%m%d%Y%H%M"));
    
    headers.insert(CONTENT_TYPE, "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".parse().unwrap());
    headers.insert(CONTENT_DISPOSITION, format!("attachment; filename=\"{file_name} basic process-{formatted_dt}\"").parse().unwrap());

    Ok((headers, stream))
}

#[derive(Clone, Copy, Debug)]
struct FoundSubTextPosInfo {
    start_idx: usize,
    end_idx: usize,
}

fn highlight_search_terms_and_contractions(
    cells: &mut [Vec<Cell>],
    job_detail: &JobDetails,
    contraction_str: &[String],
) -> CrateRes<()> {
    let default_color: colors::White = colors::White { color_pool_pos: 0 };
    let black: colors::Black = colors::Black { color_pool_pos: 0 };
    let yellow: colors::Yellow = colors::Yellow { color_pool_pos: 0 };
    let beige: colors::Beige = colors::Beige { color_pool_pos: 0 };
    let lavender: colors::Lavender = colors::Lavender { color_pool_pos: 0 };
    let navy_blue: colors::NavyBlue = colors::NavyBlue { color_pool_pos: 0 };

    let mut color_profiles: [Box<dyn CellColorProfile>; 6] = [
        Box::from(default_color),
        Box::from(yellow),
        Box::from(beige),
        Box::from(lavender),
        Box::from(navy_blue),
        Box::from(black),
    ];

    let search_terms = job_detail.search_terms();
    let ac = AhoCorasick::new(search_terms).unwrap();

    cells.iter_mut().for_each(|row| {
        row.iter_mut().for_each(|cell| {
            let cell_text = cell.get_value().to_string();
            //  aho_corsick
            event!(
                Level::TRACE,
                message = "Searching cell value for search patterns",
                cell_value = cell_text,
                search_patterns = format!("{:?}", search_terms)
            );
            let mut search_findings: Vec<FoundSubTextPosInfo> = ac
                .find_overlapping_iter(&cell_text)
                .map(|finding| FoundSubTextPosInfo {
                    start_idx: finding.start(),
                    end_idx: finding.end() - 1,
                })
                .collect();

            search_findings.sort_by(|f1, f2| f1.start_idx.partial_cmp(&f2.start_idx).unwrap());

            event!(
                Level::DEBUG,
                message = "Done searching",
                findings = format!("{:?}", search_findings)
            );

            event!(
                Level::TRACE,
                "Searching for overlaps in search findings, and recalculating their start and end"
            );

            let new_search_findings = apply_overlapping_rule(search_findings);

            let mut color_profile: &mut Box<dyn CellColorProfile> = &mut color_profiles[0];
            for (idx, contraction) in contraction_str.iter().enumerate() {
                if cell_text.trim().eq_ignore_ascii_case(contraction) {
                    let color_idx = idx + 1 % color_profiles.len();
                    event!(
                        Level::DEBUG,
                        "Contraction found for cell value={}, choosing color at idx={}",
                        cell_text,
                        color_idx
                    );
                    color_profile = &mut color_profiles[color_idx];
                    break;
                }
            }
            apply_formatting(cell, color_profile, new_search_findings);
        })
    });

    Ok(())
}

#[inline(always)]
fn apply_formatting(
    cell: &mut Cell,
    color_profile: &mut Box<dyn CellColorProfile>,
    new_search_findings: Vec<FoundSubTextPosInfo>,
) {
    let mut cell_text = cell.get_value().to_string();
    let cell_style = cell.get_style_mut();

    event!(Level::TRACE, "Current color profile: {:?}", color_profile);

    cell_style.set_background_color(colors::to_argb(
        &color_profile.as_ref().get_background_color(),
    ));

    let font = cell_style.get_font_mut();
    font.get_color_mut().set_argb(&colors::to_argb(
        &color_profile.as_ref().get_default_text_color(),
    ));

    let mut offset = 0;
    event!(Level::TRACE, "Formating text using finds");
    for mut finding in new_search_findings {
        // Adding the html font tags,etc changes the position of the texts
        // we would have to update the position of the findings.
        // luckily they're in ascending order so we just add offset
        finding.start_idx += offset;
        finding.end_idx += offset;
        cell_text.replace_range(
            finding.start_idx..=finding.end_idx,
            &format!(
                r##"<font color="{txtcolor}"><b>{value}</b></font>"##,
                txtcolor = color_profile.get_color(),
                value = &cell_text[finding.start_idx..=finding.end_idx]
            ),
        );
        // why 29, the new characters added to the old text sum up to 28
        // 29 = html instructions (including the quote surrounding the color hex), color hex = 7
        // The text value is not part, because they've been accounted for already
        offset += 36;
        event!(Level::DEBUG, "New Text: {}", cell_text);
        event!(Level::TRACE, "New offset: {}", offset);
    }
    event!(Level::TRACE, "Setting rich text");
    cell.set_rich_text(umya_spreadsheet::helper::html::html_to_richtext(&cell_text).unwrap());
    color_profile.reset_color_pool_pos();
}

#[inline(always)]
fn apply_overlapping_rule(search_findings: Vec<FoundSubTextPosInfo>) -> Vec<FoundSubTextPosInfo> {
    let mut new_search_findings = Vec::clone(&search_findings);

    for i in 0..search_findings.len() {
        let f1 = search_findings[i];

        event!(Level::TRACE, "At index i={}; Finding = {:?}", i, f1);

        for j in i + 1..search_findings.len() {
            let f2 = search_findings[j];
            let new_f2 = &mut new_search_findings[j];

            event!(
                Level::TRACE,
                "Checking index j={}; Finding = {:?}; New Finding = {:?}",
                j,
                f2,
                new_f2
            );

            let range = f1.start_idx..=f1.end_idx;

            event!(Level::TRACE, "Range of of f1: {:?}", range);

            if range.contains(&f2.start_idx) {
                event!(Level::TRACE, "Start of f2 in f1");

                if range.contains(&f2.end_idx) {
                    event!(Level::TRACE, "End of f2 in f1");

                    new_f2.end_idx = 0;
                    new_f2.start_idx = 0;

                    event!(
                        Level::TRACE,
                        "Final findings f2={:?}; new_f2={:?}",
                        f2,
                        new_f2
                    );
                    continue;
                }

                event!(Level::TRACE, "end of f2 out of f1 range");

                let new_start = f1.end_idx + 1;
                if new_start > new_f2.start_idx && new_start < new_f2.end_idx {
                    new_f2.start_idx = new_start;
                } else {
                    new_f2.start_idx = new_f2.end_idx
                }

                event!(
                    Level::TRACE,
                    "Final findings f2={:?}; new_f2={:?}",
                    f2,
                    new_f2
                );
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
        .filter(|finding| finding.start_idx != 0 || finding.end_idx != 0)
        .collect();
    event!(
        Level::DEBUG,
        message = "Final findings",
        findings = format!("{:?}", new_search_findings)
    );
    new_search_findings
}

async fn get_contraction_texts(
    contraction_f_bytes: Option<Bytes>,
    contraction_f_path: &PathBuf,
) -> CrateRes<Vec<String>> {
    let mut contraction_str: Vec<String> = Vec::new();
    if contraction_f_bytes.is_some() {
        if let Err(e) = fs::write(&contraction_f_path, contraction_f_bytes.unwrap()).await {
            return Err(Error::IOError(format!(
                "Error writing contraction file to disk, {}",
                e
            )));
        };

        let contraction_wkbook = reader::xlsx::read(contraction_f_path);

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
                let cell_text = cell_text.trim();
                if !cell_text.is_empty() {
                    contraction_str.push(cell_text.into());
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
) -> CrateRes<()> {
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
            if !(1..=12).contains(&month_val) {
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
            if !(1..=31).contains(&day_val) {
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

#[utoipa::path(
    get, 
    path = "/getHeader/{entry_uuid}", 
    responses(
        (status = 200, description = "The header row of the excel file, with each string representing a column", body = RowsPayload)
    )
)]
async fn get_header_row(
    State(datasource): State<SqliteDataSource>,
    Path(entry_uuid): Path<String>,
) -> CrateRes<Json<Value>> {
    let result = match datasource.get_file_entry(entry_uuid).await {
        Ok(r) => r,
        Err(e) => return Err(Error::DatabaseOperationFailed(e.to_string())),
    };

    let spreadsheet = match reader::xlsx::read(result.file_path) {
        Err(e) => return Err(Error::InValidExcelFile(e.to_string())),
        Ok(ss) => ss,
    };

    let first_sheet = spreadsheet.get_sheet(&0);

    if first_sheet.is_err() {
        return Err(Error::Generic("No sheet found in excel file".into()));
    }

    let first_sheet = first_sheet.unwrap();

    let mut columns: Vec<String>;
    if first_sheet.get_highest_row() < 1 {
        columns = Vec::with_capacity(0);
    }else {
        let col_count = first_sheet.get_highest_column();
        columns = Vec::with_capacity((col_count - 1) as usize);
        for col_idx in 1..=col_count {
            columns.push(first_sheet.get_value((col_idx, 1)));
        }
    }

    let rows = RowsPayload { columns };
    let rows = json!(rows);

    Ok(Json(rows))
}



#[utoipa::path(
    post,
    path = "/upload",
    request_body(content_type = "multipart/form-data", content = ExcelFileForm),
    responses(
        (status=201, body = UploadFileEntry, description = "id for referencing the uploaded file for subsequent operations"),
        (status=500, body = Error, description = "Error in multipart form data or no file found error")
    )
)]
async fn upload_file(
    State(datasource): State<SqliteDataSource>,
    mut multipart: Multipart,
) -> impl IntoResponse {
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

    let mut file_path = PathBuf::from(format!(".{MAIN_SEPARATOR}{DATA_DIR_NAME}"));
    if !file_path.exists() {
        let _ = fs::create_dir_all(&file_path).await;
    }
    file_path.push(&fname);

    if let Err(e) = fs::write(&file_path, bytes).await {
        println!("Error writing file");
        eprintln!("{} : {:?}", e, file_path);
        return Err(Error::WritingToDisk(fname));
    };

    if let Err(e) = reader::xlsx::read(&file_path) {
        let _ = fs::remove_file(file_path).await;
        return Err(Error::InValidExcelFile(e.to_string()));
    };

    let id = datasource.add_file_entry(&file_path).await;

    if id.is_err() {
        return Err(id.err().unwrap());
    }

    let f_entry = UploadFileEntry {
        id: id.unwrap().into(),
        file_path: file_path.to_string_lossy().to_string(),
    };

    Ok((StatusCode::CREATED ,Json(json!(f_entry))))
}
