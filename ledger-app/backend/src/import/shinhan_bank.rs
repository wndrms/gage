use anyhow::Result;

use super::parser::{
    DetectInput, NormalizedTransaction, TransactionParser, csv_records, ensure_column, header_map,
    parse_amount, parse_default_type, parse_local_datetime, pick_col, row_value,
};

#[derive(Debug, Default)]
pub struct ShinhanBankParser;

impl TransactionParser for ShinhanBankParser {
    fn name(&self) -> &'static str {
        "shinhan_bank"
    }

    fn detect(&self, input: &DetectInput<'_>) -> f32 {
        let mut score: f32 = 0.0;
        if let Some(name) = input.filename {
            let lowered = name.to_lowercase();
            if lowered.contains("shinhan") || lowered.contains("신한") {
                score += 0.4;
            }
            if lowered.ends_with(".csv") {
                score += 0.1;
            }
        }

        let sample = input
            .sample_text
            .map(|v| v.to_string())
            .unwrap_or_else(|| String::from_utf8_lossy(input.content).to_string());

        for marker in ["거래일자", "거래시간", "출금(원)", "입금(원)", "잔액(원)"] {
            if sample.contains(marker) {
                score += 0.15;
            }
        }

        score.min(1.0)
    }

    fn parse(&self, content: &[u8]) -> Result<Vec<NormalizedTransaction>> {
        let (headers, rows) = csv_records(content)?;
        let map = header_map(&headers);

        let date_idx = ensure_column(pick_col(&map, &["거래일자", "일자", "날짜"]), "거래일자 열이 필요합니다")?;
        let time_idx = pick_col(&map, &["거래시간", "시간"]);
        let brief_idx = pick_col(&map, &["적요", "내용"]);
        let outflow_idx = pick_col(&map, &["출금(원)", "출금", "지출"]);
        let inflow_idx = pick_col(&map, &["입금(원)", "입금", "수입"]);
        let desc_idx = pick_col(&map, &["내용", "가맹점", "상호"]);
        let balance_idx = pick_col(&map, &["잔액(원)", "잔액"]);
        let branch_idx = pick_col(&map, &["거래점", "채널"]);

        let mut result = Vec::new();

        for row in rows {
            let date = row_value(&row, Some(date_idx));
            let time = row_value(&row, time_idx);
            let transaction_at = match parse_local_datetime(date, Some(time)) {
                Some(v) => v,
                None => continue,
            };

            let outflow = parse_amount(row_value(&row, outflow_idx));
            let inflow = parse_amount(row_value(&row, inflow_idx));
            let Some((typ, amount)) = parse_default_type(outflow, inflow) else {
                continue;
            };

            let description = row_value(&row, brief_idx).trim().to_string();
            let merchant = row_value(&row, desc_idx).trim().to_string();
            let balance_after = parse_amount(row_value(&row, balance_idx));
            let branch = row_value(&row, branch_idx).trim().to_string();

            result.push(NormalizedTransaction {
                transaction_at,
                posted_at: None,
                r#type: typ,
                amount,
                merchant_name: if merchant.is_empty() { None } else { Some(merchant) },
                description: if description.is_empty() {
                    None
                } else {
                    Some(description)
                },
                account_name: None,
                card_name: None,
                source_institution: Some("shinhan_bank".to_string()),
                balance_after,
                approval_number: None,
                raw_data: serde_json::json!({
                    "branch_or_channel": branch,
                }),
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
    fn shinhan_bank_parser_works() {
        let csv = "거래일자,거래시간,적요,출금(원),입금(원),내용,잔액(원),거래점\n2026-04-21,08:30:00,카드결제,12000,0,편의점,100000,모바일\n2026-04-21,09:00:00,급여,0,2500000,회사,2600000,인터넷\n";
        let parser = ShinhanBankParser;
        let rows = parser.parse(csv.as_bytes()).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].r#type, "expense");
        assert_eq!(rows[0].amount, 12000);
        assert_eq!(rows[1].r#type, "income");
        assert_eq!(rows[1].amount, 2500000);
    }
}
