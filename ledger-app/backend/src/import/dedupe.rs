use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub fn build_dedupe_key(
    user_id: Uuid,
    source_institution: Option<&str>,
    transaction_at: DateTime<Utc>,
    amount: i64,
    merchant_name: Option<&str>,
    description: Option<&str>,
    card_id: Option<Uuid>,
    account_id: Option<Uuid>,
    approval_number: Option<&str>,
) -> String {
    let mut normalized = vec![
        user_id.to_string(),
        source_institution.unwrap_or("unknown").trim().to_lowercase(),
        transaction_at.format("%Y-%m-%dT%H:%M:%S").to_string(),
        amount.to_string(),
        merchant_name.unwrap_or("").trim().to_lowercase(),
        description.unwrap_or("").trim().to_lowercase(),
        card_id.map(|v| v.to_string()).unwrap_or_default(),
        account_id.map(|v| v.to_string()).unwrap_or_default(),
        approval_number.unwrap_or("").trim().to_lowercase(),
    ];

    if normalized[8].is_empty() {
        normalized[8] = "none".to_string();
    }

    let joined = normalized.join("|");
    let mut hasher = Sha256::new();
    hasher.update(joined.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, Utc};

    use super::*;

    #[test]
    fn dedupe_key_should_be_stable() {
        let user_id = Uuid::new_v4();
        let dt = NaiveDate::from_ymd_opt(2026, 4, 20)
            .unwrap()
            .and_hms_opt(10, 0, 0)
            .unwrap()
            .and_utc();

        let key1 = build_dedupe_key(
            user_id,
            Some("shinhan_card"),
            dt,
            10000,
            Some("스타벅스"),
            Some("커피"),
            None,
            None,
            Some("1234"),
        );

        let key2 = build_dedupe_key(
            user_id,
            Some("shinhan_card"),
            dt,
            10000,
            Some("스타벅스"),
            Some("커피"),
            None,
            None,
            Some("1234"),
        );

        assert_eq!(key1, key2);
    }

    #[test]
    fn dedupe_key_changes_on_amount() {
        let user_id = Uuid::new_v4();
        let dt = Utc::now();
        let a = build_dedupe_key(user_id, None, dt, 1000, None, None, None, None, None);
        let b = build_dedupe_key(user_id, None, dt, 2000, None, None, None, None, None);
        assert_ne!(a, b);
    }
}
