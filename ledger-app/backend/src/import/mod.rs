pub mod dedupe;
pub mod hyundai_card;
pub mod parser;
pub mod pasted_text;
pub mod samsung_card;
pub mod shinhan_bank;
pub mod shinhan_card;
pub mod watcher;

use anyhow::{Result, anyhow};
use parser::{DetectInput, NormalizedTransaction, TransactionParser};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{ImportRecord, ImportRow};

#[derive(Debug, Clone)]
pub struct ImportSummary {
    pub total: i32,
    pub new_count: i32,
    pub duplicate_count: i32,
    pub error_count: i32,
}

fn parsers() -> Vec<Box<dyn TransactionParser>> {
    vec![
        Box::new(shinhan_bank::ShinhanBankParser),
        Box::new(shinhan_card::ShinhanCardParser),
        Box::new(hyundai_card::HyundaiCardParser),
        Box::new(samsung_card::SamsungCardParser),
        Box::new(pasted_text::PastedCsvTextParser),
    ]
}

fn detect_best_parser<'a>(
    parser_list: &'a [Box<dyn TransactionParser>],
    input: &DetectInput<'_>,
) -> Option<&'a dyn TransactionParser> {
    parser_list
        .iter()
        .map(|p| (p.as_ref(), p.detect(input)))
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .and_then(|(parser, score)| if score >= 0.3 { Some(parser) } else { None })
}

