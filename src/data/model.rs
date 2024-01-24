use crate::Result;
use axum::body::Bytes;
use axum::extract::Multipart;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::Error;

#[derive(ToSchema)]
pub struct RunJobResponse(Vec<u8>);

#[allow(dead_code)]
#[derive(ToSchema)]
#[schema(rename_all = "camelCase")]
pub struct RunJobRequest {
    file_id: String,
    contraction_file: Option<Vec<u8>>,
    search_term: Option<Vec<String>>,
    check_date: Option<Vec<usize>>,
    sort_col: Option<Vec<String>>,
}

#[allow(dead_code)]
#[derive(ToSchema)]
pub struct ExcelFileForm {
    file: Vec<u8>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct UploadFileEntry {
    pub id: String,
    #[serde(skip)]
    pub file_path: String,
}

#[derive(Serialize, ToSchema)]
pub struct RowsPayload {
    pub columns: Vec<String>,
}

#[derive(Debug)]
pub enum SortInfo {
    Asc { column_index: u32 },
    Desc { column_index: u32 },
}

impl SortInfo {
    const ASC: &'static str = "asc";
    const DESC: &'static str = "desc";
}

#[derive(Debug)]
pub struct JobDetails {
    file_id: String,
    contraction_file: Option<Bytes>,
    search_terms: Vec<String>,
    check_date_cols: Vec<u32>,
    sort_cols_info: Vec<SortInfo>,
}

impl JobDetails {
    const FILE_ID_FIELD_N: &'static str = "fileId";
    const CONTRACTION_F_FIELD_N: &'static str = "contractionFile";
    const SEARCH_TERMS_FIELD_N: &'static str = "searchTerm";
    const CHECK_DATE_FIELD_N: &'static str = "checkDate";
    const SORT_COL_FIELD_N: &'static str = "sortCol";
    const SEARCH_TERM_COUNTER_LIMIT: usize = 5;

    pub fn sort_infos(&self) -> &[SortInfo] {
        &self.sort_cols_info
    }

    pub fn file_id(&self) -> &str {
        &self.file_id
    }

    pub fn pop_contraction_file(&mut self) -> Option<Bytes> {
        let bytes = self.contraction_file.clone();
        self.contraction_file = None;
        bytes
    }

    pub fn search_terms(&self) -> &Vec<String> {
        &self.search_terms
    }

    pub fn check_date_cols(&self) -> &Vec<u32> {
        &self.check_date_cols
    }

    pub async fn try_from(mut value: Multipart) -> Result<Self> {
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
                JobDetails::FILE_ID_FIELD_N => file_id = Some(field.text().await?),
                JobDetails::CONTRACTION_F_FIELD_N => {
                    let bytes = field.bytes().await?;
                    if bytes.is_empty() {
                        continue;
                    }
                    contraction_file = Some(bytes);
                }
                JobDetails::SEARCH_TERMS_FIELD_N => {
                    if search_t_counter < JobDetails::SEARCH_TERM_COUNTER_LIMIT {
                        let text = field.text().await?;
                        if text.is_empty() {
                            continue;
                        }
                        search_terms.insert(search_t_counter, text);
                        search_t_counter += 1;
                    }
                }
                JobDetails::CHECK_DATE_FIELD_N => {
                    let text = field.text().await?;
                    let text = text.trim();
                    if text.is_empty() {
                        continue;
                    }
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
                    if text.is_empty() {
                        continue;
                    }
                    let text_parts: Vec<&str> = text.split(',').collect();
                    if text_parts.len() < 2 {
                        return Err(Error::Generic(format!("sortCol data has to be of form order,index Where order can take as value either asc or desc. Got: {}", text)));
                    }
                    let order = text_parts[0];
                    let order = order.to_lowercase();
                    let index = text_parts[1];
                    let index_val = index.parse::<u32>();
                    if index_val.is_err() {
                        return Err(Error::Generic(format!(
                            "Invalid value passed as column index. Got {}, expected a valid number",
                            index
                        )));
                    }
                    let sort_info = match order.as_str() {
                        SortInfo::ASC => SortInfo::Asc {
                            column_index: index_val.unwrap(),
                        },
                        SortInfo::DESC => SortInfo::Desc {
                            column_index: index_val.unwrap(),
                        },
                        _ => {
                            return Err(Error::Generic(format!(
                                "Invalid sort order value: Got {}, Expected: asc / desc",
                                order
                            )));
                        }
                    };
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
