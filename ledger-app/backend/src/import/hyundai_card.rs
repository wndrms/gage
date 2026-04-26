use anyhow::{Result, bail};

use super::parser::{
    DetectInput, NormalizedTransaction, TransactionParser, header_map, html_table_rows,
    parse_amount, parse_korean_date, pick_col, row_value,
};
use chrono::{FixedOffset, NaiveDateTime, NaiveTime};

#[derive(Debug, Default)]
pub struct HyundaiCardParser;

impl TransactionParser for HyundaiCardParser {
    fn name(&self) -> &'static str {
        "hyundai_card"
    }

    fn detect(&self, input: &DetectInput<'_>) -> f32 {
        let mut score: f32 = 0.0;
        if let Some(name) = input.filename {
            let lowered = name.to_lowercase();
            if lowered.contains("hyundai") || lowered.contains("현대") {
                score += 0.5;
            }
            if lowered.ends_with(".xls") || lowered.ends_with(".xlsx") {
                score += 0.1;
            }
        }

        let sample = input
            .sample_text
            .map(|v| v.to_string())
            .unwrap_or_else(|| String::from_utf8_lossy(input.content).to_string());
        for marker in [
            "이용일자",
            "카드명(카드뒤4자리)",
            "이용금액",
            "결제예정일",
            "현대카드",
        ] {
            if sample.contains(marker) {
                score += 0.1;
            }
        }
        score.min(1.0)
    }

    fn parse(&self, content: &[u8]) -> Result<Vec<NormalizedTransaction>> {
        let rows = html_table_rows(content)?;
        if rows.is_empty() {
            bail!("현대카드 파일에서 데이터를 읽을 수 없습니다");
        }

        // 헤더 행 찾기
        let header_idx = rows.iter().position(|row| {
            row.iter().any(|cell| {
                cell.contains("이용일자") || cell.contains("가맹점명") || cell.contains("이용금액")
            })
        });

        let header_idx = match header_idx {
            Some(idx) => idx,
            None => {
                let preview: Vec<String> = rows.iter().take(5).map(|r| r.join(" | ")).collect();
                bail!(
                    "현대카드 헤더 행을 찾을 수 없습니다. 첫 행들:\n{}",
                    preview.join("\n")
                );
            }
        };

        let headers = rows[header_idx].clone();
        let map = header_map(&headers);

        tracing::debug!(headers = ?headers, "현대카드 헤더 인식");

        let date_idx = match pick_col(&map, &["이용일자", "거래일", "승인일자"]) {
            Some(idx) => idx,
            None => bail!("날짜 열을 찾을 수 없습니다. 헤더: {}", headers.join(", ")),
        };
        let merchant_idx = pick_col(&map, &["가맹점명", "가맹점", "이용처"]);
        let amount_idx = pick_col(&map, &["이용금액", "금액", "승인금액"]);
        let card_name_idx = pick_col(&map, &["카드명(카드뒤4자리)", "카드명", "이용카드"]);
        let approval_idx = pick_col(&map, &["승인번호"]);
        let installment_idx = pick_col(&map, &["이용구분", "할부개월"]);

        let kst = FixedOffset::east_opt(9 * 3600).unwrap();
        let mut result = Vec::new();

        for row in rows.into_iter().skip(header_idx + 1) {
            let date_raw = row_value(&row, Some(date_idx)).trim().to_string();
            if date_raw.is_empty() {
                continue;
            }

            let date = match parse_korean_date(&date_raw) {
                Some(d) => d,
                None => continue,
            };

            let naive = NaiveDateTime::new(date, NaiveTime::from_hms_opt(0, 0, 0).unwrap());
            let transaction_at = naive
                .and_local_timezone(kst)
                .single()
                .map(|dt| dt.with_timezone(&chrono::Utc));
            let transaction_at = match transaction_at {
                Some(v) => v,
                None => continue,
            };

            let amount_raw = row_value(&row, amount_idx);
            let amount = match parse_amount(amount_raw) {
                Some(v) => v,
                None => continue,
            };

            // 음수 금액 = 취소/할인 → 무시
            if amount <= 0 {
                continue;
            }

            let merchant_name = row_value(&row, merchant_idx).trim().to_string();
            let card_name = row_value(&row, card_name_idx).trim().to_string();
            let approval_number = row_value(&row, approval_idx).trim().to_string();
            let installment = row_value(&row, installment_idx).trim().to_string();

            result.push(NormalizedTransaction {
                transaction_at,
                posted_at: None,
                r#type: "expense".to_string(),
                amount,
                merchant_name: if merchant_name.is_empty() {
                    None
                } else {
                    Some(merchant_name)
                },
                description: if installment.is_empty() {
                    None
                } else {
                    Some(installment)
                },
                account_name: None,
                card_name: if card_name.is_empty() {
                    None
                } else {
                    Some(card_name)
                },
                source_institution: Some("hyundai_card".to_string()),
                balance_after: None,
                approval_number: if approval_number.is_empty() {
                    None
                } else {
                    Some(approval_number)
                },
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
    fn detect_score_hyundai() {
        let parser = HyundaiCardParser;
        let score = parser.detect(&DetectInput {
            filename: Some("hyundaicard_20260424.xls"),
            sample_text: Some("이용일자,카드명(카드뒤4자리),가맹점명,이용금액"),
            content: b"",
        });
        assert!(score > 0.5, "score={score}");
    }
}
