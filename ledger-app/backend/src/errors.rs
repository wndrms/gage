use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("인증이 필요합니다")]
    Unauthorized,
    #[error("권한이 없습니다")]
    Forbidden,
    #[error("요청 데이터가 올바르지 않습니다: {0}")]
    BadRequest(String),
    #[error("데이터를 찾을 수 없습니다")]
    NotFound,
    #[error("서버 오류가 발생했습니다")]
    Internal,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::Forbidden => StatusCode::FORBIDDEN,
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::NotFound => StatusCode::NOT_FOUND,
            AppError::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = Json(ErrorResponse {
            message: self.to_string(),
        });

        (status, body).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        tracing::error!(error = ?err, "데이터베이스 오류");
        AppError::Internal
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        tracing::error!(error = ?err, "내부 오류");
        AppError::Internal
    }
}
