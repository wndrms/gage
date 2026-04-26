use axum::{
    Json,
    extract::{Query, State},
};
use chrono::{DateTime, Datelike, FixedOffset, NaiveDate, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{AppState, auth::extractor::AuthUser, errors::AppError, models::Transaction};

#[derive(Debug, Deserialize)]
pub struct MonthQuery {
    pub month: String,
}

#[derive(Debug, Deserialize)]
pub struct DateQuery {
    pub date: String,
}

#[derive(Debug, Serialize)]
pub struct BreakdownItem {
    pub name: String,
    pub amount: i64,
}

#[derive(Debug, Serialize)]
pub struct CategoryBreakdownItem {
    pub category_id: Option<uuid::Uuid>,
    pub name: String,
    pub amount: i64,
}

#[derive(Debug, Serialize)]
pub struct MonthlyComparison {
    pub previous_month: String,
    pub previous_total_income: i64,
    pub previous_total_expense: i64,
    pub previous_net_expense: i64,
    pub income_change_amount: i64,
    pub expense_change_amount: i64,
    pub net_expense_change_amount: i64,
    pub expense_change_rate: f64,
}

#[derive(Debug, Serialize)]
pub struct MonthlyDashboardResponse {
    pub month: String,
    pub total_income: i64,
    pub total_expense: i64,
    pub net_expense: i64,
    pub comparison: MonthlyComparison,
    pub category_expense: Vec<CategoryBreakdownItem>,
    pub card_expense: Vec<BreakdownItem>,
    pub account_expense: Vec<BreakdownItem>,
    pub recent_transactions: Vec<Transaction>,
}

#[derive(Debug, Serialize)]
pub struct DailyDashboardResponse {
    pub date: String,
    pub total_income: i64,
    pub total_expense: i64,
    pub transactions: Vec<Transaction>,
}

#[derive(Debug, Serialize)]
pub struct CalendarDayTotal {
    pub date: String,
    pub total_expense: i64,
}

pub async fn monthly_dashboard(
    Query(query): Query<MonthQuery>,
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<MonthlyDashboardResponse>, AppError> {
    let month_start_date = NaiveDate::parse_from_str(&format!("{}-01", query.month), "%Y-%m-%d")
        .map_err(|_| {
            AppError::BadRequest(
                "월 형식이 올바르지 않습니다. YYYY-MM 형식으로 입력해 주세요".to_string(),
            )
        })?;
    let month_start = month_start_date.and_hms_opt(0, 0, 0).unwrap().and_utc();
    let next_month_date = shift_month(month_start_date, 1);
    let next_month = next_month_date.and_hms_opt(0, 0, 0).unwrap().and_utc();

    let previous_month_date = shift_month(month_start_date, -1);
    let previous_month = previous_month_date.and_hms_opt(0, 0, 0).unwrap().and_utc();

    // 이번 달 / 전월 수입·지출 합계를 단일 쿼리로 집계합니다.
    let (total_income, total_expense, previous_total_income, previous_total_expense) =
        sqlx::query_as::<_, (Option<i64>, Option<i64>, Option<i64>, Option<i64>)>(
            r#"
            SELECT
                (SUM(amount) FILTER (WHERE type = 'income'  AND transaction_at >= $2 AND transaction_at < $3))::bigint,
                (SUM(amount) FILTER (WHERE type = 'expense' AND transaction_at >= $2 AND transaction_at < $3))::bigint,
                (SUM(amount) FILTER (WHERE type = 'income'  AND transaction_at >= $4 AND transaction_at < $2))::bigint,
                (SUM(amount) FILTER (WHERE type = 'expense' AND transaction_at >= $4 AND transaction_at < $2))::bigint
            FROM transactions
            WHERE user_id = $1
              AND scope = 'personal'
            "#,
        )
        .bind(auth.id)
        .bind(month_start)
        .bind(next_month)
        .bind(previous_month)
        .fetch_one(&state.pool)
        .await
        .map(|(a, b, c, d)| (a.unwrap_or(0), b.unwrap_or(0), c.unwrap_or(0), d.unwrap_or(0)))?;

    let recent_transactions = sqlx::query_as::<_, Transaction>(
        r#"
        SELECT *
        FROM transactions
        WHERE user_id = $1
          AND scope = 'personal'
          AND transaction_at >= $2
          AND transaction_at < $3
        ORDER BY transaction_at DESC
        LIMIT 15
        "#,
    )
    .bind(auth.id)
    .bind(month_start)
    .bind(next_month)
    .fetch_all(&state.pool)
    .await?;

    let category_expense = sqlx::query_as::<_, (Option<uuid::Uuid>, String, Option<i64>)>(
        r#"
        SELECT t.category_id, COALESCE(c.name, '미분류') AS name, SUM(t.amount)::bigint
        FROM transactions t
        LEFT JOIN categories c ON t.category_id = c.id
        WHERE t.user_id = $1
          AND t.scope = 'personal'
          AND t.transaction_at >= $2
          AND t.transaction_at < $3
          AND t.type = 'expense'
          AND t.amount > 0
        GROUP BY t.category_id, c.name
        ORDER BY SUM(t.amount) DESC
        LIMIT 10
        "#,
    )
    .bind(auth.id)
    .bind(month_start)
    .bind(next_month)
    .fetch_all(&state.pool)
    .await?
    .into_iter()
    .map(|(category_id, name, amount)| CategoryBreakdownItem {
        category_id,
        name,
        amount: amount.unwrap_or(0),
    })
    .collect::<Vec<_>>();

    let card_expense = sqlx::query_as::<_, (Option<String>, Option<String>, Option<i64>)>(
        r#"
        SELECT cd.issuer, cd.card_name, SUM(t.amount)::bigint
        FROM transactions t
        LEFT JOIN cards cd ON t.card_id = cd.id
        WHERE t.user_id = $1
          AND t.scope = 'personal'
          AND t.transaction_at >= $2
          AND t.transaction_at < $3
          AND t.type = 'expense'
          AND t.amount > 0
        GROUP BY cd.issuer, cd.card_name
        ORDER BY SUM(t.amount) DESC
        LIMIT 10
        "#,
    )
    .bind(auth.id)
    .bind(month_start)
    .bind(next_month)
    .fetch_all(&state.pool)
    .await?
    .into_iter()
    .map(|(issuer, card_name, amount)| {
        let name = format_card_display_name(issuer.as_deref(), card_name.as_deref());
        BreakdownItem {
            name,
            amount: amount.unwrap_or(0),
        }
    })
    .collect::<Vec<_>>();

    let account_expense = sqlx::query_as::<_, (String, Option<i64>)>(
        r#"
        SELECT COALESCE(a.name, '미지정 계좌') AS name, SUM(t.amount)::bigint
        FROM transactions t
        LEFT JOIN accounts a ON t.account_id = a.id
        WHERE t.user_id = $1
          AND t.scope = 'personal'
          AND t.transaction_at >= $2
          AND t.transaction_at < $3
          AND t.type = 'expense'
          AND t.amount > 0
        GROUP BY 1
        ORDER BY SUM(t.amount) DESC
        LIMIT 10
        "#,
    )
    .bind(auth.id)
    .bind(month_start)
    .bind(next_month)
    .fetch_all(&state.pool)
    .await?
    .into_iter()
    .map(|(name, amount)| BreakdownItem {
        name,
        amount: amount.unwrap_or(0),
    })
    .collect::<Vec<_>>();

    let net_expense = total_expense - total_income;
    let previous_net_expense = previous_total_expense - previous_total_income;
    let expense_change_amount = total_expense - previous_total_expense;
    let income_change_amount = total_income - previous_total_income;
    let net_expense_change_amount = net_expense - previous_net_expense;

    let expense_change_rate = if previous_total_expense <= 0 {
        if total_expense <= 0 { 0.0 } else { 100.0 }
    } else {
        expense_change_amount as f64 / previous_total_expense as f64 * 100.0
    };

    Ok(Json(MonthlyDashboardResponse {
        month: query.month,
        total_income,
        total_expense,
        net_expense,
        comparison: MonthlyComparison {
            previous_month: previous_month_date.format("%Y-%m").to_string(),
            previous_total_income,
            previous_total_expense,
            previous_net_expense,
            income_change_amount,
            expense_change_amount,
            net_expense_change_amount,
            expense_change_rate,
        },
        category_expense,
        card_expense,
        account_expense,
        recent_transactions,
    }))
}

pub async fn daily_dashboard(
    Query(query): Query<DateQuery>,
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<DailyDashboardResponse>, AppError> {
    let date = NaiveDate::parse_from_str(&query.date, "%Y-%m-%d").map_err(|_| {
        AppError::BadRequest(
            "날짜 형식이 올바르지 않습니다. YYYY-MM-DD 형식으로 입력해 주세요".to_string(),
        )
    })?;

    let start = kst_day_start_utc(date)?;
    let end = kst_day_end_utc(date)?;

    let total_income = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT SUM(amount)::bigint FROM transactions WHERE user_id = $1 AND scope = 'personal' AND transaction_at >= $2 AND transaction_at <= $3 AND type = 'income'",
    )
    .bind(auth.id)
    .bind(start)
    .bind(end)
    .fetch_one(&state.pool)
    .await?
    .unwrap_or(0);

    let total_expense = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT SUM(amount)::bigint FROM transactions WHERE user_id = $1 AND scope = 'personal' AND transaction_at >= $2 AND transaction_at <= $3 AND type = 'expense'",
    )
    .bind(auth.id)
    .bind(start)
    .bind(end)
    .fetch_one(&state.pool)
    .await?
    .unwrap_or(0);

    let transactions = sqlx::query_as::<_, Transaction>(
        "SELECT * FROM transactions WHERE user_id = $1 AND scope = 'personal' AND transaction_at >= $2 AND transaction_at <= $3 ORDER BY transaction_at DESC",
    )
    .bind(auth.id)
    .bind(start)
    .bind(end)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(DailyDashboardResponse {
        date: query.date,
        total_income,
        total_expense,
        transactions,
    }))
}

