use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    AppState,
    auth::extractor::AuthUser,
    card_rules::{CardBenefitSummary, RuleTransaction, calculate_summary, empty_summary},
    errors::AppError,
    models::Card,
};

#[derive(Debug, Deserialize)]
pub struct CreateCardRequest {
    pub issuer: String,
    pub card_name: String,
    pub preset_id: Option<Uuid>,
    pub billing_day: Option<i32>,
    pub payment_day: Option<i32>,
    pub linked_account_id: Option<Uuid>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCardRequest {
    pub issuer: Option<String>,
    pub card_name: Option<String>,
    pub preset_id: Option<Uuid>,
    pub billing_day: Option<i32>,
    pub payment_day: Option<i32>,
    pub linked_account_id: Option<Uuid>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CardSummaryQuery {
    pub month: String,
}

#[derive(Debug, Serialize)]
pub struct CardSummaryResponse {
    pub card_id: Uuid,
    pub month: String,
    pub summary: CardBenefitSummary,
}

#[derive(Debug, Serialize)]
pub struct CardTransactionItem {
    pub id: Uuid,
    pub transaction_at: chrono::DateTime<chrono::Utc>,
    pub amount: i64,
    pub merchant_name: Option<String>,
    pub description: Option<String>,
    pub category_name: Option<String>,
    pub account_name: Option<String>,
    pub memo: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CardTransactionResponse {
    pub card_id: Uuid,
    pub month: String,
    pub total_count: i32,
    pub total_amount: i64,
    pub transactions: Vec<CardTransactionItem>,
}

pub async fn list_cards(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<Card>>, AppError> {
    let rows = sqlx::query_as::<_, Card>("SELECT * FROM cards WHERE user_id = $1 ORDER BY created_at DESC")
        .bind(auth.id)
        .fetch_all(&state.pool)
        .await?;

    Ok(Json(rows))
}

pub async fn create_card(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<CreateCardRequest>,
) -> Result<Json<Card>, AppError> {
    if payload.issuer.trim().is_empty() || payload.card_name.trim().is_empty() {
        return Err(AppError::BadRequest(
            "카드사와 카드명을 입력해 주세요".to_string(),
        ));
    }

    let row = sqlx::query_as::<_, Card>(
        r#"
        INSERT INTO cards (id, user_id, issuer, card_name, preset_id, billing_day, payment_day, linked_account_id, is_active)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(auth.id)
    .bind(payload.issuer)
    .bind(payload.card_name)
    .bind(payload.preset_id)
    .bind(payload.billing_day)
    .bind(payload.payment_day)
    .bind(payload.linked_account_id)
    .bind(payload.is_active.unwrap_or(true))
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(row))
}

pub async fn update_card(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<UpdateCardRequest>,
) -> Result<Json<Card>, AppError> {
    let row = sqlx::query_as::<_, Card>(
        r#"
        UPDATE cards
        SET
            issuer = COALESCE($3, issuer),
            card_name = COALESCE($4, card_name),
            preset_id = COALESCE($5, preset_id),
            billing_day = COALESCE($6, billing_day),
            payment_day = COALESCE($7, payment_day),
            linked_account_id = COALESCE($8, linked_account_id),
            is_active = COALESCE($9, is_active),
            updated_at = now()
        WHERE id = $1 AND user_id = $2
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(auth.id)
    .bind(payload.issuer)
    .bind(payload.card_name)
    .bind(payload.preset_id)
    .bind(payload.billing_day)
    .bind(payload.payment_day)
    .bind(payload.linked_account_id)
    .bind(payload.is_active)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    Ok(Json(row))
}

pub async fn delete_card(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = sqlx::query("DELETE FROM cards WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(auth.id)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok(Json(serde_json::json!({"message": "삭제되었습니다"})))
}

pub async fn get_card_summary(
    Path(id): Path<Uuid>,
    Query(query): Query<CardSummaryQuery>,
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<CardSummaryResponse>, AppError> {
    let month_date = NaiveDate::parse_from_str(&format!("{}-01", query.month), "%Y-%m-%d")
        .map_err(|_| {
            AppError::BadRequest("월 형식이 올바르지 않습니다. YYYY-MM 형식으로 입력해 주세요".to_string())
        })?;
    let month_start = month_date.and_hms_opt(0, 0, 0).unwrap().and_utc();
    let next_month = if month_date.month() == 12 {
        NaiveDate::from_ymd_opt(month_date.year() + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(month_date.year(), month_date.month() + 1, 1).unwrap()
    }
    .and_hms_opt(0, 0, 0)
    .unwrap()
    .and_utc();

    ensure_card_exists(&state, auth.id, id).await?;

    let monthly_spending = sqlx::query_scalar::<_, Option<i64>>(
        r#"
        SELECT SUM(amount)::bigint
        FROM transactions
        WHERE user_id = $1
          AND card_id = $2
          AND transaction_at >= $3
          AND transaction_at < $4
          AND amount > 0
          AND type = 'expense'
        "#,
    )
    .bind(auth.id)
    .bind(id)
    .bind(month_start)
    .bind(next_month)
    .fetch_one(&state.pool)
    .await?
    .unwrap_or(0);

    let tx_rows = sqlx::query_as::<_, (i64, Option<String>, Option<String>)>(
        r#"
        SELECT t.amount, t.merchant_name, c.name
        FROM transactions t
        LEFT JOIN categories c ON t.category_id = c.id
        WHERE t.user_id = $1
          AND t.card_id = $2
          AND t.transaction_at >= $3
          AND t.transaction_at < $4
          AND t.amount > 0
          AND t.type = 'expense'
        "#,
    )
    .bind(auth.id)
    .bind(id)
    .bind(month_start)
    .bind(next_month)
    .fetch_all(&state.pool)
    .await?;

    let rule_transactions = tx_rows
        .into_iter()
        .map(|(amount, merchant_name, category_name)| RuleTransaction {
            amount,
            merchant_name,
            category_name,
        })
        .collect::<Vec<_>>();

    let mut summary = empty_summary();
    summary.monthly_spending = monthly_spending;
    summary.eligible_spending = monthly_spending;

    if let Some(preset_json) = sqlx::query_scalar::<_, Option<serde_json::Value>>(
        "SELECT jsonb_build_object('monthly_requirement', cp.monthly_requirement, 'excluded', cp.rules->'excluded', 'benefits', cp.benefits) FROM card_presets cp JOIN cards c ON c.preset_id = cp.id WHERE c.id = $1",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .flatten()
    {
        summary = calculate_summary(&rule_transactions, &preset_json);
    }

    Ok(Json(CardSummaryResponse {
        card_id: id,
        month: query.month,
        summary,
    }))
}

pub async fn get_card_transactions(
    Path(id): Path<Uuid>,
    Query(query): Query<CardSummaryQuery>,
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<CardTransactionResponse>, AppError> {
    let month_date = NaiveDate::parse_from_str(&format!("{}-01", query.month), "%Y-%m-%d")
        .map_err(|_| {
            AppError::BadRequest("월 형식이 올바르지 않습니다. YYYY-MM 형식으로 입력해 주세요".to_string())
        })?;

    let month_start = month_date.and_hms_opt(0, 0, 0).unwrap().and_utc();
    let next_month = if month_date.month() == 12 {
        NaiveDate::from_ymd_opt(month_date.year() + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(month_date.year(), month_date.month() + 1, 1).unwrap()
    }
    .and_hms_opt(0, 0, 0)
    .unwrap()
    .and_utc();

    ensure_card_exists(&state, auth.id, id).await?;

    let rows = sqlx::query_as::<_, (Uuid, chrono::DateTime<chrono::Utc>, i64, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>)>(
        r#"
        SELECT
            t.id,
            t.transaction_at,
            t.amount,
            t.merchant_name,
            t.description,
            c.name AS category_name,
            a.name AS account_name,
            t.memo
        FROM transactions t
        LEFT JOIN categories c ON t.category_id = c.id
        LEFT JOIN accounts a ON t.account_id = a.id
        WHERE t.user_id = $1
          AND t.card_id = $2
          AND t.transaction_at >= $3
          AND t.transaction_at < $4
          AND t.type = 'expense'
        ORDER BY t.transaction_at DESC
        "#,
    )
    .bind(auth.id)
    .bind(id)
    .bind(month_start)
    .bind(next_month)
    .fetch_all(&state.pool)
    .await?;

    let total_count = rows.len() as i32;
    let total_amount = rows.iter().map(|(_, _, amount, _, _, _, _, _)| *amount).sum::<i64>();

    let transactions = rows
        .into_iter()
        .map(
            |(id, transaction_at, amount, merchant_name, description, category_name, account_name, memo)| {
                CardTransactionItem {
                    id,
                    transaction_at,
                    amount,
                    merchant_name,
                    description,
                    category_name,
                    account_name,
                    memo,
                }
            },
        )
        .collect::<Vec<_>>();

    Ok(Json(CardTransactionResponse {
        card_id: id,
        month: query.month,
        total_count,
        total_amount,
        transactions,
    }))
}

async fn ensure_card_exists(state: &AppState, user_id: Uuid, card_id: Uuid) -> Result<(), AppError> {
    let exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(1) FROM cards WHERE id = $1 AND user_id = $2")
        .bind(card_id)
        .bind(user_id)
        .fetch_one(&state.pool)
        .await?;

    if exists == 0 {
        return Err(AppError::NotFound);
    }

    Ok(())
}
