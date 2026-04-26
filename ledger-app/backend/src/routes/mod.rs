use axum::{
    Json, Router,
    routing::{delete, get, post, put},
};

use crate::routes::{
    accounts::{create_account, delete_account, list_accounts, update_account},
    asset_snapshots::{
        create_asset_snapshot, delete_asset_snapshot, get_net_worth, list_asset_snapshots,
        update_asset_snapshot,
    },
    auth::{change_password, login, logout, me},
    card_presets::{create_card_preset, delete_card_preset, list_card_presets, update_card_preset},
    cards::{
        create_card, delete_card, get_card_summary, get_card_transactions, list_cards, update_card,
    },
    categories::{create_category, delete_category, list_categories, update_category},
    category_rules::{create_rule, delete_rule, list_rules},
    dashboard::{calendar_dashboard, daily_dashboard, monthly_dashboard},
    export::{export_backup_json, export_transactions_csv},
    imports::{
        cancel_import, confirm_import, get_import, list_imports, upload_file_import,
        upload_pasted_text_import,
    },
    kream::{
        bulk_mark_kream_transactions, create_kream_keyword_rule, create_kream_sale,
        delete_kream_keyword_rule, list_kream_keyword_rules, list_kream_ledger,
        list_kream_match_candidates, list_kream_sales, mark_kream_transaction,
        match_kream_transaction, unmatch_kream_transaction, upload_kream_sales,
    },
    telegram::{telegram_setup, telegram_webhook},
    transactions::{
        create_transaction, delete_transaction, get_transaction, list_transactions,
        update_transaction,
    },
};

pub mod accounts;
pub mod asset_snapshots;
pub mod auth;
pub mod card_presets;
pub mod cards;
pub mod categories;
pub mod category_rules;
pub mod dashboard;
pub mod export;
pub mod imports;
pub mod kream;
pub mod telegram;
pub mod transactions;

pub async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok"}))
}

pub fn api_router() -> Router<crate::AppState> {
    Router::new()
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/me", get(me))
        .route("/auth/change-password", post(change_password))
        .route(
            "/transactions",
            get(list_transactions).post(create_transaction),
        )
        .route(
            "/transactions/{id}",
            get(get_transaction)
                .put(update_transaction)
                .delete(delete_transaction),
        )
        .route("/accounts", get(list_accounts).post(create_account))
        .route("/accounts/{id}", put(update_account).delete(delete_account))
        .route("/categories", get(list_categories).post(create_category))
        .route(
            "/categories/{id}",
            put(update_category).delete(delete_category),
        )
        .route("/cards", get(list_cards).post(create_card))
        .route("/cards/{id}", put(update_card).delete(delete_card))
        .route("/cards/{id}/summary", get(get_card_summary))
        .route("/cards/{id}/transactions", get(get_card_transactions))
        .route(
            "/card-presets",
            get(list_card_presets).post(create_card_preset),
        )
        .route(
            "/card-presets/{id}",
            put(update_card_preset).delete(delete_card_preset),
        )
        .route("/imports/upload", post(upload_file_import))
        .route("/imports/pasted-text", post(upload_pasted_text_import))
        .route("/imports", get(list_imports))
        .route("/imports/{id}", get(get_import))
        .route("/imports/{id}/confirm", post(confirm_import))
        .route("/imports/{id}/cancel", post(cancel_import))
        .route(
            "/admin/kream-sales",
            get(list_kream_sales).post(create_kream_sale),
        )
        .route("/admin/kream-sales/upload", post(upload_kream_sales))
        .route("/admin/kream-sales/ledger", get(list_kream_ledger))
        .route(
            "/admin/kream-sales/candidates",
            get(list_kream_match_candidates),
        )
        .route(
            "/admin/kream-sales/{id}/match",
            post(match_kream_transaction),
        )
        .route(
            "/admin/kream-sales/{id}/unmatch",
            post(unmatch_kream_transaction),
        )
        .route(
            "/admin/kream-transactions/mark",
            post(mark_kream_transaction),
        )
        .route(
            "/admin/kream-transactions/bulk-mark",
            post(bulk_mark_kream_transactions),
        )
        .route(
            "/admin/kream-keyword-rules",
            get(list_kream_keyword_rules).post(create_kream_keyword_rule),
        )
        .route(
            "/admin/kream-keyword-rules/{id}",
            delete(delete_kream_keyword_rule),
        )
        .route("/dashboard/monthly", get(monthly_dashboard))
        .route("/dashboard/daily", get(daily_dashboard))
        .route("/dashboard/calendar", get(calendar_dashboard))
        .route(
            "/asset-snapshots",
            get(list_asset_snapshots).post(create_asset_snapshot),
        )
        .route(
            "/asset-snapshots/{id}",
            put(update_asset_snapshot).delete(delete_asset_snapshot),
        )
        .route("/assets/net-worth", get(get_net_worth))
        .route("/telegram/webhook", post(telegram_webhook))
        .route("/telegram/setup", post(telegram_setup))
        .route("/export/transactions.csv", get(export_transactions_csv))
        .route("/export/backup.json", get(export_backup_json))
        .route("/category-rules", get(list_rules).post(create_rule))
        .route("/category-rules/{id}", delete(delete_rule))
}
