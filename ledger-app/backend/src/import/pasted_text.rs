use anyhow::Result;

use super::parser::{
    DetectInput, NormalizedTransaction, TransactionParser, csv_records, ensure_column, header_map,
    parse_amount, parse_default_type, parse_local_datetime, pick_col, row_value,
};

#[derive(Debug, Default)]
pub struct PastedCsvTextParser;

impl TransactionParser for PastedCsvTextParser {
    fn name(&self) -> &'static str {
        "pasted_text"
    }

    fn detect(&self, input: &DetectInput<'_>) -> f32 {
        let sample = input
            .sample_text
            .map(|v| v.to_string())
            .unwrap_or_else(|| String::from_utf8_lossy(input.content).to_string());
        let mut score: f32 = 0.0;

        for marker in ["날짜", "일자", "시간", "가맹점", "금액", "출금", "입금"] {
            if sample.contains(marker) {
                score += 0.15;
            }
        }

        score.min(1.0)
    }

    fn parse(&self, content: &[u8]) -> Result<Vec<NormalizedTransaction>> {
        let (headers, rows) = csv_records(content)?;
        let map = header_map(&headers);

        let date_idx = ensure_column(pick_col(&map, &["날짜", "일자", "거래일자"]), "날짜/일자 열이 필요합니다")?;
        let time_idx = pick_col(&map, &["시간", "거래시간"]);
        let merchant_idx = pick_col(&map, &["가맹점", "가맹점명", "내용", "상호"]);
        let desc_idx = pick_col(&map, &["내용", "적요", "메모"]);
        let amount_idx = pick_col(&map, &["금액", "이용금액", "승인금액"]);
        let outflow_idx = pick_col(&map, &["출금", "지출"]);
        let inflow_idx = pick_col(&map, &["입금", "수입"]);
        let card_idx = pick_col(&map, &["카드", "카드명"]);
        let account_idx = pick_col(&map, &["계좌", "계좌명"]);

        let mut result = Vec::new();

        for row in rows {
            let date = row_value(&row, Some(date_idx));
            let transaction_at = match parse_local_datetime(date, Some(row_value(&row, time_idx))) {
                Some(v) => v,
                None => continue,
            };

            let amount = parse_amount(row_value(&row, amount_idx));
            let outflow = parse_amount(row_value(&row, outflow_idx));
            let inflow = parse_amount(row_value(&row, inflow_idx));

            let (typ, amount) = if let Some(v) = amount {
                if v >= 0 {
                    ("expense".to_string(), v)
                } else {
                    ("income".to_string(), -v)
                }
            } else {
                match parse_default_type(outflow, inflow) {
                    Some(v) => v,
                    None => continue,
                }
            };

            let merchant_name = row_value(&row, merchant_idx).trim().to_string();
            let description = row_value(&row, desc_idx).trim().to_string();
            let card_name = row_value(&row, card_idx).trim().to_string();
            let account_name = row_value(&row, account_idx).trim().to_string();

            result.push(NormalizedTransaction {
                transaction_at,
                posted_at: None,
                r#type: typ,
                amount,
                merchant_name: if merchant_name.is_empty() {
                    None
                } else {
                    Some(merchant_name)
                },
                description: if description.is_empty() {
                    None
                } else {
                    Some(description)
                },
                account_name: if account_name.is_empty() {
                    None
                } else {
                    Some(account_name)
                },
                card_name: if card_name.is_empty() {
                    None
                } else {
                    Some(card_name)
                },
                source_institution: Some("pasted_text".to_string()),
                balance_after: None,
                approval_number: None,
                raw_data: serde_json::json!({}),
                dedupe_key: None,
            });
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pasted_parser_works() {
        let text = "날짜,시간,가맹점,금액,카드\n2026-04-22,09:10,스타벅스,5500,딥드림\n";
        let parser = PastedCsvTextParser;
        let rows = parser.parse(text.as_bytes()).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].amount, 5500);
        assert_eq!(rows[0].merchant_name.as_deref(), Some("스타벅스"));
    }
}
