use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct TelegramWebhookPayload {
    pub update_id: Option<i64>,
    pub telegram_user_id: Option<String>,
    pub telegram_chat_id: Option<String>,
    pub message_text: Option<String>,
    pub message: Option<TelegramMessage>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TelegramMessage {
    pub text: Option<String>,
    pub chat: Option<TelegramChat>,
    pub from: Option<TelegramUser>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TelegramChat {
    pub id: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TelegramUser {
    pub id: i64,
}

pub async fn resolve_user_id(state: &AppState, chat_id: Option<&str>) -> Result<Uuid> {
    if let Some(chat_id) = chat_id {
        if let Some(user_id) = sqlx::query_scalar::<_, Option<Uuid>>(
            "SELECT user_id FROM telegram_connections WHERE telegram_chat_id = $1 AND is_active = true ORDER BY connected_at DESC LIMIT 1",
        )
        .bind(chat_id)
        .fetch_one(&state.pool)
        .await?
        {
            return Ok(user_id);
        }
    }

    sqlx::query_scalar::<_, Option<Uuid>>("SELECT id FROM users ORDER BY created_at ASC LIMIT 1")
        .fetch_one(&state.pool)
        .await?
        .ok_or_else(|| anyhow!("사용자를 찾을 수 없습니다"))
}

pub fn extract_message_text(payload: &TelegramWebhookPayload) -> Option<String> {
    payload
        .message_text
        .clone()
        .or_else(|| payload.message.as_ref()?.text.clone())
}

pub fn extract_chat_id(payload: &TelegramWebhookPayload) -> Option<String> {
    payload.telegram_chat_id.clone().or_else(|| {
        payload
            .message
            .as_ref()?
            .chat
            .as_ref()
            .map(|c| c.id.to_string())
    })
}

pub fn extract_user_id(payload: &TelegramWebhookPayload) -> Option<String> {
    payload.telegram_user_id.clone().or_else(|| {
        payload
            .message
            .as_ref()?
            .from
            .as_ref()
            .map(|u| u.id.to_string())
    })
}
