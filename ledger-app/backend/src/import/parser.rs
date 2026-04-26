use std::{collections::HashMap, io::Cursor};

use anyhow::{Context, Result, bail};
use calamine::{Data, Reader};
use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedTransaction {
    pub transaction_at: DateTime<Utc>,
    pub posted_at: Option<DateTime<Utc>>,
    pub r#type: String,
    pub amount: i64,
    pub merchant_name: Option<String>,
    pub description: Option<String>,
    pub account_name: Option<String>,
    pub card_name: Option<String>,
    pub source_institution: Option<String>,
    pub balance_after: Option<i64>,
    pub approval_number: Option<String>,
    pub raw_data: serde_json::Value,
    pub dedupe_key: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DetectInput<'a> {
    pub filename: Option<&'a str>,
    pub sample_text: Option<&'a str>,
    pub content: &'a [u8],
}

pub trait TransactionParser: Send + Sync {
    fn name(&self) -> &'static str;
    fn detect(&self, input: &DetectInput<'_>) -> f32;
    fn parse(&self, content: &[u8]) -> Result<Vec<NormalizedTransaction>>;
}

pub fn normalize_header(value: &str) -> String {
    value
        .trim()
        .replace(' ', "")
        .replace('\u{feff}', "")
        .to_lowercase()
}

pub fn parse_amount(value: &str) -> Option<i64> {
    let cleaned = value
        .trim()
        .replace(',', "")
        .replace('원', "")
        .replace(' ', "")
        .replace('+', "");

    if cleaned.is_empty() {
        return None;
    }

    cleaned.parse::<i64>().ok()
}

pub fn parse_local_datetime(date: &str, time: Option<&str>) -> Option<DateTime<Utc>> {
    let date = date.trim();
    let time = time.unwrap_or("00:00:00").trim();
    let date_formats = ["%Y-%m-%d", "%Y.%m.%d", "%Y/%m/%d", "%Y%m%d"];
    let time_formats = ["%H:%M:%S", "%H:%M"];

    let date = date_formats
        .iter()
        .find_map(|f| NaiveDate::parse_from_str(date, f).ok())?;
    let time = time_formats
        .iter()
        .find_map(|f| NaiveTime::parse_from_str(time, f).ok())
        .unwrap_or_else(|| NaiveTime::from_hms_opt(0, 0, 0).unwrap());

    let naive = NaiveDateTime::new(date, time);
    let kst = FixedOffset::east_opt(9 * 3600)?;
    let local_dt = naive.and_local_timezone(kst).single()?;
    Some(local_dt.with_timezone(&Utc))
}

pub fn csv_records(content: &[u8]) -> Result<(Vec<String>, Vec<Vec<String>>)> {
    let text = String::from_utf8_lossy(content);
    let delimiter = if text.lines().next().unwrap_or("").contains('\t') {
        b'\t'
    } else if text.lines().next().unwrap_or("").matches(';').count() > 3 {
        b';'
    } else {
        b','
    };

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .delimiter(delimiter)
        .from_reader(Cursor::new(content));

    let headers = reader
        .headers()
        .context("CSV 헤더를 읽을 수 없습니다")?
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>();

    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record?;
        rows.push(record.iter().map(|v| v.to_string()).collect());
    }

    Ok((headers, rows))
}

pub fn sheet_rows(content: &[u8]) -> Result<Vec<Vec<String>>> {
    let cursor = Cursor::new(content.to_vec());
    let mut workbook =
        calamine::open_workbook_auto_from_rs(cursor).context("엑셀 파일을 열 수 없습니다")?;
    let first_sheet = workbook
        .sheet_names()
        .first()
        .cloned()
        .context("엑셀 시트가 없습니다")?;

    let range = workbook
        .worksheet_range(&first_sheet)
        .context("시트를 읽을 수 없습니다")?;

    let mut rows = Vec::new();
    for row in range.rows() {
        rows.push(row.iter().map(data_to_string).collect());
    }
    Ok(rows)
}

/// 특정 시트명을 포함하는 시트를 찾아 행 목록 반환 (대소문자 무시, 부분 일치)
pub fn sheet_rows_named(content: &[u8], name_hint: &str) -> Result<Vec<Vec<String>>> {
    let cursor = Cursor::new(content.to_vec());
    let mut workbook =
        calamine::open_workbook_auto_from_rs(cursor).context("엑셀 파일을 열 수 없습니다")?;

    let sheet_name = workbook
        .sheet_names()
        .iter()
        .find(|n| n.to_lowercase().contains(&name_hint.to_lowercase()))
        .cloned()
        .or_else(|| workbook.sheet_names().first().cloned())
        .context("시트를 찾을 수 없습니다")?;

    let range = workbook
        .worksheet_range(&sheet_name)
        .context("시트를 읽을 수 없습니다")?;

    let mut rows = Vec::new();
    for row in range.rows() {
        rows.push(row.iter().map(data_to_string).collect());
    }
    Ok(rows)
}

