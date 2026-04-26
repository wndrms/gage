use crate::errors::AppError;

const KREAM_KEYWORDS: &[&str] = &[
    "kream",
    "\u{D06C}\u{B9BC}",
    "\u{B86F}\u{B370}\u{AE00}\u{B85C}\u{BC8C}\u{B85C}\u{C9C0}\u{C2A4}",
    "\u{B86F}\u{B370}\u{D0DD}\u{BC30}",
    "\u{D0DD}\u{BC30}",
    "\u{BC30}\u{C1A1}\u{BE44}",
    "\u{C6B4}\u{C1A1}\u{C7A5}",
    "\u{B300}\u{D55C}\u{D1B5}\u{C6B4}",
    "cj\u{B300}\u{D55C}\u{D1B5}\u{C6B4}",
    "\u{D3B8}\u{C758}\u{C810}\u{D0DD}\u{BC30}",
];

pub fn resolve_scope(
    explicit_scope: Option<&str>,
    merchant_name: Option<&str>,
    description: Option<&str>,
) -> Result<String, AppError> {
    if let Some(scope) = explicit_scope {
        let scope = scope.trim();
        if scope.is_empty() {
            return Ok(infer_scope(merchant_name, description));
        }
        return match scope {
            "personal" | "kream" => Ok(scope.to_string()),
            _ => Err(AppError::BadRequest(
                "scope must be either personal or kream".to_string(),
            )),
        };
    }

    Ok(infer_scope(merchant_name, description))
}

pub fn infer_scope(merchant_name: Option<&str>, description: Option<&str>) -> String {
    if is_kream_related(merchant_name, description) {
        "kream".to_string()
    } else {
        "personal".to_string()
    }
}

fn is_kream_related(merchant_name: Option<&str>, description: Option<&str>) -> bool {
    let haystack = format!(
        "{} {}",
        merchant_name.unwrap_or_default(),
        description.unwrap_or_default()
    )
    .to_lowercase()
    .replace(' ', "");

    KREAM_KEYWORDS
        .iter()
        .any(|keyword| haystack.contains(&keyword.to_lowercase().replace(' ', "")))
}
