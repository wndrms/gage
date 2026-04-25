use axum::{Json, extract::State, http::HeaderMap};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    AppState,
    auth::extractor::AuthUser,
    errors::AppError,
    telegram::{commands, webhook},
};

#[derive(Debug, Serialize)]
pub struct TelegramWebhookResponse {
    pub message: String,
    pub reply: String,
    pub user_id: Uuid,
    pub telegram_chat_id: Option<String>,
}

pub async fn telegram_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<webhook::TelegramWebhookPayload>,
) -> Result<Json<TelegramWebhookResponse>, AppError> {
    // X-Telegram-Bot-Api-Secret-Token 헤더 검증 (설정된 경우)
    if let Some(expected_secret) = &state.config.telegram_webhook_secret {
        let provided = headers
            .get("x-telegram-bot-api-secret-token")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if !constant_time_eq(provided.as_bytes(), expected_secret.as_bytes()) {
            return Err(AppError::Unauthorized);
        }
    }

    let message_text = webhook::extract_message_text(&payload)
        .ok_or_else(|| AppError::BadRequest("메시지 텍스트가 없습니다".to_string()))?;

    let chat_id = webhook::extract_chat_id(&payload);
    let telegram_user_id = webhook::extract_user_id(&payload);

    let user_id = webhook::resolve_user_id(&state, chat_id.as_deref())
        .await
        .map_err(|err| AppError::BadRequest(err.to_string()))?;

    if let (Some(chat), Some(user)) = (&chat_id, &telegram_user_id) {
        let exists = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(1) FROM telegram_connections WHERE user_id = $1 AND telegram_user_id = $2 AND telegram_chat_id = $3 AND is_active = true",
        )
        .bind(user_id)
        .bind(user)
        .bind(chat)
        .fetch_one(&state.pool)
        .await?;

        if exists == 0 {
            sqlx::query(
                "INSERT INTO telegram_connections (id, user_id, telegram_user_id, telegram_chat_id, connected_at, is_active) VALUES ($1, $2, $3, $4, now(), true)",
            )
            .bind(Uuid::new_v4())
            .bind(user_id)
            .bind(user)
            .bind(chat)
            .execute(&state.pool)
            .await?;
        }
    }

    let reply = commands::handle_text_command(&state.pool, user_id, &message_text)
        .await
        .map_err(|err| AppError::BadRequest(err.to_string()))?;

    Ok(Json(TelegramWebhookResponse {
        message: "명령을 처리했습니다".to_string(),
        reply,
        user_id,
        telegram_chat_id: chat_id,
    }))
}

#[derive(Debug, Deserialize)]
pub struct TelegramSetupRequest {
    pub bot_token: String,
    pub webhook_url: String,
}

#[derive(Debug, Serialize)]
pub struct TelegramSetupResponse {
    pub ok: bool,
    pub description: String,
}

pub async fn telegram_setup(
    State(_state): State<AppState>,
    _auth: AuthUser,
    Json(payload): Json<TelegramSetupRequest>,
) -> Result<Json<TelegramSetupResponse>, AppError> {
    if payload.bot_token.trim().is_empty() {
        return Err(AppError::BadRequest("봇 토큰을 입력해 주세요".to_string()));
    }
    if payload.webhook_url.trim().is_empty() {
        return Err(AppError::BadRequest("웹훅 URL을 입력해 주세요".to_string()));
    }

    let _ = payload;
    Ok(Json(TelegramSetupResponse {
        ok: false,
        description: "웹훅 자동 등록은 아직 비활성화되어 있습니다. 텔레그램 콘솔에서 웹훅을 수동 등록해 주세요.".to_string(),
    }))
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}
