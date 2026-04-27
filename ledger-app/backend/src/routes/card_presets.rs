use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
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
    pub parse_text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ParsePresetRequest {
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct ParsedPresetResponse {
    pub monthly_requirement: Option<i64>,
    pub rules: serde_json::Value,
    pub benefits: serde_json::Value,
    pub benefit_groups: Vec<ParsedBenefitGroup>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ParsedBenefitGroup {
    pub group_name: String,
    pub discount_rate: f64,
    pub monthly_cap: Option<i64>,
    pub monthly_usage_limit: Option<i64>,
    pub merchants: Vec<String>,
    pub benefit_name: String,
}

#[derive(Debug, Serialize)]
pub struct ApplyBenefitsResponse {
    pub applied_count: u64,
    pub skipped_count: u64,
    pub preset_id: Uuid,
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
        INSERT INTO card_presets (id, issuer, card_name, aliases, monthly_requirement, rules, benefits, parse_text)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(payload.issuer)
    .bind(payload.card_name)
    .bind(payload.aliases.unwrap_or_default())
    .bind(payload.monthly_requirement)
    .bind(payload.rules.unwrap_or_else(|| serde_json::json!({})))
    .bind(payload.benefits.unwrap_or_else(|| serde_json::json!([])))
    .bind(payload.parse_text)
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
            parse_text = COALESCE($8, parse_text),
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
    .bind(payload.parse_text)
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

/// 혜택 텍스트를 파싱하여 구조화된 프리셋 JSON을 반환합니다.
pub async fn parse_preset_text(
    _auth: AuthUser,
    Json(payload): Json<ParsePresetRequest>,
) -> Result<Json<ParsedPresetResponse>, AppError> {
    let groups = parse_benefit_text(&payload.text);
    let (rules, benefits) = build_preset_json(&groups);

    let monthly_requirement = extract_monthly_requirement(&payload.text);

    Ok(Json(ParsedPresetResponse {
        monthly_requirement,
        rules,
        benefits,
        benefit_groups: groups,
    }))
}

/// 특정 프리셋을 카드에 연결된 기존 거래에 소급 적용합니다.
pub async fn apply_preset_to_transactions(
    Path(preset_id): Path<Uuid>,
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<ApplyBenefitsResponse>, AppError> {
    // 프리셋 조회
    let preset = sqlx::query_as::<_, CardPreset>(
        "SELECT * FROM card_presets WHERE id = $1",
    )
    .bind(preset_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    // 이 프리셋을 사용하는 카드 목록
    let card_ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM cards WHERE preset_id = $1 AND user_id = $2",
    )
    .bind(preset_id)
    .bind(auth.id)
    .fetch_all(&state.pool)
    .await?;

    if card_ids.is_empty() {
        return Ok(Json(ApplyBenefitsResponse {
            applied_count: 0,
            skipped_count: 0,
            preset_id,
        }));
    }

    let benefits_arr = match preset.benefits.as_array() {
        Some(arr) => arr.clone(),
        None => {
            return Ok(Json(ApplyBenefitsResponse {
                applied_count: 0,
                skipped_count: 0,
                preset_id,
            }));
        }
    };

    let mut applied_count: u64 = 0;
    let mut skipped_count: u64 = 0;

    for card_id in &card_ids {
        // 해당 카드의 모든 expense 거래를 가져옴
        let transactions = sqlx::query_as::<_, (Uuid, i64, Option<String>)>(
            r#"
            SELECT t.id, t.amount, t.merchant_name
            FROM transactions t
            WHERE t.user_id = $1
              AND t.card_id = $2
              AND t.type = 'expense'
              AND t.scope = 'personal'
              AND t.amount > 0
            ORDER BY t.transaction_at ASC
            "#,
        )
        .bind(auth.id)
        .bind(card_id)
        .fetch_all(&state.pool)
        .await?;

        for (tx_id, amount, merchant_name) in transactions {
            for benefit in &benefits_arr {
                let benefit_name = benefit
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if benefit_name.is_empty() {
                    continue;
                }

                // 이미 적용된 건은 스킵
                let already_applied: i64 = sqlx::query_scalar(
                    "SELECT COUNT(1) FROM card_benefit_applications WHERE transaction_id = $1 AND benefit_name = $2",
                )
                .bind(tx_id)
                .bind(&benefit_name)
                .fetch_one(&state.pool)
                .await?;

                if already_applied > 0 {
                    skipped_count += 1;
                    continue;
                }

                if !transaction_matches_benefit(amount, merchant_name.as_deref(), benefit) {
                    continue;
                }

                let discount_amount = calc_discount(amount, benefit);

                sqlx::query(
                    r#"
                    INSERT INTO card_benefit_applications
                        (id, user_id, transaction_id, preset_id, benefit_name, discount_amount)
                    VALUES ($1, $2, $3, $4, $5, $6)
                    ON CONFLICT (transaction_id, benefit_name) DO NOTHING
                    "#,
                )
                .bind(Uuid::new_v4())
                .bind(auth.id)
                .bind(tx_id)
                .bind(preset_id)
                .bind(&benefit_name)
                .bind(discount_amount)
                .execute(&state.pool)
                .await?;

                applied_count += 1;
            }
        }
    }

    Ok(Json(ApplyBenefitsResponse {
        applied_count,
        skipped_count,
        preset_id,
    }))
}

fn transaction_matches_benefit(
    amount: i64,
    merchant_name: Option<&str>,
    benefit: &serde_json::Value,
) -> bool {
    // 최소 금액 조건 (1회 1만원 또는 2만원 이상)
    let min_amount = benefit
        .get("min_amount")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    if amount < min_amount {
        return false;
    }

    let matcher = benefit.get("match");
    let Some(matcher) = matcher else {
        // match 조건 없으면 모든 거래에 적용
        return true;
    };

    if let Some(keywords) = matcher.get("merchant_keywords").and_then(|v| v.as_array()) {
        let merchant = merchant_name.unwrap_or("");
        let hit = keywords
            .iter()
            .filter_map(|v| v.as_str())
            .any(|kw| merchant.contains(kw));
        if !hit {
            return false;
        }
    }

    true
}

fn calc_discount(amount: i64, benefit: &serde_json::Value) -> i64 {
    let discount = benefit.get("discount");
    let Some(discount) = discount else { return 0 };

    let discount_type = discount.get("type").and_then(|v| v.as_str()).unwrap_or("");
    let value = discount.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
    let per_tx_cap = discount
        .get("per_tx_cap")
        .and_then(|v| v.as_i64())
        .unwrap_or(i64::MAX);

    let raw = match discount_type {
        "percent" => {
            let max_base = discount
                .get("max_base")
                .and_then(|v| v.as_i64())
                .unwrap_or(i64::MAX);
            let base = amount.min(max_base);
            base * value / 100
        }
        "fixed" => value,
        _ => 0,
    };

    raw.min(per_tx_cap)
}

// ─── 텍스트 파서 ──────────────────────────────────────────────────────────────

fn extract_monthly_requirement(text: &str) -> Option<i64> {
    // "전월실적 30만원" 패턴에서 첫 번째 숫자를 추출
    for line in text.lines() {
        if line.contains("전월실적") || line.contains("전월 실적") {
            // "30만원", "300,000원" 형태 파싱
            if let Some(val) = extract_amount_from_line(line) {
                return Some(val);
            }
        }
    }
    None
}

fn extract_amount_from_line(line: &str) -> Option<i64> {
    let line = line.replace(',', "");
    // "N만원" 형태
    let re_man = regex_find_man(&line);
    if let Some(v) = re_man {
        return Some(v);
    }
    // 순수 숫자 + "원"
    let digits: String = line.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    digits.parse().ok()
}

fn regex_find_man(text: &str) -> Option<i64> {
    // 간단한 "숫자만원" 파싱 (정규식 없이)
    let text_lower = text;
    if let Some(pos) = text_lower.find("만원") {
        // 앞에서 숫자 추출
        let before = &text_lower[..pos];
        let digits: String = before
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        if let Ok(n) = digits.parse::<i64>() {
            return Some(n * 10_000);
        }
    }
    None
}

/// 혜택 텍스트 전체를 파싱하여 BenefitGroup 목록 반환
fn parse_benefit_text(text: &str) -> Vec<ParsedBenefitGroup> {
    let mut groups: Vec<ParsedBenefitGroup> = Vec::new();

    // 섹션 분리: 빈 줄로 구분된 블록들을 파싱
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();

        // 퍼센트 할인 문구가 있는 줄: 헤더로 인식
        if let Some(rate) = extract_discount_rate(line) {
            let group_name = extract_group_name(line);
            let merchants = collect_merchants_after(&lines, i + 1);
            let monthly_cap = find_integrated_monthly_cap(&lines, i);
            let monthly_usage_limit = find_monthly_usage_limit(&lines, i);
            let rate_str = if rate.fract() == 0.0 {
                format!("{}", rate as i64)
            } else {
                format!("{}", rate)
            };
            let benefit_name = format!("{} {}% 할인", group_name, rate_str);

            if !group_name.is_empty() {
                groups.push(ParsedBenefitGroup {
                    group_name: group_name.clone(),
                    discount_rate: rate,
                    monthly_cap,
                    monthly_usage_limit,
                    merchants,
                    benefit_name,
                });
            }
        }

        i += 1;
    }

    // 세부 그룹을 병합: 동일 섹션 내 sub-groups (편의점, 생활/잡화, 커피 등)
    merge_sub_groups(text, &mut groups);

    groups
}

fn extract_discount_rate(line: &str) -> Option<f64> {
    // "10% 할인", "1.5% 결제일할인", "1%/1.5% 할인", "청구 할인" 패턴
    if !line.contains('%') {
        return None;
    }
    if !line.contains("할인") && !line.contains("캐시백") && !line.contains("적립") {
        return None;
    }

    // "N%/M%" 형태 → 최대값 취하기
    let mut rates: Vec<f64> = Vec::new();
    let mut search = line;
    while let Some(pos) = search.find('%') {
        let before = &search[..pos];
        // 소수점 포함 숫자 추출 (역방향으로)
        let raw: String = before
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        if let Ok(v) = raw.parse::<f64>() {
            if v > 0.0 && v <= 100.0 {
                rates.push(v);
            }
        }
        search = &search[pos + 1..];
    }

    // 최대 할인율 반환 (1%/1.5% → 1.5)
    rates.into_iter().reduce(f64::max)
}

fn extract_group_name(line: &str) -> String {
    // "4대 편의점, 생활/잡화, 커피, 월납 20% 할인 (전월실적 30만원 이상)" 에서
    // 퍼센트 이전 부분을 그룹명으로 사용
    if let Some(pos) = line.find('%') {
        let before = &line[..pos];
        // 마지막 숫자, 소수점, 슬래시, 공백 제거
        let trimmed = before
            .trim_end_matches(|c: char| c.is_ascii_digit() || c == '.' || c == '/' || c == ' ')
            .trim();
        // 괄호 제거
        let trimmed = trimmed.trim_end_matches('(').trim();
        // "이용 시" 같은 동사 구문도 제거
        let trimmed = trimmed
            .trim_end_matches("이용 시")
            .trim_end_matches("에서 결제 시")
            .trim_end_matches("에서")
            .trim_end_matches(" 및")
            .trim();
        trimmed.trim().to_string()
    } else {
        line.trim().to_string()
    }
}

fn collect_merchants_after(lines: &[&str], start: usize) -> Vec<String> {
    let mut merchants = Vec::new();

    for i in start..lines.len().min(start + 3) {
        let line = lines[i].trim();
        if line.is_empty() {
            break;
        }
        // "- 가맹점1, 가맹점2, 가맹점3" 패턴
        let content = line.trim_start_matches('-').trim();
        if content.is_empty() {
            continue;
        }
        // 괄호 이후 내용 제거 (조건 설명)
        let content = if let Some(pos) = content.find('(') {
            &content[..pos]
        } else {
            content
        };
        // 쉼표로 분리
        for part in content.split(',') {
            let m = part.trim().to_string();
            if !m.is_empty() && m.len() <= 20 {
                merchants.push(m);
            }
        }
        break;
    }
    merchants
}

fn find_integrated_monthly_cap(lines: &[&str], from: usize) -> Option<i64> {
    let search_range = &lines[from..lines.len().min(from + 60)];
    let mut caps: Vec<i64> = Vec::new();

    let mut in_cap_section = false;
    for line in search_range {
        let line = line.trim();

        // 섹션 진입 키워드
        if line.contains("통합할인한도")
            || line.contains("통합 할인 한도")
            || line.contains("할인한도")
            || line.contains("할인 한도")
            || line.contains("월 할인한도")
        {
            in_cap_section = true;
            // "할인한도 : 통합 월 5,000원" 같이 같은 줄에 값이 있을 수도 있음
            if let Some(val) = extract_amount_any(line) {
                caps.push(val);
            }
            continue;
        }

        if in_cap_section {
            if line.is_empty() {
                continue;
            }

            // "전월 실적 N만 원 이상: M만 원" 형태 (콜론 기준 오른쪽)
            if line.contains(':') || line.contains('：') {
                let rhs = line
                    .split_once(':')
                    .or_else(|| line.split_once('：'))
                    .map(|(_, r)| r)
                    .unwrap_or(line);
                if let Some(val) = extract_amount_any(rhs) {
                    caps.push(val);
                }
            } else if line.starts_with('-') || line.starts_with('•') || line.starts_with('*') {
                // "- 통합 월 5,000원" 같은 패턴
                if let Some(val) = extract_amount_any(line) {
                    caps.push(val);
                }
            } else if line.contains("원") {
                // 삼성카드 표 형식: 금액만 있는 줄 ("10,000원", "5,000원")
                if let Some(val) = extract_amount_any(line) {
                    caps.push(val);
                }
            } else {
                // 새 섹션 시작
                break;
            }
        }
    }
    caps.into_iter().max()
}

/// 텍스트에서 금액 추출 (만원, 천원, 숫자+원 모두 처리)
fn extract_amount_any(text: &str) -> Option<i64> {
    // 쉼표 제거 후 파싱
    let text = text.replace(',', "").replace(' ', "");
    // 만+천 조합: "1만5천원"
    if text.contains("만") && text.contains("천") {
        if let Some(v) = find_chon(&text) {
            return Some(v);
        }
    }
    // 만원
    if let Some(v) = regex_find_man(&text) {
        return Some(v);
    }
    // 순수 숫자원: "5000원", "10000원"
    if let Some(pos) = text.find("원") {
        let before = &text[..pos];
        let digits: String = before
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        if let Ok(v) = digits.parse::<i64>() {
            if v > 0 {
                return Some(v);
            }
        }
    }
    None
}

fn find_monthly_usage_limit(lines: &[&str], from: usize) -> Option<i64> {
    let search_range = &lines[from..lines.len().min(from + 30)];
    for line in search_range {
        let line = line.trim();
        if (line.contains("월") && line.contains("회")) || line.contains("이용조건") {
            // "월 6회", "각 영역별 월 4회" 패턴
            if let Some(pos) = line.find("회") {
                let before = &line[..pos];
                let digits: String = before
                    .chars()
                    .rev()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .chars()
                    .rev()
                    .collect();
                if let Ok(n) = digits.parse::<i64>() {
                    return Some(n);
                }
            }
        }
    }
    None
}

fn find_chon(text: &str) -> Option<i64> {
    // "1만 5천 원" → 15000
    let text = text.replace(',', "").replace(' ', "");
    let mut result: i64 = 0;
    if let Some(pos) = text.find("만원").or_else(|| text.find("만")) {
        let before = &text[..pos];
        let digits: String = before
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        if let Ok(n) = digits.parse::<i64>() {
            result += n * 10_000;
        }
    }
    if let Some(pos) = text.find("천원").or_else(|| text.find("천")) {
        let before = &text[..pos];
        // 이미 만 단위 이후의 숫자
        let digits: String = before
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        if let Ok(n) = digits.parse::<i64>() {
            result += n * 1_000;
        }
    }
    if result > 0 { Some(result) } else { None }
}

/// 세부 혜택 그룹 병합: 신한카드 텍스트에서 편의점/커피/생활 등 개별 그룹 추출
fn merge_sub_groups(text: &str, groups: &mut Vec<ParsedBenefitGroup>) {
    // 패턴: "가맹점 20% 결제일 할인\n\n- 가맹점A, 가맹점B"
    let lines: Vec<&str> = text.lines().collect();

    let sub_patterns = [
        ("편의점", "GS25,CU,세븐일레븐,이마트24"),
        ("생활/잡화", "올리브영,다이소"),
        ("커피", "스타벅스,투썸플레이스"),
        ("월납", "쿠팡 정기배송,위메프 정기배송,리디북스,프레딧"),
        ("간편결제", ""),
        ("항공", "제주항공,에어부산"),
    ];

    for &(category, default_merchants) in &sub_patterns {
        // 이미 추출된 그룹에 해당 카테고리가 있으면 merchants 보완
        if let Some(group) = groups.iter_mut().find(|g| g.group_name.contains(category)) {
            if group.merchants.is_empty() && !default_merchants.is_empty() {
                group.merchants = default_merchants
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect();
            }
        } else {
            // 텍스트에서 해당 카테고리 섹션 찾기
            for (i, line) in lines.iter().enumerate() {
                let line = line.trim();
                if line.contains(category) && line.contains("할인") {
                    if let Some(rate) = extract_discount_rate(line) {
                        let merchants: Vec<String> = if default_merchants.is_empty() {
                            collect_merchants_after(&lines, i + 1)
                        } else {
                            default_merchants
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .collect()
                        };
                        let monthly_cap = find_integrated_monthly_cap(&lines, i);
                        let usage_limit = find_monthly_usage_limit(&lines, i);
                        let rate_str = if rate.fract() == 0.0 {
                            format!("{}", rate as i64)
                        } else {
                            format!("{}", rate)
                        };
                        groups.push(ParsedBenefitGroup {
                            group_name: category.to_string(),
                            discount_rate: rate,
                            monthly_cap,
                            monthly_usage_limit: usage_limit,
                            merchants,
                            benefit_name: format!("{} {}% 할인", category, rate_str),
                        });
                        break;
                    }
                }
            }
        }
    }

    // 중복 제거
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    groups.retain(|g| seen.insert(g.benefit_name.clone()));
}

fn build_preset_json(
    groups: &[ParsedBenefitGroup],
) -> (serde_json::Value, serde_json::Value) {
    let rules = serde_json::json!({
        "excluded": [
            { "merchant_contains": "상품권" },
            { "merchant_contains": "선불카드" },
            { "merchant_contains": "충전" }
        ]
    });

    let benefits: Vec<serde_json::Value> = groups
        .iter()
        .map(|g| {
            let keywords: Vec<serde_json::Value> = g
                .merchants
                .iter()
                .map(|m| serde_json::Value::String(m.clone()))
                .collect();

            let per_tx_cap = g.monthly_cap.map(|cap| {
                // 1회 한도 = 통합한도 / 월 사용 횟수 (없으면 통합한도의 절반으로 추정)
                let divisor = g.monthly_usage_limit.unwrap_or(4).max(1);
                cap / divisor
            });

            let mut discount = serde_json::json!({
                "type": "percent",
                "value": g.discount_rate,
            });
            if let Some(cap) = g.monthly_cap {
                discount["monthly_cap"] = serde_json::json!(cap);
            }
            if let Some(cap) = per_tx_cap {
                discount["per_tx_cap"] = serde_json::json!(cap);
            }

            let mut benefit = serde_json::json!({
                "name": g.benefit_name,
                "discount": discount,
            });

            if !keywords.is_empty() {
                benefit["match"] = serde_json::json!({
                    "merchant_keywords": keywords,
                });
                // 1회 최소 결제 금액 조건 (기본 1만원)
                benefit["min_amount"] = serde_json::json!(10000);
            }

            benefit
        })
        .collect();

    (rules, serde_json::json!(benefits))
}
