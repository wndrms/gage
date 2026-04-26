use axum::{
    Json,
    extract::{Path, State},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    AppState, auth::extractor::AuthUser, errors::AppError,
    services::auto_categorize::normalize_merchant,
};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CategoryRule {
    pub id: Uuid,
    pub user_id: Uuid,
    pub keyword: String,
    pub keyword_normalized: String,
    pub category_id: Uuid,
    pub priority: i32,
    pub source: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRuleRequest {
    pub keyword: String,
    pub category_id: Uuid,
    pub priority: Option<i32>,
}

pub async fn list_rules(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<CategoryRule>>, AppError> {
    let rows = sqlx::query_as::<_, CategoryRule>(
        "SELECT * FROM merchant_category_rules WHERE user_id = $1 ORDER BY priority DESC, keyword ASC",
    )
    .bind(auth.id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}

pub async fn create_rule(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<CreateRuleRequest>,
) -> Result<Json<CategoryRule>, AppError> {
    let keyword = payload.keyword.trim().to_string();
    if keyword.is_empty() {
        return Err(AppError::BadRequest("키워드를 입력해 주세요".into()));
    }
    let normalized = normalize_merchant(&keyword);

    let owned: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM categories WHERE id = $1 AND user_id = $2")
            .bind(payload.category_id)
            .bind(auth.id)
            .fetch_optional(&state.pool)
            .await?;

    if owned.is_none() {
        return Err(AppError::BadRequest("유효하지 않은 카테고리입니다".into()));
    }

    let row = sqlx::query_as::<_, CategoryRule>(
        r#"
        INSERT INTO merchant_category_rules
            (user_id, keyword, keyword_normalized, category_id, priority, source)
        VALUES ($1, $2, $3, $4, $5, 'user')
        ON CONFLICT (user_id, keyword_normalized)
        DO UPDATE SET
            category_id = EXCLUDED.category_id,
            keyword = EXCLUDED.keyword,
            priority = EXCLUDED.priority,
            source = 'user',
            updated_at = now()
        RETURNING *
        "#,
    )
    .bind(auth.id)
    .bind(&keyword)
    .bind(normalized)
    .bind(payload.category_id)
    .bind(payload.priority.unwrap_or(200))
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(row))
}

pub async fn delete_rule(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = sqlx::query("DELETE FROM merchant_category_rules WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(auth.id)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok(Json(serde_json::json!({"message": "삭제되었습니다"})))
}