pub async fn create_import_preview(
    pool: &PgPool,
    user_id: Uuid,
    source_type: &str,
    institution: &str,
    original_filename: Option<String>,
    raw_text: Option<String>,
    content: &[u8],
) -> Result<(ImportRecord, ImportSummary)> {
    let import_id = Uuid::new_v4();
    let mut tx = pool.begin().await?;

    let mut import = sqlx::query_as::<_, ImportRecord>(
        r#"
        INSERT INTO imports (id, user_id, source_type, institution, original_filename, status, raw_text)
        VALUES ($1, $2, $3, $4, $5, 'pending', $6)
        RETURNING *
        "#,
    )
    .bind(import_id)
    .bind(user_id)
    .bind(source_type)
    .bind(institution)
    .bind(original_filename)
    .bind(raw_text)
    .fetch_one(&mut *tx)
    .await?;

    let parser_list = parsers();
    let sample_text = String::from_utf8_lossy(content).chars().take(600).collect::<String>();
    let detect_input = DetectInput {
        filename: import.original_filename.as_deref(),
        sample_text: Some(&sample_text),
        content,
    };

    let parser = detect_best_parser(&parser_list, &detect_input).ok_or_else(|| anyhow!("적합한 파서를 찾을 수 없습니다"))?;

    let parse_result = parser.parse(content);
    let rows = match parse_result {
        Ok(rows) => rows,
        Err(err) => {
            let failed = sqlx::query_as::<_, ImportRecord>(
                "UPDATE imports SET status = 'failed', error_message = $2, updated_at = now() WHERE id = $1 RETURNING *",
            )
            .bind(import_id)
            .bind(err.to_string())
            .fetch_one(&mut *tx)
            .await?;
            tx.commit().await?;
            return Ok((
                failed,
                ImportSummary {
                    total: 0,
                    new_count: 0,
                    duplicate_count: 0,
                    error_count: 0,
                },
            ));
        }
    };

    // 파싱된 전체 dedupe_key 목록을 먼저 계산한 뒤 배치 쿼리로 중복 여부를 한 번에 확인합니다.
    let keyed_rows: Vec<(String, NormalizedTransaction)> = rows
        .into_iter()
        .map(|r| {
            let key = normalize_row_dedupe_key(user_id, &r);
            (key, r)
        })
        .collect();

    let all_keys: Vec<String> = keyed_rows.iter().map(|(k, _)| k.clone()).collect();

    let existing_keys: std::collections::HashSet<String> = sqlx::query_scalar::<_, String>(
        "SELECT dedupe_key FROM transactions WHERE user_id = $1 AND dedupe_key = ANY($2)",
    )
    .bind(user_id)
    .bind(&all_keys)
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .collect();

    let total = keyed_rows.len() as i32;
    let mut new_count = 0i32;
    let mut duplicate_count = 0i32;

    for (idx, (dedupe_key, tx_row)) in keyed_rows.into_iter().enumerate() {
        let status = if existing_keys.contains(&dedupe_key) {
            duplicate_count += 1;
            "duplicate"
        } else {
            new_count += 1;
            "new"
        };

        let mut value = serde_json::to_value(tx_row)?;
        value["dedupe_key"] = serde_json::Value::String(dedupe_key);

        sqlx::query(
            r#"
            INSERT INTO import_rows (id, import_id, user_id, row_index, parsed_transaction, status)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(import_id)
        .bind(user_id)
        .bind(idx as i32)
        .bind(value)
        .bind(status)
        .execute(&mut *tx)
        .await?;
    }

    import = sqlx::query_as::<_, ImportRecord>(
        r#"
        UPDATE imports
        SET status = 'parsed',
            parsed_count = $2,
            duplicate_count = $3,
            imported_count = 0,
            updated_at = now()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(import_id)
    .bind(total)
    .bind(duplicate_count)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok((
        import,
        ImportSummary {
            total,
            new_count,
            duplicate_count,
            error_count: 0,
        },
    ))
}

pub async fn confirm_import(pool: &PgPool, user_id: Uuid, import_id: Uuid) -> Result<ImportSummary> {
    let mut tx = pool.begin().await?;

    let import = sqlx::query_as::<_, ImportRecord>(
        "SELECT * FROM imports WHERE id = $1 AND user_id = $2",
    )
    .bind(import_id)
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| anyhow!("가져오기 정보를 찾을 수 없습니다"))?;

    match import.status.as_str() {
        "imported" => {
            return Ok(ImportSummary {
                total: import.parsed_count,
                new_count: import.imported_count,
                duplicate_count: import.duplicate_count,
                error_count: 0,
            });
        }
        "failed" => {
            return Err(anyhow!("이미 취소된 가져오기입니다. 다시 업로드해 주세요."));
        }
        _ => {}
    }

    let rows = sqlx::query_as::<_, ImportRow>(
        "SELECT * FROM import_rows WHERE import_id = $1 AND user_id = $2 ORDER BY row_index ASC",
    )
    .bind(import_id)
    .bind(user_id)
    .fetch_all(&mut *tx)
    .await?;

    let accounts = sqlx::query_as::<_, (Uuid, String)>("SELECT id, name FROM accounts WHERE user_id = $1")
        .bind(user_id)
        .fetch_all(&mut *tx)
        .await?;
    let cards = sqlx::query_as::<_, (Uuid, String)>("SELECT id, card_name FROM cards WHERE user_id = $1")
        .bind(user_id)
        .fetch_all(&mut *tx)
        .await?;

    let account_map = accounts
        .into_iter()
        .map(|(id, name)| (name.trim().to_lowercase(), id))
        .collect::<std::collections::HashMap<_, _>>();
    let mut card_map = std::collections::HashMap::<String, Uuid>::new();
    let mut card_last4_map = std::collections::HashMap::<String, Uuid>::new();
    let mut ambiguous_last4 = std::collections::HashSet::<String>::new();
    for (id, name) in cards {
        let key = normalize_card_lookup_key(&name);
        card_map.entry(key).or_insert(id);

        if let Some(last4) = extract_card_last4(&name) {
            if card_last4_map.insert(last4.clone(), id).is_some() {
                ambiguous_last4.insert(last4);
            }
        }
    }
    for key in ambiguous_last4 {
        card_last4_map.remove(&key);
    }

    let mut imported_count = 0;
    let mut duplicate_count = 0;
    let mut error_count = 0;

    for row in rows {
        if row.status != "new" {
            if row.status == "duplicate" {
                duplicate_count += 1;
            }
            continue;
        }

        let parsed: NormalizedTransaction = serde_json::from_value(row.parsed_transaction.clone())?;
        let source_institution = parsed
            .source_institution
            .clone()
            .or_else(|| Some(import.institution.clone()));
        let account_id = parsed
            .account_name
            .as_ref()
            .and_then(|name| account_map.get(&name.trim().to_lowercase()).copied());
        let mut card_id = None;
        if let Some(raw_card_name) = parsed.card_name.as_ref() {
            let normalized = normalize_card_lookup_key(raw_card_name);
            card_id = card_map.get(&normalized).copied();

            if card_id.is_none() {
                if let Some(last4) = extract_card_last4(raw_card_name) {
                    card_id = card_last4_map.get(&last4).copied();
                }
            }

            if card_id.is_none() {
                let new_card_id = Uuid::new_v4();
                let issuer = infer_card_issuer(source_institution.as_deref(), raw_card_name);
                let display_name = make_card_display_name(&issuer, raw_card_name);

                sqlx::query(
                    r#"
                    INSERT INTO cards (id, user_id, issuer, card_name, is_active)
                    VALUES ($1, $2, $3, $4, true)
                    "#,
                )
                .bind(new_card_id)
                .bind(user_id)
                .bind(&issuer)
                .bind(&display_name)
                .execute(&mut *tx)
                .await?;

                card_map.insert(normalized, new_card_id);
                card_map.insert(normalize_card_lookup_key(&display_name), new_card_id);
                if let Some(last4) = extract_card_last4(raw_card_name) {
                    card_last4_map.entry(last4).or_insert(new_card_id);
                }
                card_id = Some(new_card_id);
            }
        }

        let source_type = match import.source_type.as_str() {
            "pasted_text" => "pasted_text",
            _ => "file",
        };

        let dedupe_key = parsed.dedupe_key.clone().unwrap_or_else(|| {
            normalize_row_dedupe_key(user_id, &parsed)
        });
        let dedupe_key_for_upsert = dedupe_key.clone();
        let source_institution_for_upsert = source_institution.clone();

        // 자동 카테고리 분류
        let category_id = crate::services::auto_categorize::auto_categorize(
            pool,
            user_id,
            parsed.merchant_name.as_deref(),
        ).await;

        let insert = sqlx::query(
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
                $16, $17, NULL
            )
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(parsed.transaction_at)
        .bind(parsed.posted_at)
        .bind(parsed.r#type)
        .bind(parsed.amount)
        .bind(parsed.merchant_name)
        .bind(parsed.description)
        .bind(category_id)
        .bind(account_id)
        .bind(card_id)
        .bind(source_type)
        .bind(source_institution)
        .bind(import_id)
        .bind(parsed.balance_after)
        .bind(parsed.raw_data)
        .bind(dedupe_key)
        .execute(&mut *tx)
        .await;

        match insert {
            Ok(_) => imported_count += 1,
            Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
                sqlx::query(
                    r#"
                    UPDATE transactions
                    SET card_id = COALESCE(card_id, $3),
                        account_id = COALESCE(account_id, $4),
                        source_institution = COALESCE(source_institution, $5),
                        updated_at = now()
                    WHERE user_id = $1
                      AND dedupe_key = $2
                    "#,
                )
                .bind(user_id)
                .bind(dedupe_key_for_upsert)
                .bind(card_id)
                .bind(account_id)
                .bind(source_institution_for_upsert)
                .execute(&mut *tx)
                .await?;
                duplicate_count += 1;
            }
            Err(err) => {
                error_count += 1;
                tracing::warn!(import_id = %import_id, row = row.row_index, error = %err, "거래 저장 실패");
                sqlx::query(
                    "UPDATE import_rows SET status = 'error', error_message = $3 WHERE id = $1 AND user_id = $2",
                )
                .bind(row.id)
                .bind(user_id)
                .bind("저장 중 오류가 발생했습니다")
                .execute(&mut *tx)
                .await?;
            }
        }
    }

    sqlx::query(
        r#"
        UPDATE imports
        SET status = 'imported',
            imported_count = $2,
            duplicate_count = $3,
            updated_at = now()
        WHERE id = $1 AND user_id = $4
        "#,
    )
    .bind(import_id)
    .bind(imported_count)
    .bind(duplicate_count)
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(ImportSummary {
        total: imported_count + duplicate_count + error_count,
        new_count: imported_count,
        duplicate_count,
        error_count,
    })
}

pub async fn first_user_id(pool: &PgPool) -> Result<Option<Uuid>> {
    let user_id = sqlx::query_scalar::<_, Option<Uuid>>("SELECT id FROM users ORDER BY created_at ASC LIMIT 1")
        .fetch_one(pool)
        .await?;
    Ok(user_id)
}

pub async fn process_file_from_path(pool: &PgPool, user_id: Uuid, path: &std::path::Path) -> Result<()> {
    let content = tokio::fs::read(path).await?;
    let filename = path.file_name().map(|v| v.to_string_lossy().to_string());

    let lower = filename.clone().unwrap_or_default().to_lowercase();
    let source_type = if lower.ends_with(".xlsx") {
        "xlsx"
    } else if lower.ends_with(".xls") {
        "xls"
    } else {
        "csv"
    };

    let institution = detect_institution_from_filename(filename.as_deref()).unwrap_or_else(|| "unknown".to_string());

    let _ = create_import_preview(
        pool,
        user_id,
        source_type,
        &institution,
        filename,
        None,
        &content,
    )
    .await?;

    Ok(())
}

pub fn detect_institution_from_filename(filename: Option<&str>) -> Option<String> {
    let name = filename?.to_lowercase();
    if name.contains("shinhan") || name.contains("신한") {
        if name.contains("card") || name.contains("카드") {
            Some("shinhan_card".to_string())
        } else {
            Some("shinhan_bank".to_string())
        }
    } else if name.contains("hyundai") || name.contains("현대") {
        Some("hyundai_card".to_string())
    } else if name.contains("samsung") || name.contains("삼성") {
        Some("samsung_card".to_string())
    } else if name.contains("bc") {
        Some("bc_card".to_string())
    } else {
        None
    }
}

fn normalize_card_lookup_key(value: &str) -> String {
    value
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>()
        .to_lowercase()
}

/// 발급사 + 뒷 4자리로 깔끔한 카드 표시명 생성
/// 예: "536648******9959" + "삼성카드" → "삼성카드 (9959)"
/// 예: "본인205*" + "신한카드" → "신한카드 (205*)" (숫자 3자리+*)
pub fn make_card_display_name(issuer: &str, raw_card_name: &str) -> String {
    // 뒷 숫자 4자리 추출 시도
    if let Some(last4) = extract_card_last4(raw_card_name) {
        return format!("{} ({})", issuer, last4);
    }
    // 숫자*패턴 추출 시도 (예: 205*)
    let trimmed = raw_card_name.trim();
    let digits_star: String = trimmed
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit() || *c == '*')
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    if !digits_star.is_empty() && digits_star.len() <= 6 {
        return format!("{} ({})", issuer, digits_star);
    }
    issuer.to_string()
}

