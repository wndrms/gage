use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleTransaction {
    pub amount: i64,
    pub merchant_name: Option<String>,
    pub category_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenefitRule {
    pub name: String,
    pub used_amount: i64,
    pub cap: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardBenefitSummary {
    pub monthly_spending: i64,
    pub eligible_spending: i64,
    pub monthly_requirement: i64,
    pub requirement_ratio: f64,
    pub benefits: Vec<BenefitRule>,
}

#[derive(Debug, Clone, Deserialize)]
struct ExcludedRule {
    merchant_contains: Option<String>,
    category: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct BenefitMatch {
    merchant_keywords: Option<Vec<String>>,
    category: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct DiscountRule {
    r#type: String,
    value: i64,
    monthly_cap: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
struct BenefitConfig {
    name: String,
    r#match: Option<BenefitMatch>,
    discount: DiscountRule,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct PresetConfig {
    monthly_requirement: Option<i64>,
    excluded: Option<Vec<ExcludedRule>>,
    benefits: Option<Vec<BenefitConfig>>,
}

pub fn empty_summary() -> CardBenefitSummary {
    CardBenefitSummary {
        monthly_spending: 0,
        eligible_spending: 0,
        monthly_requirement: 0,
        requirement_ratio: 0.0,
        benefits: Vec::new(),
    }
}

pub fn calculate_from_json(monthly_spending: i64, preset_json: &serde_json::Value) -> CardBenefitSummary {
    let rules = preset_json
        .get("rules")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let benefits = preset_json
        .get("benefits")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));

    let merged = serde_json::json!({
        "monthly_requirement": preset_json.get("monthly_requirement").cloned().unwrap_or(serde_json::Value::Null),
        "excluded": rules.get("excluded").cloned().unwrap_or(serde_json::json!([])),
        "benefits": if benefits.is_array() { benefits } else { benefits.get("benefits").cloned().unwrap_or(serde_json::json!([])) }
    });

    let tx = vec![RuleTransaction {
        amount: monthly_spending,
        merchant_name: None,
        category_name: None,
    }];

    calculate_summary(&tx, &merged)
}

pub fn calculate_summary(transactions: &[RuleTransaction], preset_json: &serde_json::Value) -> CardBenefitSummary {
    let config: PresetConfig = serde_json::from_value(preset_json.clone()).unwrap_or_default();

    let monthly_spending = transactions
        .iter()
        .map(|tx| tx.amount.max(0))
        .sum::<i64>();

    let excluded_rules = config.excluded.unwrap_or_default();
    let eligible_transactions = transactions
        .iter()
        .filter(|tx| !is_excluded(tx, &excluded_rules))
        .cloned()
        .collect::<Vec<_>>();

    let eligible_spending = eligible_transactions
        .iter()
        .map(|tx| tx.amount.max(0))
        .sum::<i64>();

    let monthly_requirement = config.monthly_requirement.unwrap_or(0);

    let requirement_ratio = if monthly_requirement <= 0 {
        100.0
    } else {
        (eligible_spending as f64 / monthly_requirement as f64 * 100.0).min(100.0)
    };

    let benefits = config
        .benefits
        .unwrap_or_default()
        .into_iter()
        .map(|benefit| {
            let cap = benefit.discount.monthly_cap.unwrap_or(i64::MAX);
            let used = calculate_benefit_used(&eligible_transactions, &benefit).min(cap);
            BenefitRule {
                name: benefit.name,
                used_amount: used,
                cap,
            }
        })
        .collect();

    CardBenefitSummary {
        monthly_spending,
        eligible_spending,
        monthly_requirement,
        requirement_ratio,
        benefits,
    }
}

fn is_excluded(tx: &RuleTransaction, excluded_rules: &[ExcludedRule]) -> bool {
    excluded_rules.iter().any(|rule| {
        let merchant_hit = match (&rule.merchant_contains, &tx.merchant_name) {
            (Some(keyword), Some(merchant)) => merchant.contains(keyword),
            _ => false,
        };
        let category_hit = match (&rule.category, &tx.category_name) {
            (Some(category), Some(tx_category)) => tx_category == category,
            _ => false,
        };
        merchant_hit || category_hit
    })
}

fn calculate_benefit_used(transactions: &[RuleTransaction], benefit: &BenefitConfig) -> i64 {
    transactions
        .iter()
        .filter(|tx| benefit_matches(tx, benefit.r#match.as_ref()))
        .map(|tx| match benefit.discount.r#type.as_str() {
            "percent" => tx.amount.max(0) * benefit.discount.value / 100,
            "fixed" => benefit.discount.value,
            _ => 0,
        })
        .sum()
}

fn benefit_matches(tx: &RuleTransaction, matcher: Option<&BenefitMatch>) -> bool {
    let Some(matcher) = matcher else {
        return true;
    };

    let merchant_ok = if let Some(keywords) = &matcher.merchant_keywords {
        let merchant = tx.merchant_name.as_deref().unwrap_or("");
        keywords.iter().any(|keyword| merchant.contains(keyword))
    } else {
        true
    };

    let category_ok = if let Some(category) = &matcher.category {
        tx.category_name.as_deref() == Some(category.as_str())
    } else {
        true
    };

    merchant_ok && category_ok
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculates_benefit_with_cap() {
        let tx = vec![
            RuleTransaction {
                amount: 10000,
                merchant_name: Some("스타벅스 강남점".to_string()),
                category_name: Some("카페".to_string()),
            },
            RuleTransaction {
                amount: 12000,
                merchant_name: Some("투썸플레이스".to_string()),
                category_name: Some("카페".to_string()),
            },
        ];

        let preset = serde_json::json!({
            "monthly_requirement": 300000,
            "excluded": [{"merchant_contains": "상품권"}],
            "benefits": [
                {
                    "name": "커피 할인",
                    "match": { "merchant_keywords": ["스타벅스", "투썸", "이디야"]},
                    "discount": {"type": "percent", "value": 10, "monthly_cap": 2000}
                }
            ]
        });

        let summary = calculate_summary(&tx, &preset);
        assert_eq!(summary.monthly_spending, 22000);
        assert_eq!(summary.eligible_spending, 22000);
        assert_eq!(summary.benefits.len(), 1);
        assert_eq!(summary.benefits[0].used_amount, 2000);
    }
}
