use anyhow::{Result, bail};

use super::parser::{
    DetectInput, NormalizedTransaction, TransactionParser, header_map,
    parse_amount, parse_local_datetime, pick_col, row_value, sheet_rows,
};

#[derive(Debug, Default)]
pub struct ShinhanCardParser;

impl TransactionParser for ShinhanCardParser {
    fn name(&self) -> &'static str {
        "shinhan_card"
    }

    fn detect(&self, input: &DetectInput<'_>) -> f32 {
        let mut score: f32 = 0.0;
        if let Some(name) = input.filename {
            let lowered = name.to_lowercase();
            if lowered.contains("shinhan") || lowered.contains("신한") {
                score += 0.5;
            }
            if lowered.ends_with(".xls") || lowered.ends_with(".xlsx") {
                score += 0.2;
            }
        }

        let sample = input
            .sample_text
            .map(|v| v.to_string())
            .unwrap_or_else(|| String::from_utf8_lossy(input.content).to_string());
        for marker in ["거래일", "이용일자", "승인일자", "가맹점", "이용금액", "승인금액", "승인번호"] {
            if sample.contains(marker) {
                score += 0.05;
            }
        }
        score.min(1.0)
    }

    fn parse(&self, content: &[u8]) -> Result<Vec<NormalizedTransaction>> {
        let rows = sheet_rows(content)?;
        if rows.is_empty() {
            bail!("엑셀 데이터가 비어 있습니다");
        }

        let header_idx = rows
            .iter()
            .position(|row| {
                row.iter().any(|cell| {
                    let cell = cell.replace(' ', "");
                    cell.contains("이용일자")
                        || cell.contains("승인일자")
                        || cell.contains("거래일")
                        || cell.contains("매출일")
                        || cell.contains("가맹점")
                        || cell.contains("이용금액")
                })
            });

        let header_idx = match header_idx {
            Some(idx) => idx,
            None => {
                let preview: Vec<String> = rows.iter().take(5).map(|r| r.join(" | ")).collect();
                bail!(
                    "신한카드 헤더 행을 찾을 수 없습니다. 파일의 첫 행들:\n{}",
                    preview.join("\n")
                );
            }
        };

        let headers = rows[header_idx].clone();
        let map = header_map(&headers);

        tracing::debug!(headers = ?headers, "신한카드 헤더 인식");

        let date_idx = match pick_col(&map, &["거래일", "승인일자", "이용일자", "일자", "날짜", "매출일"]) {
            Some(idx) => idx,
            None => bail!(
                "날짜 열을 찾을 수 없습니다. 인식된 헤더: {}",
                headers.join(", ")
            ),
        };
        let merchant_idx = pick_col(&map, &["가맹점명", "가맹점", "이용처", "가맹점/ATM명"]);
        let amount_idx = pick_col(&map, &["금액", "승인금액", "이용금액", "거래금액", "매출금액"]);
        let card_name_idx = pick_col(&map, &["이용카드", "카드명", "카드"]);
        let approval_idx = pick_col(&map, &["승인번호", "승인no", "approval"]);
        let cancel_idx = pick_col(&map, &["취소상태", "취소여부", "승인취소", "취소", "매입구분"]);
        let installment_idx = pick_col(&map, &["이용구분", "할부", "할부개월"]);

        let mut result = Vec::new();

        for row in rows.into_iter().skip(header_idx + 1) {
            let date_raw = row_value(&row, Some(date_idx)).trim().to_string();
            if date_raw.is_empty() {
                continue;
            }

            // "2026.04.24 13:48" 형식 — 날짜+시간 합쳐진 경우
            let transaction_at = if date_raw.contains(' ') {
                let parts: Vec<&str> = date_raw.splitn(2, ' ').collect();
                parse_local_datetime(parts[0], Some(parts[1]))
            } else {
                parse_local_datetime(&date_raw, None)
            };

            let transaction_at = match transaction_at {
                Some(v) => v,
                None => continue,
            };

            let amount_raw = row_value(&row, amount_idx);
            let amount = match parse_amount(amount_raw) {
                Some(v) if v != 0 => v,
                _ => continue,
            };

            let cancel_text = row_value(&row, cancel_idx).trim().to_string();
            // 취소상태 = "취소" 또는 매입구분 = "승인취소"/"거래취소" → 무시
            let is_cancelled = cancel_text == "취소"
                || cancel_text.contains("취소")
                || cancel_text.eq_ignore_ascii_case("Y");

            if is_cancelled {
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
                amount: amount.abs(),
                merchant_name: if merchant_name.is_empty() { None } else { Some(merchant_name) },
                description: if installment.is_empty() { None } else { Some(installment) },
                account_name: None,
                card_name: if card_name.is_empty() { None } else { Some(card_name) },
                source_institution: Some("shinhan_card".to_string()),
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
    fn detect_score_shinhan_xls() {
        let parser = ShinhanCardParser;
        let score = parser.detect(&DetectInput {
            filename: Some("Shinhancard_20260424.xls"),
            sample_text: Some("거래일,가맹점명,금액,취소상태"),
            content: b"",
        });
        assert!(score > 0.5, "score={score}");
    }
}
