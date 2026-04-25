use anyhow::Result;
use chrono::{Datelike, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{import, import::dedupe::build_dedupe_key, telegram::parser};

pub async fn handle_text_command(pool: &PgPool, user_id: Uuid, text: &str) -> Result<String> {
    let parsed = match parser::parse_command(text) {
        Some(v) => v,
        None => {
            return Ok("명령어를 입력해 주세요. 예: /today, /month, /add".to_string());
        }
    };

    match parsed.command.as_str() {
        "/today" => today_summary(pool, user_id).await,
        "/month" => month_summary(pool, user_id).await,
        "/add" => add_expense(pool, user_id, &parsed.args).await,
        "/cards" => cards_summary(pool, user_id).await,
        "/import" => list_pending_imports(pool, user_id).await,
        "/ok" => confirm_pending_import(pool, user_id, parsed.args.first()).await,
        _ => Ok("지원하지 않는 명령입니다. 사용 가능: /today /month /add /cards /import /ok".to_string()),
    }
}

async fn today_summary(pool: &PgPool, user_id: Uuid) -> Result<String> {
    let now_kst = Utc::now().with_timezone(&kst()?);
    let date = now_kst.date_naive();
    let start = kst_date_time(date, NaiveTime::from_hms_opt(0, 0, 0).unwrap())?;
    let end = kst_date_time(date, NaiveTime::from_hms_opt(23, 59, 59).unwrap())?;

    let total_expense = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT SUM(amount)::bigint FROM transactions WHERE user_id = $1 AND type = 'expense' AND amount > 0 AND transaction_at >= $2 AND transaction_at <= $3",
    )
    .bind(user_id)
    .bind(start)
    .bind(end)
    .fetch_one(pool)
    .await?
    .unwrap_or(0);

    let total_income = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT SUM(amount)::bigint FROM transactions WHERE user_id = $1 AND type = 'income' AND amount > 0 AND transaction_at >= $2 AND transaction_at <= $3",
    )
    .bind(user_id)
    .bind(start)
    .bind(end)
    .fetch_one(pool)
    .await?
    .unwrap_or(0);

    let rows = sqlx::query_as::<_, (Option<String>, Option<String>, i64)>(
        r#"
        SELECT merchant_name, description, amount
        FROM transactions
        WHERE user_id = $1
          AND type = 'expense'
          AND amount > 0
          AND transaction_at >= $2
          AND transaction_at <= $3
        ORDER BY transaction_at DESC
        LIMIT 5
        "#,
    )
    .bind(user_id)
    .bind(start)
    .bind(end)
    .fetch_all(pool)
    .await?;

    let mut reply = format!(
        "오늘 요약 ({})\n지출: {}\n수입: {}\n",
        date.format("%Y-%m-%d"),
        won(total_expense),
        won(total_income)
    );

    if rows.is_empty() {
        reply.push_str("\n오늘 등록된 지출 내역이 없습니다.");
    } else {
        reply.push_str("\n최근 지출\n");
        for (merchant, desc, amount) in rows {
            let name = merchant.or(desc).unwrap_or_else(|| "내역 없음".to_string());
            reply.push_str(&format!("- {} {}\n", name, won(amount)));
        }
    }

    Ok(reply)
}

async fn month_summary(pool: &PgPool, user_id: Uuid) -> Result<String> {
    let now_kst = Utc::now().with_timezone(&kst()?);
    let month_start_date = NaiveDate::from_ymd_opt(now_kst.year(), now_kst.month(), 1).unwrap();
    let next_month_date = if now_kst.month() == 12 {
        NaiveDate::from_ymd_opt(now_kst.year() + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(now_kst.year(), now_kst.month() + 1, 1).unwrap()
    };

    let start = kst_date_time(month_start_date, NaiveTime::from_hms_opt(0, 0, 0).unwrap())?;
    let end = kst_date_time(next_month_date, NaiveTime::from_hms_opt(0, 0, 0).unwrap())?;

    let total_expense = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT SUM(amount)::bigint FROM transactions WHERE user_id = $1 AND type = 'expense' AND amount > 0 AND transaction_at >= $2 AND transaction_at < $3",
    )
    .bind(user_id)
    .bind(start)
    .bind(end)
    .fetch_one(pool)
    .await?
    .unwrap_or(0);

    let total_income = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT SUM(amount)::bigint FROM transactions WHERE user_id = $1 AND type = 'income' AND amount > 0 AND transaction_at >= $2 AND transaction_at < $3",
    )
    .bind(user_id)
    .bind(start)
    .bind(end)
    .fetch_one(pool)
    .await?
    .unwrap_or(0);

    Ok(format!(
        "이번 달 요약 ({})\n지출: {}\n수입: {}\n순지출: {}",
        month_start_date.format("%Y-%m"),
        won(total_expense),
        won(total_income),
        won(total_expense - total_income)
    ))
}

