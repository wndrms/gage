use anyhow::{Result, bail};

use super::parser::{
    DetectInput, NormalizedTransaction, TransactionParser, header_map,
    parse_amount, parse_local_datetime, sheet_rows_named, pick_col, row_value,
};

#[derive(Debug, Default)]
pub struct SamsungCardParser;

impl TransactionParser for SamsungCardParser {
    fn name(&self) -> &'static str {
        "samsung_card"
    }

    fn detect(&self, input: &DetectInput<'_>) -> f32 {
        let mut score: f32 = 0.0;
        if let Some(name) = input.filename {
            let lowered = name.to_lowercase();
            if lowered.contains("samsung") || lowered.contains("삼성") {
                score += 0.5;
            }
            if lowered.ends_with(".xlsx") || lowered.ends_with(".xls") {
                score += 0.1;
            }
        }

        let sample = input
            .sample_text
            .map(|v| v.to_string())
            .unwrap_or_else(|| String::from_utf8_lossy(input.content).to_string());
        for marker in ["승인일자", "승인시각", "승인금액(원)", "취소여부", "국내이용내역"] {
            if sample.contains(marker) {
                score += 0.1;
            }
        }
        score.min(1.0)
    }

    fn parse(&self, content: &[u8]) -> Result<Vec<NormalizedTransaction>> {
        // 삼성카드는 "■ 국내이용내역" 시트에 실제 거래 데이터가 있습니다
        let rows = sheet_rows_named(content, "국내이용내역")?;
        if rows.is_empty() {
            bail!("삼성카드 파일에서 데이터를 읽을 수 없습니다");
        }

        let header_idx = rows
            .iter()
            .position(|row| {
                row.iter().any(|cell| {
                    cell.contains("승인일자") || cell.contains("가맹점명") || cell.contains("승인금액")
                })
            });

        let header_idx = match header_idx {
            Some(idx) => idx,
            None => {
                let preview: Vec<String> = rows.iter().take(5).map(|r| r.join(" | ")).collect();
                bail!(
                    "삼성카드 헤더 행을 찾을 수 없습니다. 첫 행들:\n{}",
                    preview.join("\n")
                );
            }
        };

        let headers = rows[header_idx].clone();
        let map = header_map(&headers);

        tracing::debug!(headers = ?headers, "삼성카드 헤더 인식");

        let date_idx = match pick_col(&map, &["승인일자", "이용일자", "거래일"]) {
            Some(idx) => idx,
            None => bail!("날짜 열을 찾을 수 없습니다. 헤더: {}", headers.join(", ")),
        };
        let time_idx = pick_col(&map, &["승인시각", "이용시간", "시간"]);
        let merchant_idx = pick_col(&map, &["가맹점명", "가맹점"]);
        let amount_idx = pick_col(&map, &["승인금액(원)", "승인금액", "이용금액", "금액"]);
        let card_no_idx = pick_col(&map, &["카드번호"]);
        let approval_idx = pick_col(&map, &["승인번호"]);
        let cancel_idx = pick_col(&map, &["취소여부"]);
        let installment_idx = pick_col(&map, &["일시불할부구분", "이용구분"]);

        let mut result = Vec::new();

        for row in rows.into_iter().skip(header_idx + 1) {
            let date_raw = row_value(&row, Some(date_idx)).trim().to_string();
            if date_raw.is_empty() {
                continue;
            }

            let time_raw = row_value(&row, time_idx).trim().to_string();
            let transaction_at = match parse_local_datetime(
                &date_raw,
                if time_raw.is_empty() { None } else { Some(&time_raw) },
            ) {
                Some(v) => v,
                None => continue,
            };

            let amount_raw = row_value(&row, amount_idx);
            let amount = match parse_amount(amount_raw) {
                Some(v) => v,
                None => continue,
            };

            // 취소여부 = "전체취소" / "부분취소" → 무시
            let cancel_text = row_value(&row, cancel_idx).trim().to_string();
            if cancel_text.contains("취소") {
                continue;
            }

            // 음수 금액 (할인 행 등) → 무시
            if amount <= 0 {
                continue;
            }

            let merchant_name = row_value(&row, merchant_idx).trim().to_string();
            let card_no = row_value(&row, card_no_idx).trim().to_string();
            let approval_number = row_value(&row, approval_idx).trim().to_string();
            let installment = row_value(&row, installment_idx).trim().to_string();

            result.push(NormalizedTransaction {
                transaction_at,
                posted_at: None,
                r#type: "expense".to_string(),
                amount,
                merchant_name: if merchant_name.is_empty() { None } else { Some(merchant_name) },
                description: if installment.is_empty() { None } else { Some(installment) },
                account_name: None,
                card_name: if card_no.is_empty() { None } else { Some(card_no) },
                source_institution: Some("samsung_card".to_string()),
                balance_after: None,
                approval_number: if approval_number.is_empty() { None } else { Some(approval_number) },
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
    fn detect_score_samsung() {
        let parser = SamsungCardParser;
        let score = parser.detect(&DetectInput {
            filename: Some("samsung.xlsx"),
            sample_text: Some("승인일자,승인시각,가맹점명,승인금액(원),취소여부"),
            content: b"",
        });
        assert!(score > 0.5, "score={score}");
    }
}
