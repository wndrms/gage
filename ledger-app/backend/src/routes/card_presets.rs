use axum::{
    Json,
    extract::{Path, State},
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{AppState, auth::extractor::AuthUser, errors::AppError, models::CardPreset};

#[derive(Debug, Deserialize)]
pub struct CardPresetPayload {
    pub issuer: String,
    pub card_name: String,
    pub aliases: Option<Vec<String>>,
    pub monthly_requirement: Option<i64>,
    pub rules: Option<serde_json::Value>,
    pub benefits: Option<serde_json::Value>,
}

pub async fn list_card_presets(
    State(state): State<AppState>,
    _auth: AuthUser,
) -> Result<Json<Vec<CardPreset>>, AppError> {
    let rows =
        sqlx::query_as::<_, CardPreset>("SELECT * FROM card_presets ORDER BY created_at DESC")
            .fetch_all(&state.pool)
            .await?;
    Ok(Json(rows))
}

pub async fn create_card_preset(
    State(state): State<AppState>,
    _auth: AuthUser,
    Json(payload): Json<CardPresetPayload>,
) -> Result<Json<CardPreset>, AppError> {
    if payload.issuer.trim().is_empty() || payload.card_name.trim().is_empty() {
        return Err(AppError::BadRequest(
            "카드사와 카드명을 입력해 주세요".to_string(),
        ));
    }

    let row = sqlx::query_as::<_, CardPreset>(
        r#"
        INSERT INTO card_presets (id, issuer, card_name, aliases, monthly_requirement, rules, benefits)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(payload.issuer)
    .bind(payload.card_name)
    .bind(payload.aliases.unwrap_or_default())
    .bind(payload.monthly_requirement)
    .bind(payload.rules.unwrap_or_else(|| serde_json::json!({})))
    .bind(payload.benefits.unwrap_or_else(|| serde_json::json!({})))
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(row))
}

pub async fn update_card_preset(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    _auth: AuthUser,
    Json(payload): Json<CardPresetPayload>,
) -> Result<Json<CardPreset>, AppError> {
    let row = sqlx::query_as::<_, CardPreset>(
        r#"
        UPDATE card_presets
        SET
            issuer = COALESCE(NULLIF($2, ''), issuer),
            card_name = COALESCE(NULLIF($3, ''), card_name),
            aliases = COALESCE($4, aliases),
            monthly_requirement = COALESCE($5, monthly_requirement),
            rules = COALESCE($6, rules),
            benefits = COALESCE($7, benefits),
            updated_at = now()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(payload.issuer)
    .bind(payload.card_name)
    .bind(payload.aliases)
    .bind(payload.monthly_requirement)
    .bind(payload.rules)
    .bind(payload.benefits)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    Ok(Json(row))
}

pub async fn delete_card_preset(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    _auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = sqlx::query("DELETE FROM card_presets WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok(Json(serde_json::json!({"message": "삭제되었습니다"})))
}
