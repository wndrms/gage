use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: Option<String>,
    pub display_name: String,
    pub password_hash: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Account {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub r#type: String,
    pub institution: Option<String>,
    pub currency: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Category {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub r#type: String,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Card {
    pub id: Uuid,
    pub user_id: Uuid,
    pub issuer: String,
    pub card_name: String,
    pub preset_id: Option<Uuid>,
    pub billing_day: Option<i32>,
    pub payment_day: Option<i32>,
    pub linked_account_id: Option<Uuid>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CardPreset {
    pub id: Uuid,
    pub issuer: String,
    pub card_name: String,
    pub aliases: Vec<String>,
    pub monthly_requirement: Option<i64>,
    pub rules: serde_json::Value,
    pub benefits: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Transaction {
    pub id: Uuid,
    pub user_id: Uuid,
    pub transaction_at: DateTime<Utc>,
    pub posted_at: Option<DateTime<Utc>>,
    pub r#type: String,
    pub amount: i64,
    pub merchant_name: Option<String>,
    pub description: Option<String>,
    pub category_id: Option<Uuid>,
    pub account_id: Option<Uuid>,
    pub card_id: Option<Uuid>,
    pub source_type: String,
    pub source_institution: Option<String>,
    pub source_file_id: Option<Uuid>,
    pub balance_after: Option<i64>,
    pub raw_data: serde_json::Value,
    pub dedupe_key: String,
    pub memo: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ImportRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub source_type: String,
    pub institution: String,
    pub original_filename: Option<String>,
    pub status: String,
    pub raw_text: Option<String>,
    pub parsed_count: i32,
    pub imported_count: i32,
    pub duplicate_count: i32,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ImportRow {
    pub id: Uuid,
    pub import_id: Uuid,
    pub user_id: Uuid,
    pub row_index: i32,
    pub parsed_transaction: serde_json::Value,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AssetSnapshot {
    pub id: Uuid,
    pub user_id: Uuid,
    pub snapshot_date: NaiveDate,
    pub account_id: Uuid,
    pub amount: i64,
    pub memo: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
