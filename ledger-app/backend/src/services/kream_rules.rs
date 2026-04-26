use sqlx::PgPool;
use uuid::Uuid;

pub fn normalize_keyword(value: &str) -> String {
    value
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>()
        .to_lowercase()
}

pub fn normalize_haystack(values: &[Option<&str>]) -> String {
    values
        .iter()
        .filter_map(|value| *value)
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>()
        .to_lowercase()
}

pub async fn infer_kream_kind(
    pool: &PgPool,
    user_id: Uuid,
    merchant_name: Option<&str>,
    description: Option<&str>,
    memo: Option<&str>,
) -> Result<Option<String>, sqlx::Error> {
    let haystack = normalize_haystack(&[merchant_name, description, memo]);
    if haystack.is_empty() {
        return Ok(None);
    }

    let rules = sqlx::query_as::<_, (String, String)>(
        r#"
        SELECT keyword_normalized, kream_kind
        FROM kream_keyword_rules
        WHERE user_id = $1
          AND is_active = true
        ORDER BY created_at ASC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rules
        .into_iter()
        .find_map(|(keyword, kind)| haystack.contains(&keyword).then_some(kind)))
}

pub fn sql_keyword_pattern(keyword: &str) -> String {
    format!("%{}%", normalize_keyword(keyword))
}
