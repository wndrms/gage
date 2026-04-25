use axum::{Json, extract::{Multipart, Path, State}};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{AppState, auth::extractor::AuthUser, errors::AppError, import};

#[derive(Debug, Serialize)]
pub struct ImportListItem {
    pub id: Uuid,
    pub source_type: String,
    pub institution: String,
    pub original_filename: Option<String>,
    pub status: String,
    pub parsed_count: i32,
    pub imported_count: i32,
    pub duplicate_count: i32,
    pub error_message: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct ImportDetailResponse {
    pub import: crate::models::ImportRecord,
    pub rows: Vec<crate::models::ImportRow>,
}

#[derive(Debug, Serialize)]
pub struct ImportPreviewResponse {
    pub import_id: Uuid,
    pub status: String,
    pub 총건수: i32,
    pub 신규건수: i32,
    pub 중복건수: i32,
    pub 오류건수: i32,
}

#[derive(Debug, Deserialize)]
pub struct PastedTextImportRequest {
    pub institution: Option<String>,
    pub text: String,
}

pub async fn upload_file_import(
    State(state): State<AppState>,
    auth: AuthUser,
    mut multipart: Multipart,
) -> Result<Json<ImportPreviewResponse>, AppError> {
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut filename: Option<String> = None;
    let mut institution: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::BadRequest("멀티파트 데이터를 읽을 수 없습니다".to_string()))?
    {
        let name = field.name().unwrap_or_default().to_string();
        if name == "file" {
            filename = field.file_name().map(ToString::to_string);
            let bytes = field
                .bytes()
                .await
                .map_err(|_| AppError::BadRequest("파일 내용을 읽을 수 없습니다".to_string()))?;
            file_bytes = Some(bytes.to_vec());
        } else if name == "institution" {
            let value = field
                .text()
                .await
                .map_err(|_| AppError::BadRequest("기관 정보를 읽을 수 없습니다".to_string()))?;
            if !value.trim().is_empty() {
                institution = Some(value);
            }
        }
    }

    let file_bytes = file_bytes.ok_or_else(|| AppError::BadRequest("파일을 선택해 주세요".to_string()))?;
    let ext = filename
        .as_ref()
        .and_then(|name| name.rsplit('.').next())
        .unwrap_or("csv")
        .to_lowercase();
    let source_type = match ext.as_str() {
        "xlsx" => "xlsx",
        "xls" => "xls",
        _ => "csv",
    };

    let detected = import::detect_institution_from_filename(filename.as_deref());
    let institution = institution
        .or(detected)
        .unwrap_or_else(|| "unknown".to_string());

    let (import_record, summary) = import::create_import_preview(
        &state.pool,
        auth.id,
        source_type,
        &institution,
        filename,
        None,
        &file_bytes,
    )
    .await
    .map_err(|err| AppError::BadRequest(err.to_string()))?;

    Ok(Json(ImportPreviewResponse {
        import_id: import_record.id,
        status: import_record.status,
        총건수: summary.total,
        신규건수: summary.new_count,
        중복건수: summary.duplicate_count,
        오류건수: summary.error_count,
    }))
}

pub async fn upload_pasted_text_import(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<PastedTextImportRequest>,
) -> Result<Json<ImportPreviewResponse>, AppError> {
    if payload.text.trim().is_empty() {
        return Err(AppError::BadRequest("붙여넣을 텍스트를 입력해 주세요".to_string()));
    }

    let institution = payload
        .institution
        .unwrap_or_else(|| "pasted_text".to_string());

    let (import_record, summary) = import::create_import_preview(
        &state.pool,
        auth.id,
        "pasted_text",
        &institution,
        None,
        Some(payload.text.clone()),
        payload.text.as_bytes(),
    )
    .await
    .map_err(|err| AppError::BadRequest(err.to_string()))?;

    Ok(Json(ImportPreviewResponse {
        import_id: import_record.id,
        status: import_record.status,
        총건수: summary.total,
        신규건수: summary.new_count,
        중복건수: summary.duplicate_count,
        오류건수: summary.error_count,
    }))
}

pub async fn list_imports(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<ImportListItem>>, AppError> {
    let rows = import::list_imports(&state.pool, auth.id)
        .await
        .map_err(|err| AppError::BadRequest(err.to_string()))?;

    let result = rows
        .into_iter()
        .map(|v| ImportListItem {
            id: v.id,
            source_type: v.source_type,
            institution: v.institution,
            original_filename: v.original_filename,
            status: v.status,
            parsed_count: v.parsed_count,
            imported_count: v.imported_count,
            duplicate_count: v.duplicate_count,
            error_message: v.error_message,
            created_at: v.created_at,
        })
        .collect();

    Ok(Json(result))
}

pub async fn get_import(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<ImportDetailResponse>, AppError> {
    let (import_row, rows) = import::get_import(&state.pool, auth.id, id)
        .await
        .map_err(|err| AppError::BadRequest(err.to_string()))?;

    Ok(Json(ImportDetailResponse {
        import: import_row,
        rows,
    }))
}

pub async fn confirm_import(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let summary = import::confirm_import(&state.pool, auth.id, id)
        .await
        .map_err(|err| AppError::BadRequest(err.to_string()))?;

    Ok(Json(serde_json::json!({
        "message": "가져오기 저장이 완료되었습니다",
        "총건수": summary.total,
        "신규건수": summary.new_count,
        "중복건수": summary.duplicate_count,
        "오류건수": summary.error_count
    })))
}

pub async fn cancel_import(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    import::cancel_import(&state.pool, auth.id, id)
        .await
        .map_err(|err| AppError::BadRequest(err.to_string()))?;

    Ok(Json(serde_json::json!({"message": "가져오기를 취소했습니다"})))
}
