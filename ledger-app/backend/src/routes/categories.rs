use axum::{
    Json,
    extract::{Path, State},
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{AppState, auth::extractor::AuthUser, errors::AppError, models::Category};

#[derive(Debug, Deserialize)]
pub struct CreateCategoryRequest {
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub r#type: String,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCategoryRequest {
    pub name: Option<String>,
    pub parent_id: Option<Uuid>,
    pub r#type: Option<String>,
    pub sort_order: Option<i32>,
}

pub async fn list_categories(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<Category>>, AppError> {
    let rows = sqlx::query_as::<_, Category>(
        "SELECT * FROM categories WHERE user_id = $1 ORDER BY sort_order ASC, created_at ASC",
    )
    .bind(auth.id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(rows))
}

pub async fn create_category(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<CreateCategoryRequest>,
) -> Result<Json<Category>, AppError> {
    if payload.name.trim().is_empty() {
        return Err(AppError::BadRequest(
            "카테고리 이름을 입력해 주세요".to_string(),
        ));
    }

    let row = sqlx::query_as::<_, Category>(
        r#"
        INSERT INTO categories (id, user_id, name, parent_id, type, sort_order)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(auth.id)
    .bind(payload.name)
    .bind(payload.parent_id)
    .bind(payload.r#type)
    .bind(payload.sort_order.unwrap_or(0))
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(row))
}

pub async fn update_category(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<UpdateCategoryRequest>,
) -> Result<Json<Category>, AppError> {
    let row = sqlx::query_as::<_, Category>(
        r#"
        UPDATE categories
        SET
            name = COALESCE($3, name),
            parent_id = COALESCE($4, parent_id),
            type = COALESCE($5, type),
            sort_order = COALESCE($6, sort_order),
            updated_at = now()
        WHERE id = $1 AND user_id = $2
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(auth.id)
    .bind(payload.name)
    .bind(payload.parent_id)
    .bind(payload.r#type)
    .bind(payload.sort_order)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    Ok(Json(row))
}

pub async fn delete_category(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = sqlx::query("DELETE FROM categories WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(auth.id)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok(Json(serde_json::json!({"message": "삭제되었습니다"})))
}