async fn add_expense(pool: &PgPool, user_id: Uuid, args: &[String]) -> Result<String> {
    if args.len() < 2 {
        return Ok("사용법: /add 금액 가맹점 [메모]".to_string());
    }

    let amount = match parser::parse_amount(&args[0]) {
        Some(v) => v,
        None => return Ok("금액 형식이 올바르지 않습니다. 예: /add 12000 점심".to_string()),
    };

    let merchant_name = args[1].trim().to_string();
    if merchant_name.is_empty() {
        return Ok("가맹점명을 입력해 주세요.".to_string());
    }

    let memo = if args.len() > 2 {
        Some(args[2..].join(" "))
    } else {
        None
    };

    let transaction_at = Utc::now();
    let dedupe_key = build_dedupe_key(
        user_id,
        Some("telegram"),
        transaction_at,
        amount,
        Some(&merchant_name),
        memo.as_deref(),
        None,
        None,
        None,
    );

    let insert = sqlx::query(
        r#"
        INSERT INTO transactions (
            id, user_id, transaction_at, posted_at, type, amount,
            merchant_name, description, category_id, account_id, card_id,
            source_type, source_institution, source_file_id, balance_after,
            raw_data, dedupe_key, memo
        ) VALUES (
            $1, $2, $3, NULL, 'expense', $4,
            $5, NULL, NULL, NULL, NULL,
            'telegram', 'telegram', NULL, NULL,
            '{}'::jsonb, $6, $7
        )
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(transaction_at)
    .bind(amount)
    .bind(merchant_name.clone())
    .bind(dedupe_key)
    .bind(memo.clone())
    .execute(pool)
    .await;

    match insert {
        Ok(_) => Ok(format!(
            "거래를 추가했습니다.\n가맹점: {}\n금액: {}",
            merchant_name,
            won(amount)
        )),
        Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
            Ok("중복 거래로 판단되어 저장하지 않았습니다.".to_string())
        }
        Err(err) => Err(err.into()),
    }
}

async fn cards_summary(pool: &PgPool, user_id: Uuid) -> Result<String> {
    let now_kst = Utc::now().with_timezone(&kst()?);
    let month_start_date = NaiveDate::from_ymd_opt(now_kst.year(), now_kst.month(), 1).unwrap();
    let next_month_date = if now_kst.month() == 12 {
        NaiveDate::from_ymd_opt(now_kst.year() + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(now_kst.year(), now_kst.month() + 1, 1).unwrap()
    };

    let start = kst_date_time(month_start_date, NaiveTime::from_hms_opt(0, 0, 0).unwrap())?;
    let end = kst_date_time(next_month_date, NaiveTime::from_hms_opt(0, 0, 0).unwrap())?;

    let rows = sqlx::query_as::<_, (String, Option<i64>)>(
        r#"
        SELECT COALESCE(c.card_name, '미지정 카드') AS name, SUM(t.amount)::bigint
        FROM transactions t
        LEFT JOIN cards c ON t.card_id = c.id
        WHERE t.user_id = $1
          AND t.type = 'expense'
          AND t.amount > 0
          AND t.transaction_at >= $2
          AND t.transaction_at < $3
        GROUP BY 1
        ORDER BY SUM(t.amount) DESC
        LIMIT 5
        "#,
    )
    .bind(user_id)
    .bind(start)
    .bind(end)
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Ok("이번 달 카드 지출 내역이 없습니다.".to_string());
    }

    let mut reply = format!("이번 달 카드 지출 ({})\n", month_start_date.format("%Y-%m"));
    for (name, amount) in rows {
        reply.push_str(&format!("- {} {}\n", name, won(amount.unwrap_or(0))));
    }
    Ok(reply)
}

