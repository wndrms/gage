use axum::{Json, extract::{Path, Query, State}};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{AppState, auth::extractor::AuthUser, errors::AppError, models::AssetSnapshot};

#[derive(Debug, Deserialize)]
pub struct AssetSnapshotPayload {
    pub snapshot_date: NaiveDate,
    pub account_id: Uuid,
    pub amount: i64,
    pub memo: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct NetWorthQuery {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Serialize)]
pub struct NetWorthPoint {
    pub date: String,
    pub assets: i64,
    pub liabilities: i64,
    pub net_worth: i64,
}

pub async fn list_asset_snapshots(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<AssetSnapshot>>, AppError> {
    let rows = sqlx::query_as::<_, AssetSnapshot>(
        "SELECT * FROM asset_snapshots WHERE user_id = $1 ORDER BY snapshot_date DESC, created_at DESC",
    )
    .bind(auth.id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(rows))
}

pub async fn create_asset_snapshot(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<AssetSnapshotPayload>,
) -> Result<Json<AssetSnapshot>, AppError> {
    let row = sqlx::query_as::<_, AssetSnapshot>(
        r#"
        INSERT INTO asset_snapshots (id, user_id, snapshot_date, account_id, amount, memo)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT(user_id, snapshot_date, account_id)
        DO UPDATE SET amount = EXCLUDED.amount, memo = EXCLUDED.memo, updated_at = now()
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(auth.id)
    .bind(payload.snapshot_date)
    .bind(payload.account_id)
    .bind(payload.amount)
    .bind(payload.memo)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(row))
}

pub async fn update_asset_snapshot(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<AssetSnapshotPayload>,
) -> Result<Json<AssetSnapshot>, AppError> {
    let row = sqlx::query_as::<_, AssetSnapshot>(
        r#"
        UPDATE asset_snapshots
        SET
            snapshot_date = $3,
            account_id = $4,
            amount = $5,
            memo = $6,
            updated_at = now()
        WHERE id = $1 AND user_id = $2
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(auth.id)
    .bind(payload.snapshot_date)
    .bind(payload.account_id)
    .bind(payload.amount)
    .bind(payload.memo)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    Ok(Json(row))
}

pub async fn delete_asset_snapshot(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = sqlx::query("DELETE FROM asset_snapshots WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(auth.id)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok(Json(serde_json::json!({"message": "삭제되었습니다"})))
}

pub async fn get_net_worth(
    Query(query): Query<NetWorthQuery>,
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<NetWorthPoint>>, AppError> {
    let from = NaiveDate::parse_from_str(&query.from, "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest("시작일 형식이 올바르지 않습니다".to_string()))?;
    let to = NaiveDate::parse_from_str(&query.to, "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest("종료일 형식이 올바르지 않습니다".to_string()))?;

    let rows = sqlx::query_as::<_, (NaiveDate, String, Option<i64>, Option<i64>)>(
        r#"
        SELECT s.snapshot_date,
               a.type,
               SUM(CASE WHEN a.type != 'credit_card_liability' THEN s.amount ELSE 0 END)::bigint,
               SUM(CASE WHEN a.type = 'credit_card_liability' THEN s.amount ELSE 0 END)::bigint
        FROM asset_snapshots s
        JOIN accounts a ON s.account_id = a.id
        WHERE s.user_id = $1
          AND s.snapshot_date >= $2
          AND s.snapshot_date <= $3
        GROUP BY s.snapshot_date, a.type
        ORDER BY s.snapshot_date ASC
        "#,
    )
    .bind(auth.id)
    .bind(from)
    .bind(to)
    .fetch_all(&state.pool)
    .await?;

    let mut map = std::collections::BTreeMap::<NaiveDate, (i64, i64)>::new();
    for (date, _typ, assets, liabilities) in rows {
        let entry = map.entry(date).or_insert((0, 0));
        entry.0 += assets.unwrap_or(0);
        entry.1 += liabilities.unwrap_or(0);
    }

    let result = map
        .into_iter()
        .map(|(date, (assets, liabilities))| NetWorthPoint {
            date: date.format("%Y-%m-%d").to_string(),
            assets,
            liabilities,
            net_worth: assets - liabilities,
        })
        .collect();

    Ok(Json(result))
}
