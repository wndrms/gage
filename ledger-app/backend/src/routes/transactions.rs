use axum::{Json, extract::{Path, Query, State}};
use chrono::{DateTime, NaiveDate, Utc};
use serde::Deserialize;
use sqlx::QueryBuilder;
use uuid::Uuid;

use crate::{
    AppState,
    auth::extractor::AuthUser,
    errors::AppError,
    import::dedupe::build_dedupe_key,
    models::Transaction,
    services::auto_categorize::{auto_categorize, learn_rule},
};

#[derive(Debug, Deserialize)]
pub struct ListTransactionsQuery {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub r#type: Option<String>,
    pub account_id: Option<Uuid>,
    pub card_id: Option<Uuid>,
    pub category_id: Option<Uuid>,
    pub keyword: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TransactionPayload {
    pub transaction_at: DateTime<Utc>,
    pub posted_at: Option<DateTime<Utc>>,
    pub r#type: String,
    pub amount: i64,
    pub merchant_name: Option<String>,
    pub description: Option<String>,
    pub category_id: Option<Uuid>,
    pub account_id: Option<Uuid>,
    pub card_id: Option<Uuid>,
    pub source_type: Option<String>,
    pub source_institution: Option<String>,
    pub source_file_id: Option<Uuid>,
    pub balance_after: Option<i64>,
    pub raw_data: Option<serde_json::Value>,
    pub memo: Option<String>,
}

pub async fn list_transactions(
    Query(query): Query<ListTransactionsQuery>,
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<Transaction>>, AppError> {
    let mut qb = QueryBuilder::<sqlx::Postgres>::new(
        "SELECT * FROM transactions WHERE user_id = ",
    );
    qb.push_bind(auth.id);

    if let Some(start_date) = &query.start_date {
        let start_date = NaiveDate::parse_from_str(start_date, "%Y-%m-%d")
            .map_err(|_| AppError::BadRequest("시작일 형식이 올바르지 않습니다".to_string()))?;
        qb.push(" AND transaction_at >= ")
            .push_bind(start_date.and_hms_opt(0, 0, 0).unwrap().and_utc());
    }

    if let Some(end_date) = &query.end_date {
        let end_date = NaiveDate::parse_from_str(end_date, "%Y-%m-%d")
            .map_err(|_| AppError::BadRequest("종료일 형식이 올바르지 않습니다".to_string()))?;
        qb.push(" AND transaction_at <= ")
            .push_bind(end_date.and_hms_opt(23, 59, 59).unwrap().and_utc());
    }

    if let Some(v) = &query.r#type {
        qb.push(" AND type = ").push_bind(v);
    }
    if let Some(v) = query.account_id {
        qb.push(" AND account_id = ").push_bind(v);
    }
    if let Some(v) = query.card_id {
        qb.push(" AND card_id = ").push_bind(v);
    }
    if let Some(v) = query.category_id {
        qb.push(" AND category_id = ").push_bind(v);
    }
    if let Some(keyword) = &query.keyword {
        let pattern = format!("%{}%", keyword.trim());
        qb.push(" AND (merchant_name ILIKE ")
            .push_bind(pattern.clone())
            .push(" OR description ILIKE ")
            .push_bind(pattern.clone())
            .push(" OR memo ILIKE ")
            .push_bind(pattern)
            .push(")");
    }

    qb.push(" ORDER BY transaction_at DESC, created_at DESC");
    let rows = qb.build_query_as::<Transaction>().fetch_all(&state.pool).await?;

    Ok(Json(rows))
}

pub async fn create_transaction(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<TransactionPayload>,
) -> Result<Json<Transaction>, AppError> {
    if payload.amount == 0 {
        return Err(AppError::BadRequest("금액은 0이 될 수 없습니다".to_string()));
    }

    let dedupe_key = build_dedupe_key(
        auth.id,
        payload.source_institution.as_deref(),
        payload.transaction_at,
        payload.amount,
        payload.merchant_name.as_deref(),
        payload.description.as_deref(),
        payload.card_id,
        payload.account_id,
        None,
    );

    // 카테고리 미지정 시 자동 분류 시도
    let resolved_category_id = match payload.category_id {
        Some(id) => Some(id),
        None => auto_categorize(&state.pool, auth.id, payload.merchant_name.as_deref()).await,
    };

    let row = sqlx::query_as::<_, Transaction>(
        r#"
        INSERT INTO transactions (
            id, user_id, transaction_at, posted_at, type, amount,
            merchant_name, description, category_id, account_id, card_id,
            source_type, source_institution, source_file_id, balance_after,
            raw_data, dedupe_key, memo
        )
        VALUES (
            $1, $2, $3, $4, $5, $6,
            $7, $8, $9, $10, $11,
            $12, $13, $14, $15,
            $16, $17, $18
        )
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(auth.id)
    .bind(payload.transaction_at)
    .bind(payload.posted_at)
    .bind(payload.r#type)
    .bind(payload.amount)
    .bind(payload.merchant_name)
    .bind(payload.description)
    .bind(resolved_category_id)
    .bind(payload.account_id)
    .bind(payload.card_id)
    .bind(payload.source_type.unwrap_or_else(|| "manual".to_string()))
    .bind(payload.source_institution)
    .bind(payload.source_file_id)
    .bind(payload.balance_after)
    .bind(payload.raw_data.unwrap_or_else(|| serde_json::json!({})))
    .bind(dedupe_key)
    .bind(payload.memo)
    .fetch_one(&state.pool)
    .await
    .map_err(|err| {
        if let sqlx::Error::Database(db_err) = &err {
            if db_err.is_unique_violation() {
                return AppError::BadRequest("중복된 거래입니다".to_string());
            }
        }
        err.into()
    })?;

    Ok(Json(row))
}

pub async fn get_transaction(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Transaction>, AppError> {
    let row = sqlx::query_as::<_, Transaction>(
        "SELECT * FROM transactions WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(auth.id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    Ok(Json(row))
}

pub async fn update_transaction(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<TransactionPayload>,
) -> Result<Json<Transaction>, AppError> {
    let dedupe_key = build_dedupe_key(
        auth.id,
        payload.source_institution.as_deref(),
        payload.transaction_at,
        payload.amount,
        payload.merchant_name.as_deref(),
        payload.description.as_deref(),
        payload.card_id,
        payload.account_id,
        None,
    );

    let row = sqlx::query_as::<_, Transaction>(
        r#"
        UPDATE transactions
        SET
            transaction_at = $3,
            posted_at = $4,
            type = $5,
            amount = $6,
            merchant_name = $7,
            description = $8,
            category_id = $9,
            account_id = $10,
            card_id = $11,
            source_type = $12,
            source_institution = $13,
            source_file_id = $14,
            balance_after = $15,
            raw_data = $16,
            dedupe_key = $17,
            memo = $18,
            updated_at = now()
        WHERE id = $1 AND user_id = $2
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(auth.id)
    .bind(payload.transaction_at)
    .bind(payload.posted_at)
    .bind(payload.r#type)
    .bind(payload.amount)
    .bind(payload.merchant_name)
    .bind(payload.description)
    .bind(payload.category_id)
    .bind(payload.account_id)
    .bind(payload.card_id)
    .bind(payload.source_type.unwrap_or_else(|| "manual".to_string()))
    .bind(payload.source_institution)
    .bind(payload.source_file_id)
    .bind(payload.balance_after)
    .bind(payload.raw_data.unwrap_or_else(|| serde_json::json!({})))
    .bind(dedupe_key)
    .bind(payload.memo)
    .fetch_optional(&state.pool)
    .await
    .map_err(|err| {
        if let sqlx::Error::Database(db_err) = &err {
            if db_err.is_unique_violation() {
                return AppError::BadRequest("중복된 거래입니다".to_string());
            }
        }
        AppError::from(err)
    })?
    .ok_or(AppError::NotFound)?;

    // 가맹점 + 카테고리 모두 지정된 경우 규칙 학습
    if let (Some(merchant), Some(category_id)) = (row.merchant_name.as_deref(), row.category_id) {
        learn_rule(&state.pool, auth.id, merchant, category_id).await;
    }

    Ok(Json(row))
}

pub async fn delete_transaction(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = sqlx::query("DELETE FROM transactions WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(auth.id)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok(Json(serde_json::json!({"message": "삭제되었습니다"})))
}