pub async fn calendar_dashboard(
    Query(query): Query<MonthQuery>,
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<CalendarDayTotal>>, AppError> {
    let month_start_date = NaiveDate::parse_from_str(&format!("{}-01", query.month), "%Y-%m-%d")
        .map_err(|_| {
            AppError::BadRequest(
                "월 형식이 올바르지 않습니다. YYYY-MM 형식으로 입력해 주세요".to_string(),
            )
        })?;
    let month_start = month_start_date.and_hms_opt(0, 0, 0).unwrap().and_utc();
    let next_month = shift_month(month_start_date, 1)
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();

    let rows = sqlx::query_as::<_, (NaiveDate, Option<i64>)>(
        r#"
        SELECT DATE(transaction_at AT TIME ZONE 'Asia/Seoul') AS day, SUM(amount)::bigint
        FROM transactions
        WHERE user_id = $1
          AND scope = 'personal'
          AND transaction_at >= $2
          AND transaction_at < $3
          AND type = 'expense'
        GROUP BY day
        ORDER BY day ASC
        "#,
    )
    .bind(auth.id)
    .bind(month_start)
    .bind(next_month)
    .fetch_all(&state.pool)
    .await?;

    let result = rows
        .into_iter()
        .map(|(day, total)| CalendarDayTotal {
            date: day.format("%Y-%m-%d").to_string(),
            total_expense: total.unwrap_or(0),
        })
        .collect();

    Ok(Json(result))
}

fn shift_month(base: NaiveDate, delta: i32) -> NaiveDate {
    let mut year = base.year();
    let mut month = base.month() as i32 + delta;

    while month <= 0 {
        month += 12;
        year -= 1;
    }
    while month > 12 {
        month -= 12;
        year += 1;
    }

    NaiveDate::from_ymd_opt(year, month as u32, 1).unwrap()
}

fn kst_day_start_utc(date: NaiveDate) -> Result<DateTime<Utc>, AppError> {
    let kst = FixedOffset::east_opt(9 * 3600)
        .ok_or_else(|| AppError::BadRequest("시간대 계산에 실패했습니다".to_string()))?;
    let local = NaiveDateTime::new(date, chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap());
    let dt = local
        .and_local_timezone(kst)
        .single()
        .ok_or_else(|| AppError::BadRequest("시작 시각 계산에 실패했습니다".to_string()))?;
    Ok(dt.with_timezone(&Utc))
}

fn kst_day_end_utc(date: NaiveDate) -> Result<DateTime<Utc>, AppError> {
    let kst = FixedOffset::east_opt(9 * 3600)
        .ok_or_else(|| AppError::BadRequest("시간대 계산에 실패했습니다".to_string()))?;
    let local = NaiveDateTime::new(date, chrono::NaiveTime::from_hms_opt(23, 59, 59).unwrap());
    let dt = local
        .and_local_timezone(kst)
        .single()
        .ok_or_else(|| AppError::BadRequest("종료 시각 계산에 실패했습니다".to_string()))?;
    Ok(dt.with_timezone(&Utc))
}

fn format_card_display_name(issuer: Option<&str>, card_name: Option<&str>) -> String {
    use crate::import::make_card_display_name;
    match (issuer, card_name) {
        (Some(iss), Some(name)) => make_card_display_name(iss, name),
        (Some(iss), None) => iss.to_string(),
        (None, Some(name)) => name.to_string(),
        (None, None) => "미지정 카드".to_string(),
    }
}
