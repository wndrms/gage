use axum::{
    Json,
    extract::{Path, State},
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{AppState, auth::extractor::AuthUser, errors::AppError, models::Account};

#[derive(Debug, Deserialize)]
pub struct CreateAccountRequest {
    pub name: String,
    pub r#type: String,
    pub institution: Option<String>,
    pub currency: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAccountRequest {
    pub name: Option<String>,
    pub r#type: Option<String>,
    pub institution: Option<String>,
    pub currency: Option<String>,
    pub is_active: Option<bool>,
}

pub async fn list_accounts(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<Account>>, AppError> {
    let rows = sqlx::query_as::<_, Account>(
        "SELECT * FROM accounts WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(auth.id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(rows))
}

pub async fn create_account(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<CreateAccountRequest>,
) -> Result<Json<Account>, AppError> {
    if payload.name.trim().is_empty() {
        return Err(AppError::BadRequest(
            "계좌 이름을 입력해 주세요".to_string(),
        ));
    }

    let row = sqlx::query_as::<_, Account>(
        r#"
        INSERT INTO accounts (id, user_id, name, type, institution, currency, is_active)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(auth.id)
    .bind(payload.name)
    .bind(payload.r#type)
    .bind(payload.institution)
    .bind(payload.currency.unwrap_or_else(|| "KRW".to_string()))
    .bind(payload.is_active.unwrap_or(true))
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(row))
}

pub async fn update_account(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<UpdateAccountRequest>,
) -> Result<Json<Account>, AppError> {
    let row = sqlx::query_as::<_, Account>(
        r#"
        UPDATE accounts
        SET
            name = COALESCE($3, name),
            type = COALESCE($4, type),
            institution = COALESCE($5, institution),
            currency = COALESCE($6, currency),
            is_active = COALESCE($7, is_active),
            updated_at = now()
        WHERE id = $1 AND user_id = $2
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(auth.id)
    .bind(payload.name)
    .bind(payload.r#type)
    .bind(payload.institution)
    .bind(payload.currency)
    .bind(payload.is_active)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    Ok(Json(row))
}

pub async fn delete_account(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = sqlx::query("DELETE FROM accounts WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(auth.id)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok(Json(serde_json::json!({"message": "삭제되었습니다"})))
}