async fn list_pending_imports(pool: &PgPool, user_id: Uuid) -> Result<String> {
    let rows = sqlx::query_as::<_, (Uuid, Option<String>, String, i32, i32, chrono::DateTime<Utc>)>(
        r#"
        SELECT id, original_filename, institution, parsed_count, duplicate_count, created_at
        FROM imports
        WHERE user_id = $1
          AND status = 'parsed'
        ORDER BY created_at DESC
        LIMIT 5
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Ok("확정 대기 중인 가져오기가 없습니다. 먼저 파일을 가져오고 미리보기를 생성해 주세요.".to_string());
    }

    let mut reply = String::from("확정 가능한 가져오기 목록\n");
    for (id, filename, institution, parsed_count, duplicate_count, created_at) in rows {
        let code = short_id(id);
        let title = filename.unwrap_or(institution);
        let new_count = parsed_count.saturating_sub(duplicate_count);
        let date_label = created_at.with_timezone(&kst()?).format("%m-%d %H:%M").to_string();

        reply.push_str(&format!(
            "- 코드 {} | {} | 신규 {}건 | 중복 {}건 | {}\n",
            code, title, new_count, duplicate_count, date_label
        ));
    }
    reply.push_str("\n저장하려면 /ok 코드 를 입력하세요. 코드 없이 /ok 를 입력하면 가장 최근 건을 저장합니다.");

    Ok(reply)
}

async fn confirm_pending_import(pool: &PgPool, user_id: Uuid, code: Option<&String>) -> Result<String> {
    let rows = sqlx::query_as::<_, (Uuid, Option<String>, String, i32, i32)>(
        r#"
        SELECT id, original_filename, institution, parsed_count, duplicate_count
        FROM imports
        WHERE user_id = $1
          AND status = 'parsed'
        ORDER BY created_at DESC
        LIMIT 20
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Ok("확정할 가져오기 항목이 없습니다. /import 로 목록을 확인해 주세요.".to_string());
    }

    let target = if let Some(code) = code {
        let normalized = code.trim().to_lowercase();
        let mut matched = rows
            .iter()
            .filter(|(id, _, _, _, _)| id.to_string().starts_with(&normalized))
            .cloned()
            .collect::<Vec<_>>();

        if matched.is_empty() {
            return Ok("해당 코드를 찾을 수 없습니다. /import 로 코드를 확인해 주세요.".to_string());
        }
        if matched.len() > 1 {
            return Ok("코드가 여러 항목과 일치합니다. 더 긴 코드로 다시 입력해 주세요.".to_string());
        }
        matched.remove(0)
    } else {
        rows[0].clone()
    };

    let (import_id, filename, institution, _parsed_count, _duplicate_count) = target;
    let summary = import::confirm_import(pool, user_id, import_id).await?;

    let title = filename.unwrap_or(institution);
    Ok(format!(
        "가져오기를 저장했습니다.\n대상: {}\n신규 저장: {}건\n중복: {}건\n오류: {}건",
        title, summary.new_count, summary.duplicate_count, summary.error_count
    ))
}

fn short_id(id: Uuid) -> String {
    id.to_string().chars().take(8).collect()
}

fn kst() -> Result<FixedOffset> {
    FixedOffset::east_opt(9 * 3600).ok_or_else(|| anyhow::anyhow!("시간대 계산 실패"))
}

fn kst_date_time(date: NaiveDate, time: NaiveTime) -> Result<chrono::DateTime<Utc>> {
    let kst = kst()?;
    let dt = NaiveDateTime::new(date, time)
        .and_local_timezone(kst)
        .single()
        .ok_or_else(|| anyhow::anyhow!("시간 변환 실패"))?;
    Ok(dt.with_timezone(&Utc))
}

fn won(value: i64) -> String {
    let mut num = value.abs().to_string();
    let mut out = String::new();

    while num.len() > 3 {
        let split = num.split_off(num.len() - 3);
        if out.is_empty() {
            out = split;
        } else {
            out = format!("{},{}", split, out);
        }
    }

    if out.is_empty() {
        out = num;
    } else {
        out = format!("{},{}", num, out);
    }

    if value < 0 {
        format!("-{}원", out)
    } else {
        format!("{}원", out)
    }
}
