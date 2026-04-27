use axum::{
    body::Body,
    extract::State,
    http::{StatusCode, header},
    response::Response,
};
use chrono::Utc;

use crate::{AppState, auth::extractor::AuthUser, errors::AppError, models::Transaction};

pub async fn export_transactions_csv(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Response<Body>, AppError> {
    let transactions = sqlx::query_as::<_, Transaction>(
        "SELECT t.*, cd.card_name FROM transactions t LEFT JOIN cards cd ON t.card_id = cd.id WHERE t.user_id = $1 ORDER BY t.transaction_at DESC",
    )
    .bind(auth.id)
    .fetch_all(&state.pool)
    .await?;

    let mut output = String::new();
    output.push('\u{FEFF}'); // BOM — Excel UTF-8 인식
    output.push_str("거래일시,유형,금액,가맹점,내용,카테고리ID,계좌ID,카드ID,카드명,메모,출처\n");

    for tx in &transactions {
        let row = format!(
            "{},{},{},{},{},{},{},{},{},{},{}\n",
            // 날짜는 숫자만 포함하므로 escape 불필요
            tx.transaction_at.format("%Y-%m-%d %H:%M:%S"),
            csv_cell(&tx.r#type),
            tx.amount,
            csv_cell(tx.merchant_name.as_deref().unwrap_or("")),
            csv_cell(tx.description.as_deref().unwrap_or("")),
            tx.category_id.map(|u| u.to_string()).unwrap_or_default(),
            tx.account_id.map(|u| u.to_string()).unwrap_or_default(),
            tx.card_id.map(|u| u.to_string()).unwrap_or_default(),
            csv_cell(tx.card_name.as_deref().unwrap_or("")),
            csv_cell(tx.memo.as_deref().unwrap_or("")),
            csv_cell(&tx.source_type),
        );
        output.push_str(&row);
    }

    let filename = format!("transactions_{}.csv", Utc::now().format("%Y%m%d_%H%M%S"));

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{filename}\""),
        )
        .body(Body::from(output.into_bytes()))
        .map_err(|_| AppError::Internal)?)
}

pub async fn export_backup_json(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Response<Body>, AppError> {
    // 4개 쿼리를 병렬 실행
    let (transactions, accounts, categories, cards) = tokio::try_join!(
            sqlx::query_as::<_, Transaction>(
                "SELECT t.*, cd.card_name FROM transactions t LEFT JOIN cards cd ON t.card_id = cd.id WHERE t.user_id = $1 ORDER BY t.transaction_at DESC",
            )
            .bind(auth.id)
            .fetch_all(&state.pool),
            sqlx::query_as::<_, crate::models::Account>(
                "SELECT * FROM accounts WHERE user_id = $1",
            )
            .bind(auth.id)
            .fetch_all(&state.pool),
            sqlx::query_as::<_, crate::models::Category>(
                "SELECT * FROM categories WHERE user_id = $1",
            )
            .bind(auth.id)
            .fetch_all(&state.pool),
            sqlx::query_as::<_, crate::models::Card>("SELECT * FROM cards WHERE user_id = $1",)
                .bind(auth.id)
                .fetch_all(&state.pool),
        )?;

    let backup = serde_json::json!({
        "exported_at": Utc::now(),
        "version": "1",
        "transactions": transactions,
        "accounts": accounts,
        "categories": categories,
        "cards": cards,
    });

    let body = serde_json::to_vec_pretty(&backup).map_err(|_| AppError::Internal)?;
    let filename = format!("ledger_backup_{}.json", Utc::now().format("%Y%m%d_%H%M%S"));

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{filename}\""),
        )
        .body(Body::from(body))
        .map_err(|_| AppError::Internal)?)
}

/// CSV formula injection 방어: `=`, `+`, `-`, `@`, `\t`, `\r`로 시작하는 값은
/// 탭을 앞에 붙여 Excel의 수식 실행을 차단합니다.
fn csv_cell(s: &str) -> String {
    let s = if !s.is_empty()
        && s.starts_with(|c: char| matches!(c, '=' | '+' | '-' | '@' | '\t' | '\r'))
    {
        format!("\t{s}")
    } else {
        s.to_string()
    };

    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\t') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s
    }
}
