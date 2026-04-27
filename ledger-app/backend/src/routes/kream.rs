use std::collections::{HashMap, HashSet};

use axum::{
    Json,
    extract::{Multipart, Path, Query, State},
};
use chrono::{DateTime, Duration, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::Postgres;
use uuid::Uuid;

use crate::{
    AppState,
    auth::extractor::AuthUser,
    errors::AppError,
    import::parser,
    models::{KreamKeywordRule, KreamSale},
    services::kream_rules::{normalize_keyword, sql_keyword_pattern},
};

#[derive(Debug, Serialize)]
pub struct KreamUploadResponse {
    pub imported_count: i32,
    pub duplicate_count: i32,
    pub error_count: i32,
    pub sales: Vec<KreamSale>,
}

#[derive(Debug, Serialize)]
pub struct KreamSalesSummary {
    pub total_purchase_price: i64,
    pub total_settlement_price: i64,
    pub total_side_cost: i64,
    pub total_profit: i64,
}

#[derive(Debug, Serialize)]
pub struct KreamSalesResponse {
    pub sales: Vec<KreamSale>,
    pub summary: KreamSalesSummary,
}

#[derive(Debug, Deserialize)]
pub struct CreateKreamSaleRequest {
    pub product_name: String,
    pub purchase_date: NaiveDate,
    pub settlement_date: Option<NaiveDate>,
    pub purchase_price: i64,
    pub settlement_price: Option<i64>,
    pub memo: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateKreamKeywordRuleRequest {
    pub keyword: String,
    pub kream_kind: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateKreamKeywordRuleResponse {
    pub rule: KreamKeywordRule,
    pub applied_count: u64,
}

#[derive(Debug, Deserialize)]
pub struct BulkMarkKreamTransactionsRequest {
    pub transaction_ids: Vec<Uuid>,
    pub kream_kind: String,
}

#[derive(Debug, Serialize)]
pub struct BulkMarkKreamTransactionsResponse {
    pub updated_count: u64,
}

#[derive(Debug, Serialize)]
pub struct KreamLedgerTransaction {
    pub id: Uuid,
    pub transaction_at: DateTime<Utc>,
    pub r#type: String,
    pub amount: i64,
    pub merchant_name: Option<String>,
    pub description: Option<String>,
    pub memo: Option<String>,
    pub sale_id: Option<Uuid>,
    pub sale_code: Option<String>,
    pub product_name: Option<String>,
    pub link_kind: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct KreamCandidateQuery {
    pub kind: Option<String>,
    pub keyword: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct KreamTransactionCandidate {
    pub id: Uuid,
    pub transaction_at: DateTime<Utc>,
    pub r#type: String,
    pub amount: i64,
    pub merchant_name: Option<String>,
    pub description: Option<String>,
    pub memo: Option<String>,
    pub scope: String,
}

#[derive(Debug, Deserialize)]
pub struct MatchKreamTransactionRequest {
    pub transaction_id: Uuid,
    pub kind: String,
}

#[derive(Debug, Deserialize)]
pub struct UnmatchKreamTransactionRequest {
    pub kind: String,
}

#[derive(Debug, Deserialize)]
pub struct MarkKreamTransactionRequest {
    pub transaction_id: Uuid,
    pub scope: String,
    pub kream_kind: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ParsedKreamRow {
    product_name: String,
    purchase_date: NaiveDate,
    settlement_date: NaiveDate,
    purchase_price: i64,
    settlement_price: i64,
    side_cost: i64,
    external_id: Option<String>,
    source_row_index: i32,
    raw_data: serde_json::Value,
}

#[derive(Debug)]
struct PreparedKreamSale {
    sale_code: String,
    dedupe_key: String,
    row: ParsedKreamRow,
}

pub async fn upload_kream_sales(
    State(state): State<AppState>,
    auth: AuthUser,
    mut multipart: Multipart,
) -> Result<Json<KreamUploadResponse>, AppError> {
    auth.require_admin()?;

    let mut file_bytes: Option<Vec<u8>> = None;
    let mut filename: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::BadRequest("multipart payload could not be read".to_string()))?
    {
        if field.name().unwrap_or_default() == "file" {
            filename = field.file_name().map(ToString::to_string);
            let bytes = field
                .bytes()
                .await
                .map_err(|_| AppError::BadRequest("file content could not be read".to_string()))?;
            file_bytes = Some(bytes.to_vec());
        }
    }

    let file_bytes = file_bytes
        .ok_or_else(|| AppError::BadRequest("KREAM upload file is required".to_string()))?;
    let parsed_rows = parse_kream_file(filename.as_deref(), &file_bytes)?;
    let prepared_rows = prepare_rows(parsed_rows);

    if prepared_rows.is_empty() {
        return Err(AppError::BadRequest(
            "KREAM file does not contain importable rows".to_string(),
        ));
    }

    let dedupe_keys = prepared_rows
        .iter()
        .map(|row| row.dedupe_key.clone())
        .collect::<Vec<_>>();
    let existing_keys = sqlx::query_scalar::<_, String>(
        "SELECT dedupe_key FROM kream_sales WHERE user_id = $1 AND dedupe_key = ANY($2)",
    )
    .bind(auth.id)
    .bind(&dedupe_keys)
    .fetch_all(&state.pool)
    .await?
    .into_iter()
    .collect::<HashSet<_>>();

    let mut tx = state.pool.begin().await?;
    let mut seen_keys = HashSet::new();
    let mut imported = Vec::new();
    let mut duplicate_count = 0i32;
    let mut error_count = 0i32;

    for prepared in prepared_rows {
        if existing_keys.contains(&prepared.dedupe_key)
            || !seen_keys.insert(prepared.dedupe_key.clone())
        {
            duplicate_count += 1;
            continue;
        }

        match insert_prepared_sale(&mut tx, auth.id, filename.as_deref(), prepared).await {
            Ok(row) => imported.push(row),
            Err(err) => {
                error_count += 1;
                tracing::warn!(error = ?err, "KREAM row import failed");
            }
        }
    }

    tx.commit().await?;

    Ok(Json(KreamUploadResponse {
        imported_count: imported.len() as i32,
        duplicate_count,
        error_count,
        sales: imported,
    }))
}

pub async fn list_kream_sales(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<KreamSalesResponse>, AppError> {
    auth.require_admin()?;

    let sales = sqlx::query_as::<_, KreamSale>(
        r#"
        SELECT *
        FROM kream_sales
        WHERE user_id = $1
        ORDER BY settlement_date DESC NULLS LAST, purchase_date DESC, created_at DESC
        "#,
    )
    .bind(auth.id)
    .fetch_all(&state.pool)
    .await?;

    let common_side_cost = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COALESCE(SUM(amount), 0)::BIGINT
        FROM transactions
        WHERE user_id = $1
          AND scope = 'kream'
          AND kream_kind = 'side_cost'
          AND type = 'expense'
        "#,
    )
    .bind(auth.id)
    .fetch_one(&state.pool)
    .await?;

    let summary = sales.iter().fold(
        KreamSalesSummary {
            total_purchase_price: 0,
            total_settlement_price: 0,
            total_side_cost: common_side_cost,
            total_profit: 0,
        },
        |mut acc, sale| {
            acc.total_purchase_price += sale.purchase_price;
            acc.total_settlement_price += sale.settlement_price;
            if sale.settlement_date.is_some() {
                acc.total_profit += sale.settlement_price - sale.purchase_price;
            }
            acc
        },
    );
    let summary = KreamSalesSummary {
        total_profit: summary.total_profit - summary.total_side_cost,
        ..summary
    };

    Ok(Json(KreamSalesResponse { sales, summary }))
}

pub async fn create_kream_sale(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<CreateKreamSaleRequest>,
) -> Result<Json<KreamSale>, AppError> {
    auth.require_admin()?;

    let product_name = payload.product_name.trim();
    if product_name.is_empty() {
        return Err(AppError::BadRequest("product_name is required".to_string()));
    }
    let settlement_price = payload.settlement_price.unwrap_or(0);
    if payload.purchase_price < 0 || settlement_price < 0 {
        return Err(AppError::BadRequest(
            "prices must be greater than or equal to zero".to_string(),
        ));
    }

    let seed = Uuid::new_v4().to_string();
    let user_id = auth.id.to_string();
    let dedupe_key = hash_key(&["kream_sale_manual", &user_id, &seed]);
    let sale_code = format!(
        "KREAM-{}-{}",
        payload.purchase_date.format("%Y%m%d"),
        &dedupe_key[..8]
    );

    let row = sqlx::query_as::<_, KreamSale>(
        r#"
        INSERT INTO kream_sales (
            id, user_id, sale_code, product_name, purchase_date, settlement_date,
            purchase_price, settlement_price, side_cost,
            purchase_transaction_id, settlement_transaction_id, side_cost_transaction_id,
            dedupe_key, source_filename, source_row_index, raw_data, memo
        )
        VALUES (
            $1, $2, $3, $4, $5, $6,
            $7, $8, 0,
            NULL, NULL, NULL,
            $9, NULL, NULL, $10, $11
        )
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(auth.id)
    .bind(sale_code)
    .bind(product_name)
    .bind(payload.purchase_date)
    .bind(payload.settlement_date)
    .bind(payload.purchase_price)
    .bind(settlement_price)
    .bind(dedupe_key)
    .bind(serde_json::json!({
        "source": "manual",
        "product_name": product_name,
        "purchase_date": payload.purchase_date,
        "settlement_date": payload.settlement_date,
        "purchase_price": payload.purchase_price,
        "settlement_price": settlement_price,
    }))
    .bind(payload.memo)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(row))
}

pub async fn list_kream_keyword_rules(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<KreamKeywordRule>>, AppError> {
    auth.require_admin()?;

    let rows = sqlx::query_as::<_, KreamKeywordRule>(
        r#"
        SELECT *
        FROM kream_keyword_rules
        WHERE user_id = $1
          AND is_active = true
        ORDER BY created_at DESC
        "#,
    )
    .bind(auth.id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(rows))
}

pub async fn create_kream_keyword_rule(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<CreateKreamKeywordRuleRequest>,
) -> Result<Json<CreateKreamKeywordRuleResponse>, AppError> {
    auth.require_admin()?;

    let keyword = payload.keyword.trim();
    if keyword.is_empty() {
        return Err(AppError::BadRequest("keyword is required".to_string()));
    }

    let kind = payload
        .kream_kind
        .unwrap_or_else(|| "side_cost".to_string());
    if kind != "side_cost" {
        return Err(AppError::BadRequest(
            "only side_cost keyword rules are supported for now".to_string(),
        ));
    }

    let normalized = normalize_keyword(keyword);
    let mut tx = state.pool.begin().await?;
    let rule = sqlx::query_as::<_, KreamKeywordRule>(
        r#"
        INSERT INTO kream_keyword_rules (
            id, user_id, keyword, keyword_normalized, kream_kind, is_active
        )
        VALUES ($1, $2, $3, $4, $5, true)
        ON CONFLICT (user_id, keyword_normalized, kream_kind)
        DO UPDATE SET
            keyword = EXCLUDED.keyword,
            is_active = true,
            updated_at = now()
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(auth.id)
    .bind(keyword)
    .bind(&normalized)
    .bind(&kind)
    .fetch_one(&mut *tx)
    .await?;

    let pattern = sql_keyword_pattern(keyword);
    let applied = sqlx::query(
        r#"
        UPDATE transactions
        SET scope = 'kream',
            kream_kind = 'side_cost',
            updated_at = now()
        WHERE user_id = $1
          AND type = 'expense'
          AND scope = 'personal'
          AND replace(lower(
                coalesce(merchant_name, '') || ' ' ||
                coalesce(description, '') || ' ' ||
                coalesce(memo, '')
              ), ' ', '') LIKE $2
        "#,
    )
    .bind(auth.id)
    .bind(pattern)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(CreateKreamKeywordRuleResponse {
        rule,
        applied_count: applied.rows_affected(),
    }))
}

pub async fn delete_kream_keyword_rule(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    auth.require_admin()?;

    let result = sqlx::query(
        r#"
        UPDATE kream_keyword_rules
        SET is_active = false,
            updated_at = now()
        WHERE id = $1
          AND user_id = $2
        "#,
    )
    .bind(id)
    .bind(auth.id)
    .execute(&state.pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok(Json(serde_json::json!({"message": "deleted"})))
}

#[derive(Debug, Serialize)]
pub struct ApplyKreamKeywordRulesResponse {
    pub applied_count: u64,
    pub rule_count: u64,
}

pub async fn apply_kream_keyword_rules(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<ApplyKreamKeywordRulesResponse>, AppError> {
    auth.require_admin()?;

    let rules = sqlx::query_as::<_, (String, String)>(
        r#"
        SELECT keyword_normalized, kream_kind
        FROM kream_keyword_rules
        WHERE user_id = $1
          AND is_active = true
        ORDER BY created_at ASC
        "#,
    )
    .bind(auth.id)
    .fetch_all(&state.pool)
    .await?;

    let rule_count = rules.len() as u64;
    if rule_count == 0 {
        return Ok(Json(ApplyKreamKeywordRulesResponse { applied_count: 0, rule_count: 0 }));
    }

    // 각 규칙을 순서대로 독립 UPDATE — 이미 kream_kind가 지정된 거래는 건너뜀
    let mut total_applied: u64 = 0;

    for (keyword, kind) in &rules {
        // kream_kind 값은 DB CHECK 제약(purchase|settlement|side_cost)으로 보장되므로 safe
        let pattern = format!("%{}%", keyword);
        let result = sqlx::query(
            r#"
            UPDATE transactions
            SET scope = 'kream',
                kream_kind = $3,
                updated_at = now()
            WHERE user_id = $1
              AND type = 'expense'
              AND replace(lower(
                    coalesce(merchant_name, '') ||
                    coalesce(description, '') ||
                    coalesce(memo, '')
                  ), ' ', '') LIKE $2
              AND NOT (scope = 'kream' AND kream_kind IS NOT NULL)
            "#,
        )
        .bind(auth.id)
        .bind(&pattern)
        .bind(kind)
        .execute(&state.pool)
        .await?;
        total_applied += result.rows_affected();
    }

    Ok(Json(ApplyKreamKeywordRulesResponse {
        applied_count: total_applied,
        rule_count,
    }))
}

pub async fn list_kream_ledger(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<KreamLedgerTransaction>>, AppError> {
    auth.require_admin()?;

    let rows = sqlx::query_as::<
        _,
        (
            Uuid,
            DateTime<Utc>,
            String,
            i64,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<Uuid>,
            Option<String>,
            Option<String>,
            Option<String>,
        ),
    >(
        r#"
        SELECT
            t.id,
            t.transaction_at,
            t.type,
            t.amount,
            t.merchant_name,
            t.description,
            t.memo,
            sale_link.sale_id,
            sale_link.sale_code,
            sale_link.product_name,
            COALESCE(sale_link.link_kind, t.kream_kind) AS link_kind
        FROM transactions t
        LEFT JOIN LATERAL (
            SELECT
                ks.id AS sale_id,
                ks.sale_code,
                ks.product_name,
                CASE
                    WHEN ks.purchase_transaction_id = t.id THEN 'purchase'
                    WHEN ks.settlement_transaction_id = t.id THEN 'settlement'
                END AS link_kind
            FROM kream_sales ks
            WHERE ks.user_id = t.user_id
              AND (
                ks.purchase_transaction_id = t.id
                OR ks.settlement_transaction_id = t.id
              )
            ORDER BY ks.updated_at DESC, ks.created_at DESC
            LIMIT 1
        ) sale_link ON TRUE
        WHERE t.user_id = $1
          AND t.scope = 'kream'
        ORDER BY t.transaction_at DESC, t.created_at DESC
        "#,
    )
    .bind(auth.id)
    .fetch_all(&state.pool)
    .await?
    .into_iter()
    .map(
        |(
            id,
            transaction_at,
            typ,
            amount,
            merchant_name,
            description,
            memo,
            sale_id,
            sale_code,
            product_name,
            link_kind,
        )| {
            KreamLedgerTransaction {
                id,
                transaction_at,
                r#type: typ,
                amount,
                merchant_name,
                description,
                memo,
                sale_id,
                sale_code,
                product_name,
                link_kind,
            }
        },
    )
    .collect::<Vec<_>>();

    Ok(Json(rows))
}

pub async fn list_kream_match_candidates(
    Query(query): Query<KreamCandidateQuery>,
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<KreamTransactionCandidate>>, AppError> {
    auth.require_admin()?;

    let typ = match query.kind.as_deref() {
        Some("settlement") => "income",
        Some("purchase") | Some("side_cost") | None => "expense",
        Some(_) => {
            return Err(AppError::BadRequest(
                "kind must be purchase, settlement, or side_cost".to_string(),
            ));
        }
    };
    let limit = query.limit.unwrap_or(200).clamp(1, 500);
    let keyword = query.keyword.unwrap_or_default();
    let keyword = keyword.trim();

    let rows = if keyword.is_empty() {
        sqlx::query_as::<
            _,
            (
                Uuid,
                DateTime<Utc>,
                String,
                i64,
                Option<String>,
                Option<String>,
                Option<String>,
                String,
            ),
        >(
            r#"
            SELECT id, transaction_at, type, amount, merchant_name, description, memo, scope
            FROM transactions
            WHERE user_id = $1
              AND type = $2
            ORDER BY transaction_at DESC, created_at DESC
            LIMIT $3
            "#,
        )
        .bind(auth.id)
        .bind(typ)
        .bind(limit)
        .fetch_all(&state.pool)
        .await?
    } else {
        let pattern = format!("%{}%", keyword);
        sqlx::query_as::<
            _,
            (
                Uuid,
                DateTime<Utc>,
                String,
                i64,
                Option<String>,
                Option<String>,
                Option<String>,
                String,
            ),
        >(
            r#"
            SELECT id, transaction_at, type, amount, merchant_name, description, memo, scope
            FROM transactions
            WHERE user_id = $1
              AND type = $2
              AND (
                merchant_name ILIKE $3
                OR description ILIKE $3
                OR memo ILIKE $3
              )
            ORDER BY transaction_at DESC, created_at DESC
            LIMIT $4
            "#,
        )
        .bind(auth.id)
        .bind(typ)
        .bind(pattern)
        .bind(limit)
        .fetch_all(&state.pool)
        .await?
    };

    let result = rows
        .into_iter()
        .map(
            |(id, transaction_at, typ, amount, merchant_name, description, memo, scope)| {
                KreamTransactionCandidate {
                    id,
                    transaction_at,
                    r#type: typ,
                    amount,
                    merchant_name,
                    description,
                    memo,
                    scope,
                }
            },
        )
        .collect();

    Ok(Json(result))
}

pub async fn match_kream_transaction(
    Path(sale_id): Path<Uuid>,
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<MatchKreamTransactionRequest>,
) -> Result<Json<KreamSale>, AppError> {
    auth.require_admin()?;
    let field = match payload.kind.as_str() {
        "purchase" => "purchase_transaction_id",
        "settlement" => "settlement_transaction_id",
        _ => {
            return Err(AppError::BadRequest(
                "kind must be purchase or settlement".to_string(),
            ));
        }
    };
    let expected_type = if payload.kind == "settlement" {
        "income"
    } else {
        "expense"
    };

    let mut tx = state.pool.begin().await?;
    let sale =
        sqlx::query_as::<_, KreamSale>("SELECT * FROM kream_sales WHERE id = $1 AND user_id = $2")
            .bind(sale_id)
            .bind(auth.id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(AppError::NotFound)?;

    let (transaction_type, transaction_at, transaction_amount) =
        sqlx::query_as::<_, (String, DateTime<Utc>, i64)>(
            "SELECT type, transaction_at, amount FROM transactions WHERE id = $1 AND user_id = $2",
        )
        .bind(payload.transaction_id)
        .bind(auth.id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(AppError::NotFound)?;

    if transaction_type != expected_type {
        return Err(AppError::BadRequest(format!(
            "{} matches require a {} transaction",
            payload.kind, expected_type
        )));
    }

    sqlx::query(
        "UPDATE transactions SET scope = 'kream', kream_kind = $3, updated_at = now() WHERE id = $1 AND user_id = $2",
    )
        .bind(payload.transaction_id)
        .bind(auth.id)
        .bind(&payload.kind)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        r#"
        UPDATE kream_sales
        SET purchase_transaction_id = CASE WHEN purchase_transaction_id = $2 THEN NULL ELSE purchase_transaction_id END,
            settlement_transaction_id = CASE WHEN settlement_transaction_id = $2 THEN NULL ELSE settlement_transaction_id END,
            updated_at = now()
        WHERE user_id = $1
          AND (
            purchase_transaction_id = $2
            OR settlement_transaction_id = $2
          )
        "#,
    )
    .bind(auth.id)
    .bind(payload.transaction_id)
    .execute(&mut *tx)
    .await?;

    let old_transaction_id = match payload.kind.as_str() {
        "purchase" => sale.purchase_transaction_id,
        "settlement" => sale.settlement_transaction_id,
        _ => None,
    };

    let updated = if payload.kind == "settlement" {
        let update_sql = format!(
            "UPDATE kream_sales SET {field} = $3, settlement_date = COALESCE(settlement_date, ($4 AT TIME ZONE 'Asia/Seoul')::date), settlement_price = CASE WHEN settlement_price = 0 THEN $5 ELSE settlement_price END, updated_at = now() WHERE id = $1 AND user_id = $2 RETURNING *"
        );
        sqlx::query_as::<_, KreamSale>(&update_sql)
            .bind(sale_id)
            .bind(auth.id)
            .bind(payload.transaction_id)
            .bind(transaction_at)
            .bind(transaction_amount)
            .fetch_one(&mut *tx)
            .await?
    } else {
        let update_sql = format!(
            "UPDATE kream_sales SET {field} = $3, updated_at = now() WHERE id = $1 AND user_id = $2 RETURNING *"
        );
        sqlx::query_as::<_, KreamSale>(&update_sql)
            .bind(sale_id)
            .bind(auth.id)
            .bind(payload.transaction_id)
            .fetch_one(&mut *tx)
            .await?
    };

    if old_transaction_id != Some(payload.transaction_id) {
        if let Some(old_id) = old_transaction_id {
            delete_generated_kream_transaction(&mut tx, auth.id, old_id).await?;
        }
    }

    tx.commit().await?;
    Ok(Json(updated))
}

pub async fn unmatch_kream_transaction(
    Path(sale_id): Path<Uuid>,
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<UnmatchKreamTransactionRequest>,
) -> Result<Json<KreamSale>, AppError> {
    auth.require_admin()?;
    let field = match payload.kind.as_str() {
        "purchase" => "purchase_transaction_id",
        "settlement" => "settlement_transaction_id",
        _ => {
            return Err(AppError::BadRequest(
                "kind must be purchase or settlement".to_string(),
            ));
        }
    };

    let mut tx = state.pool.begin().await?;
    let sale =
        sqlx::query_as::<_, KreamSale>("SELECT * FROM kream_sales WHERE id = $1 AND user_id = $2")
            .bind(sale_id)
            .bind(auth.id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(AppError::NotFound)?;

    let old_transaction_id = match payload.kind.as_str() {
        "purchase" => sale.purchase_transaction_id,
        "settlement" => sale.settlement_transaction_id,
        _ => None,
    };

    let update_sql = format!(
        "UPDATE kream_sales SET {field} = NULL, updated_at = now() WHERE id = $1 AND user_id = $2 RETURNING *"
    );
    let updated = sqlx::query_as::<_, KreamSale>(&update_sql)
        .bind(sale_id)
        .bind(auth.id)
        .fetch_one(&mut *tx)
        .await?;

    if let Some(old_id) = old_transaction_id {
        delete_generated_kream_transaction(&mut tx, auth.id, old_id).await?;
    }

    tx.commit().await?;
    Ok(Json(updated))
}

pub async fn mark_kream_transaction(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<MarkKreamTransactionRequest>,
) -> Result<Json<KreamTransactionCandidate>, AppError> {
    auth.require_admin()?;
    if !matches!(payload.scope.as_str(), "personal" | "kream") {
        return Err(AppError::BadRequest(
            "scope must be personal or kream".to_string(),
        ));
    }
    if let Some(kind) = payload.kream_kind.as_deref() {
        if !matches!(kind, "purchase" | "settlement" | "side_cost") {
            return Err(AppError::BadRequest(
                "kream_kind must be purchase, settlement, or side_cost".to_string(),
            ));
        }
    }
    let kream_kind = if payload.scope == "kream" {
        payload.kream_kind.as_deref()
    } else {
        None
    };

    let mut tx = state.pool.begin().await?;

    if payload.scope == "personal" || kream_kind == Some("side_cost") {
        sqlx::query(
            r#"
            UPDATE kream_sales
            SET purchase_transaction_id = CASE WHEN purchase_transaction_id = $2 THEN NULL ELSE purchase_transaction_id END,
                settlement_transaction_id = CASE WHEN settlement_transaction_id = $2 THEN NULL ELSE settlement_transaction_id END,
                side_cost_transaction_id = CASE WHEN side_cost_transaction_id = $2 THEN NULL ELSE side_cost_transaction_id END,
                updated_at = now()
            WHERE user_id = $1
              AND (
                purchase_transaction_id = $2
                OR settlement_transaction_id = $2
                OR side_cost_transaction_id = $2
              )
            "#,
        )
        .bind(auth.id)
        .bind(payload.transaction_id)
        .execute(&mut *tx)
        .await?;
    }

    let row = sqlx::query_as::<
        _,
        (
            Uuid,
            DateTime<Utc>,
            String,
            i64,
            Option<String>,
            Option<String>,
            Option<String>,
            String,
        ),
    >(
        r#"
        UPDATE transactions
        SET scope = $3, kream_kind = $4, updated_at = now()
        WHERE id = $1 AND user_id = $2
        RETURNING id, transaction_at, type, amount, merchant_name, description, memo, scope
        "#,
    )
    .bind(payload.transaction_id)
    .bind(auth.id)
    .bind(&payload.scope)
    .bind(kream_kind)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;

    tx.commit().await?;

    let (id, transaction_at, typ, amount, merchant_name, description, memo, scope) = row;
    Ok(Json(KreamTransactionCandidate {
        id,
        transaction_at,
        r#type: typ,
        amount,
        merchant_name,
        description,
        memo,
        scope,
    }))
}

pub async fn bulk_mark_kream_transactions(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<BulkMarkKreamTransactionsRequest>,
) -> Result<Json<BulkMarkKreamTransactionsResponse>, AppError> {
    auth.require_admin()?;
    if payload.kream_kind != "side_cost" {
        return Err(AppError::BadRequest(
            "bulk marking currently supports only side_cost".to_string(),
        ));
    }
    if payload.transaction_ids.is_empty() {
        return Ok(Json(BulkMarkKreamTransactionsResponse { updated_count: 0 }));
    }

    let mut tx = state.pool.begin().await?;

    sqlx::query(
        r#"
        UPDATE kream_sales
        SET purchase_transaction_id = CASE WHEN purchase_transaction_id = ANY($2) THEN NULL ELSE purchase_transaction_id END,
            settlement_transaction_id = CASE WHEN settlement_transaction_id = ANY($2) THEN NULL ELSE settlement_transaction_id END,
            side_cost_transaction_id = CASE WHEN side_cost_transaction_id = ANY($2) THEN NULL ELSE side_cost_transaction_id END,
            updated_at = now()
        WHERE user_id = $1
          AND (
            purchase_transaction_id = ANY($2)
            OR settlement_transaction_id = ANY($2)
            OR side_cost_transaction_id = ANY($2)
          )
        "#,
    )
    .bind(auth.id)
    .bind(&payload.transaction_ids)
    .execute(&mut *tx)
    .await?;

    let updated = sqlx::query(
        r#"
        UPDATE transactions
        SET scope = 'kream',
            kream_kind = 'side_cost',
            updated_at = now()
        WHERE user_id = $1
          AND id = ANY($2)
          AND type = 'expense'
        "#,
    )
    .bind(auth.id)
    .bind(&payload.transaction_ids)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(BulkMarkKreamTransactionsResponse {
        updated_count: updated.rows_affected(),
    }))
}

async fn insert_prepared_sale(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    user_id: Uuid,
    source_filename: Option<&str>,
    prepared: PreparedKreamSale,
) -> Result<KreamSale, sqlx::Error> {
    let row = prepared.row;
    let purchase_tx_id = insert_kream_transaction(
        tx,
        user_id,
        kst_date_time(row.purchase_date, 9),
        "expense",
        row.purchase_price,
        "KREAM purchase",
        &row.product_name,
        &format!("{}:purchase", prepared.dedupe_key),
        row.raw_data.clone(),
        Some("purchase"),
    )
    .await?;

    let settlement_tx_id = insert_kream_transaction(
        tx,
        user_id,
        kst_date_time(row.settlement_date, 9),
        "income",
        row.settlement_price,
        "KREAM settlement",
        &row.product_name,
        &format!("{}:settlement", prepared.dedupe_key),
        row.raw_data.clone(),
        Some("settlement"),
    )
    .await?;

    if row.side_cost > 0 {
        insert_kream_transaction(
            tx,
            user_id,
            kst_date_time(row.purchase_date, 10),
            "expense",
            row.side_cost,
            "KREAM side cost",
            &row.product_name,
            &format!("{}:side_cost", prepared.dedupe_key),
            row.raw_data.clone(),
            Some("side_cost"),
        )
        .await?;
    }

    sqlx::query_as::<_, KreamSale>(
        r#"
        INSERT INTO kream_sales (
            id, user_id, sale_code, product_name, purchase_date, settlement_date,
            purchase_price, settlement_price, side_cost,
            purchase_transaction_id, settlement_transaction_id, side_cost_transaction_id,
            dedupe_key, source_filename, source_row_index, raw_data
        )
        VALUES (
            $1, $2, $3, $4, $5, $6,
            $7, $8, 0,
            $9, $10, NULL,
            $11, $12, $13, $14
        )
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(prepared.sale_code)
    .bind(row.product_name)
    .bind(row.purchase_date)
    .bind(row.settlement_date)
    .bind(row.purchase_price)
    .bind(row.settlement_price)
    .bind(purchase_tx_id)
    .bind(settlement_tx_id)
    .bind(prepared.dedupe_key)
    .bind(source_filename.map(ToString::to_string))
    .bind(Some(row.source_row_index))
    .bind(row.raw_data)
    .fetch_one(&mut **tx)
    .await
}

async fn delete_generated_kream_transaction(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    user_id: Uuid,
    transaction_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        DELETE FROM transactions
        WHERE id = $1
          AND user_id = $2
          AND scope = 'kream'
          AND source_institution = 'kream'
          AND source_file_id IS NULL
          AND account_id IS NULL
          AND card_id IS NULL
        "#,
    )
    .bind(transaction_id)
    .bind(user_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn insert_kream_transaction(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    user_id: Uuid,
    transaction_at: DateTime<Utc>,
    typ: &str,
    amount: i64,
    merchant_name: &str,
    description: &str,
    key_seed: &str,
    raw_data: serde_json::Value,
    kream_kind: Option<&str>,
) -> Result<Uuid, sqlx::Error> {
    let id = Uuid::new_v4();
    let dedupe_key = hash_key(&["kream_transaction", key_seed]);

    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO transactions (
            id, user_id, transaction_at, posted_at, type, amount,
            merchant_name, description, category_id, account_id, card_id,
            source_type, source_institution, source_file_id, balance_after,
            raw_data, dedupe_key, memo, scope, kream_kind
        )
        VALUES (
            $1, $2, $3, NULL, $4, $5,
            $6, $7, NULL, NULL, NULL,
            'file', 'kream', NULL, NULL,
            $8, $9, NULL, 'kream', $10
        )
        ON CONFLICT (user_id, dedupe_key)
        DO UPDATE SET
            scope = 'kream',
            kream_kind = EXCLUDED.kream_kind,
            updated_at = now()
        RETURNING id
        "#,
    )
    .bind(id)
    .bind(user_id)
    .bind(transaction_at)
    .bind(typ)
    .bind(amount)
    .bind(merchant_name)
    .bind(description)
    .bind(raw_data)
    .bind(dedupe_key)
    .bind(kream_kind)
    .fetch_one(&mut **tx)
    .await
}

fn parse_kream_file(
    filename: Option<&str>,
    content: &[u8],
) -> Result<Vec<ParsedKreamRow>, AppError> {
    let ext = filename
        .and_then(|name| name.rsplit('.').next())
        .unwrap_or("csv")
        .to_lowercase();

    let (headers, rows) = if ext == "xls" || ext == "xlsx" {
        sheet_table(content)?
    } else {
        csv_table(content)?
    };

    rows_to_kream_rows(headers, rows)
}

fn csv_table(content: &[u8]) -> Result<(Vec<String>, Vec<Vec<String>>), AppError> {
    let text = String::from_utf8_lossy(content);
    let first_line = text.lines().next().unwrap_or_default();
    let delimiter = if first_line.contains('\t') {
        b'\t'
    } else {
        b','
    };
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .delimiter(delimiter)
        .from_reader(text.as_bytes());

    let headers = reader
        .headers()
        .map_err(|err| AppError::BadRequest(err.to_string()))?
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record.map_err(|err| AppError::BadRequest(err.to_string()))?;
        rows.push(record.iter().map(ToString::to_string).collect());
    }
    Ok((headers, rows))
}

fn sheet_table(content: &[u8]) -> Result<(Vec<String>, Vec<Vec<String>>), AppError> {
    let rows = parser::sheet_rows(content).map_err(|err| AppError::BadRequest(err.to_string()))?;
    let header_index = rows
        .iter()
        .position(|row| {
            let normalized = row.iter().map(|v| normalize_header(v)).collect::<Vec<_>>();
            has_any(&normalized, &["상품명", "productname"])
                && has_any(&normalized, &["구매가격", "purchaseprice"])
                && has_any(&normalized, &["정산가격", "settlementprice"])
        })
        .ok_or_else(|| AppError::BadRequest("KREAM header row could not be found".to_string()))?;

    let headers = rows[header_index].clone();
    let data_rows = rows.into_iter().skip(header_index + 1).collect::<Vec<_>>();
    Ok((headers, data_rows))
}

fn rows_to_kream_rows(
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
) -> Result<Vec<ParsedKreamRow>, AppError> {
    let product_col = find_col(&headers, &["상품명", "productname", "product"]);
    let purchase_date_col = find_col(&headers, &["구매날짜", "구매일", "purchasedate"]);
    let settlement_date_col = find_col(&headers, &["정산날짜", "정산일", "settlementdate"]);
    let purchase_price_col = find_col(&headers, &["구매가격", "구매가", "purchaseprice", "cost"]);
    let settlement_price_col = find_col(
        &headers,
        &["정산가격", "정산가", "settlementprice", "settlement"],
    );
    let side_cost_col = find_col(
        &headers,
        &["부대비용", "배송비", "택배비", "sidecost", "additionalcost"],
    );
    let external_id_col = find_col(
        &headers,
        &["주문번호", "판매번호", "관리번호", "orderno", "orderid"],
    );

    let product_col = require_col(product_col, "상품명")?;
    let purchase_date_col = require_col(purchase_date_col, "구매 날짜")?;
    let settlement_date_col = require_col(settlement_date_col, "정산 날짜")?;
    let purchase_price_col = require_col(purchase_price_col, "구매가격")?;
    let settlement_price_col = require_col(settlement_price_col, "정산 가격")?;

    let mut parsed = Vec::new();
    for (idx, row) in rows.into_iter().enumerate() {
        if row.iter().all(|value| value.trim().is_empty()) {
            continue;
        }

        let product_name = cell(&row, product_col).trim().to_string();
        if product_name.is_empty() {
            continue;
        }

        let purchase_date = parse_date(cell(&row, purchase_date_col), "구매 날짜")?;
        let settlement_date = parse_date(cell(&row, settlement_date_col), "정산 날짜")?;
        let purchase_price = parse_money(cell(&row, purchase_price_col), "구매가격")?;
        let settlement_price = parse_money(cell(&row, settlement_price_col), "정산 가격")?;
        let side_cost = if let Some(col) = side_cost_col {
            parse_optional_money(cell(&row, col))?.unwrap_or(0)
        } else {
            0
        };
        let external_id = external_id_col
            .map(|col| cell(&row, col).trim().to_string())
            .filter(|value| !value.is_empty());

        parsed.push(ParsedKreamRow {
            product_name: product_name.clone(),
            purchase_date,
            settlement_date,
            purchase_price,
            settlement_price,
            side_cost,
            external_id: external_id.clone(),
            source_row_index: idx as i32 + 2,
            raw_data: serde_json::json!({
                "product_name": product_name,
                "purchase_date": purchase_date,
                "settlement_date": settlement_date,
                "purchase_price": purchase_price,
                "settlement_price": settlement_price,
                "side_cost": side_cost,
                "external_id": external_id,
            }),
        });
    }

    Ok(parsed)
}

fn prepare_rows(rows: Vec<ParsedKreamRow>) -> Vec<PreparedKreamSale> {
    let mut occurrence_by_base = HashMap::<String, i32>::new();

    rows.into_iter()
        .map(|row| {
            let base_key = if let Some(external_id) = row.external_id.as_deref() {
                format!("external|{}", normalize_value(external_id))
            } else {
                format!(
                    "row|{}|{}|{}|{}|{}",
                    normalize_value(&row.product_name),
                    row.purchase_date,
                    row.settlement_date,
                    row.purchase_price,
                    row.settlement_price
                )
            };
            let occurrence = occurrence_by_base.entry(base_key.clone()).or_insert(0);
            *occurrence += 1;
            let dedupe_seed = if row.external_id.is_some() {
                base_key
            } else {
                format!("{}|{}", base_key, occurrence)
            };
            let dedupe_key = hash_key(&["kream_sale", &dedupe_seed]);
            let sale_code = format!(
                "KREAM-{}-{}",
                row.purchase_date.format("%Y%m%d"),
                &dedupe_key[..8]
            );

            PreparedKreamSale {
                sale_code,
                dedupe_key,
                row,
            }
        })
        .collect()
}

fn find_col(headers: &[String], candidates: &[&str]) -> Option<usize> {
    let normalized_candidates = candidates
        .iter()
        .map(|candidate| normalize_header(candidate))
        .collect::<HashSet<_>>();

    headers
        .iter()
        .position(|header| normalized_candidates.contains(&normalize_header(header)))
}

fn require_col(value: Option<usize>, label: &str) -> Result<usize, AppError> {
    value.ok_or_else(|| AppError::BadRequest(format!("{label} column is required")))
}

fn has_any(values: &[String], candidates: &[&str]) -> bool {
    let candidates = candidates
        .iter()
        .map(|candidate| normalize_header(candidate))
        .collect::<HashSet<_>>();
    values.iter().any(|value| candidates.contains(value))
}

fn cell(row: &[String], idx: usize) -> &str {
    row.get(idx).map(|value| value.as_str()).unwrap_or_default()
}

fn parse_date(value: &str, label: &str) -> Result<NaiveDate, AppError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::BadRequest(format!("{label} is required")));
    }

    for fmt in [
        "%Y-%m-%d",
        "%Y.%m.%d",
        "%Y/%m/%d",
        "%Y%m%d",
        "%Y-%m-%d %H:%M:%S",
        "%Y.%m.%d %H:%M:%S",
        "%m/%d/%Y",
    ] {
        if let Ok(date) = NaiveDate::parse_from_str(value, fmt) {
            return Ok(date);
        }
        if let Ok(datetime) = NaiveDateTime::parse_from_str(value, fmt) {
            return Ok(datetime.date());
        }
    }

    if let Ok(serial) = value.parse::<f64>() {
        if (20000.0..70000.0).contains(&serial) {
            let base = NaiveDate::from_ymd_opt(1899, 12, 30).unwrap();
            return Ok(base + Duration::days(serial.trunc() as i64));
        }
    }

    Err(AppError::BadRequest(format!(
        "{label} has an unsupported date format: {value}"
    )))
}

fn parse_money(value: &str, label: &str) -> Result<i64, AppError> {
    parse_optional_money(value)?.ok_or_else(|| AppError::BadRequest(format!("{label} is required")))
}

fn parse_optional_money(value: &str) -> Result<Option<i64>, AppError> {
    let cleaned = value
        .trim()
        .chars()
        .filter(|ch| ch.is_ascii_digit() || *ch == '-')
        .collect::<String>();

    if cleaned.is_empty() || cleaned == "-" {
        return Ok(None);
    }

    cleaned
        .parse::<i64>()
        .map(Some)
        .map_err(|_| AppError::BadRequest(format!("invalid money value: {value}")))
}

fn normalize_header(value: &str) -> String {
    value
        .chars()
        .filter(|ch| !ch.is_whitespace() && !matches!(ch, '_' | '-' | '/' | '(' | ')'))
        .collect::<String>()
        .to_lowercase()
}

fn normalize_value(value: &str) -> String {
    value
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>()
        .to_lowercase()
}

fn hash_key(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(parts.join("|").as_bytes());
    format!("{:x}", hasher.finalize())
}

fn kst_date_time(date: NaiveDate, hour: u32) -> DateTime<Utc> {
    let kst = FixedOffset::east_opt(9 * 3600).unwrap();
    let local = NaiveDateTime::new(date, NaiveTime::from_hms_opt(hour, 0, 0).unwrap());
    local
        .and_local_timezone(kst)
        .single()
        .unwrap()
        .with_timezone(&Utc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_kream_csv_with_required_columns() {
        let csv = "\
상품명,구매 날짜,정산 날짜,구매가격,정산 가격,부대비용,주문번호
Nike Dunk Low,2026-04-01,2026-04-10,120000,150000,3500,KREAM-1
";

        let rows = parse_kream_file(Some("kream.csv"), csv.as_bytes()).unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].product_name, "Nike Dunk Low");
        assert_eq!(rows[0].purchase_price, 120000);
        assert_eq!(rows[0].settlement_price, 150000);
        assert_eq!(rows[0].side_cost, 3500);
    }

    #[test]
    fn same_product_rows_get_distinct_sale_codes_without_order_id() {
        let csv = "\
상품명,구매 날짜,정산 날짜,구매가격,정산 가격
Nike Dunk Low,2026-04-01,2026-04-10,120000,150000
Nike Dunk Low,2026-04-01,2026-04-10,120000,150000
";

        let rows = parse_kream_file(Some("kream.csv"), csv.as_bytes()).unwrap();
        let prepared = prepare_rows(rows);

        assert_eq!(prepared.len(), 2);
        assert_ne!(prepared[0].sale_code, prepared[1].sale_code);
        assert_ne!(prepared[0].dedupe_key, prepared[1].dedupe_key);
    }
}
