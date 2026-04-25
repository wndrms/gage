use sqlx::PgPool;
use uuid::Uuid;

pub fn normalize_merchant(value: &str) -> String {
    value
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_lowercase()
}

/// 가맹점명으로 카테고리 ID를 추론합니다.
/// 규칙 우선순위(priority DESC) → 키워드 길이(DESC) 순으로 최적 매치를 반환합니다.
pub async fn auto_categorize(
    pool: &PgPool,
    user_id: Uuid,
    merchant_name: Option<&str>,
) -> Option<Uuid> {
    let raw = merchant_name?.trim();
    if raw.is_empty() {
        return None;
    }
    let normalized = normalize_merchant(raw);

    // keyword_normalized가 가맹점명(normalized)에 포함되는 규칙 중 최우선 선택
    sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT category_id
        FROM merchant_category_rules
        WHERE user_id = $1
          AND position(keyword_normalized IN $2) > 0
        ORDER BY priority DESC, length(keyword_normalized) DESC
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .bind(&normalized)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
}

/// 사용자가 가맹점에 카테고리를 직접 지정했을 때 규칙을 학습합니다.
/// 기존 규칙이 있으면 category_id를 갱신하고 source를 'learned'로 승격합니다.
pub async fn learn_rule(
    pool: &PgPool,
    user_id: Uuid,
    merchant_name: &str,
    category_id: Uuid,
) {
    let keyword = merchant_name.trim();
    if keyword.is_empty() {
        return;
    }
    let normalized = normalize_merchant(keyword);

    let _ = sqlx::query(
        r#"
        INSERT INTO merchant_category_rules
            (user_id, keyword, keyword_normalized, category_id, priority, source)
        VALUES ($1, $2, $3, $4, 150, 'learned')
        ON CONFLICT (user_id, keyword_normalized)
        DO UPDATE SET
            category_id = EXCLUDED.category_id,
            source = CASE
                WHEN merchant_category_rules.source = 'seed' THEN 'learned'
                ELSE merchant_category_rules.source
            END,
            updated_at = now()
        "#,
    )
    .bind(user_id)
    .bind(keyword)
    .bind(normalized)
    .bind(category_id)
    .execute(pool)
    .await;
}