fn extract_card_last4(value: &str) -> Option<String> {
    let digits = value
        .chars()
        .filter(|ch| ch.is_ascii_digit())
        .collect::<String>();
    if digits.len() < 4 {
        return None;
    }
    Some(digits[digits.len() - 4..].to_string())
}

fn infer_card_issuer(source_institution: Option<&str>, card_name: &str) -> String {
    let source = source_institution.unwrap_or("").to_lowercase();
    if source.contains("hyundai") {
        return "현대카드".to_string();
    }
    if source.contains("samsung") {
        return "삼성카드".to_string();
    }
    if source.contains("shinhan") {
        return "신한카드".to_string();
    }
    if source.contains("kb") {
        return "KB국민카드".to_string();
    }
    if source.contains("bc") {
        return "BC카드".to_string();
    }

    let name = card_name.to_lowercase();
    if name.contains("현대") {
        return "현대카드".to_string();
    }
    if name.contains("삼성") {
        return "삼성카드".to_string();
    }
    if name.contains("신한") {
        return "신한카드".to_string();
    }
    if name.contains("국민") || name.contains("kb") {
        return "KB국민카드".to_string();
    }
    if name.contains("bc") {
        return "BC카드".to_string();
    }

    "미분류".to_string()
}

pub fn normalize_row_dedupe_key(user_id: Uuid, row: &NormalizedTransaction) -> String {
    dedupe::build_dedupe_key(
        user_id,
        row.source_institution.as_deref(),
        row.transaction_at,
        row.amount,
        row.merchant_name.as_deref(),
        row.description.as_deref(),
        None,
        None,
        row.approval_number.as_deref(),
    )
}