/// HTML로 위장된 xls 파일(현대카드 등)에서 테이블을 파싱합니다.
/// `<td>`와 `<th>` 모두 셀로 인식합니다.
pub fn html_table_rows(content: &[u8]) -> Result<Vec<Vec<String>>> {
    let text = String::from_utf8_lossy(content);
    let lower = text.to_lowercase();
    if !lower.contains("</td>") && !lower.contains("</th>") {
        bail!("HTML 테이블이 없습니다");
    }

    let mut all_rows = Vec::new();
    let mut pos = 0;

    while let Some(tr_start) = lower[pos..].find("<tr") {
        let abs_tr = pos + tr_start;
        let Some(tr_end) = lower[abs_tr..].find("</tr>") else {
            break;
        };
        let tr_content = &text[abs_tr..abs_tr + tr_end + 5];
        let tr_lower = tr_content.to_lowercase();

        let mut cells = Vec::new();
        let mut cell_pos = 0;

        loop {
            // <td 또는 <th 중 더 앞에 있는 것을 선택
            let next_td = tr_lower[cell_pos..].find("<td").map(|i| (i, "td"));
            let next_th = tr_lower[cell_pos..].find("<th").map(|i| (i, "th"));
            let (rel_start, tag) = match (next_td, next_th) {
                (None, None) => break,
                (Some(a), None) => a,
                (None, Some(b)) => b,
                (Some(a), Some(b)) => {
                    if a.0 <= b.0 {
                        a
                    } else {
                        b
                    }
                }
            };
            let abs_cell = cell_pos + rel_start;
            let close_open = tr_lower[abs_cell..].find('>').unwrap_or(0);
            let content_start = abs_cell + close_open + 1;
            let close_tag = format!("</{}>", tag);
            let Some(cell_end) = tr_lower[content_start..].find(close_tag.as_str()) else {
                break;
            };
            let raw = &tr_content[content_start..content_start + cell_end];
            cells.push(strip_html_tags(raw));
            cell_pos = content_start + cell_end + close_tag.len();
        }

        if !cells.is_empty() {
            all_rows.push(cells);
        }
        pos = abs_tr + 1;
    }

    if all_rows.is_empty() {
        bail!("HTML 테이블에서 데이터를 추출할 수 없습니다");
    }
    Ok(all_rows)
}

fn strip_html_tags(s: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }
    // HTML 엔티티 간단 처리
    result
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
        .replace("&#160;", " ")
        .trim()
        .to_string()
}

/// `2026년 04월 23일` 형식 날짜 파싱
pub fn parse_korean_date(s: &str) -> Option<NaiveDate> {
    let s = s.trim();
    // "2026년 04월 23일" 또는 "2026년04월23일"
    let re_parts: Vec<&str> = s.split(['년', '월', '일']).collect();
    if re_parts.len() >= 3 {
        let y = re_parts[0].trim().parse::<i32>().ok()?;
        let m = re_parts[1].trim().parse::<u32>().ok()?;
        let d = re_parts[2].trim().parse::<u32>().ok()?;
        return NaiveDate::from_ymd_opt(y, m, d);
    }
    None
}

fn data_to_string(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(v) => v.trim().to_string(),
        Data::Float(v) => {
            if v.fract() == 0.0 {
                format!("{}", *v as i64)
            } else {
                format!("{}", v)
            }
        }
        Data::Int(v) => v.to_string(),
        Data::Bool(v) => v.to_string(),
        Data::DateTime(v) => v.to_string(),
        _ => cell.to_string(),
    }
}

pub fn header_map(headers: &[String]) -> HashMap<String, usize> {
    headers
        .iter()
        .enumerate()
        .map(|(idx, h)| (normalize_header(h), idx))
        .collect()
}

pub fn pick_col(map: &HashMap<String, usize>, candidates: &[&str]) -> Option<usize> {
    candidates
        .iter()
        .find_map(|key| map.get(&normalize_header(key)).copied())
}

pub fn ensure_column(idx: Option<usize>, message: &str) -> Result<usize> {
    idx.ok_or_else(|| anyhow::anyhow!(message.to_string()))
}

pub fn row_value<'a>(row: &'a [String], idx: Option<usize>) -> &'a str {
    idx.and_then(|i| row.get(i).map(|s| s.as_str()))
        .unwrap_or("")
}

pub fn parse_default_type(outflow: Option<i64>, inflow: Option<i64>) -> Option<(String, i64)> {
    match (outflow.unwrap_or(0), inflow.unwrap_or(0)) {
        (o, _) if o > 0 => Some(("expense".to_string(), o)),
        (_, i) if i > 0 => Some(("income".to_string(), i)),
        _ => None,
    }
}

pub fn require_non_empty_rows(rows: &[Vec<String>]) -> Result<()> {
    if rows.is_empty() {
        bail!("파싱할 데이터가 없습니다");
    }
    Ok(())
}