pub async fn list_imports(pool: &PgPool, user_id: Uuid) -> Result<Vec<ImportRecord>> {
    let rows = sqlx::query_as::<_, ImportRecord>(
        "SELECT * FROM imports WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_import(pool: &PgPool, user_id: Uuid, import_id: Uuid) -> Result<(ImportRecord, Vec<ImportRow>)> {
    let import = sqlx::query_as::<_, ImportRecord>("SELECT * FROM imports WHERE id = $1 AND user_id = $2")
        .bind(import_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| anyhow!("가져오기 정보를 찾을 수 없습니다"))?;

    let rows = sqlx::query_as::<_, ImportRow>(
        "SELECT * FROM import_rows WHERE import_id = $1 AND user_id = $2 ORDER BY row_index ASC",
    )
    .bind(import_id)
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok((import, rows))
}

pub async fn cancel_import(pool: &PgPool, user_id: Uuid, import_id: Uuid) -> Result<()> {
    let result = sqlx::query(
        "UPDATE imports SET status = 'failed', error_message = $3, updated_at = now() WHERE id = $1 AND user_id = $2 AND status IN ('pending', 'parsed')",
    )
    .bind(import_id)
    .bind(user_id)
    .bind("사용자가 가져오기를 취소했습니다")
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(anyhow!("취소할 수 없는 상태입니다. 이미 저장 완료되었거나 존재하지 않는 항목입니다."));
    }

    Ok(())
}
