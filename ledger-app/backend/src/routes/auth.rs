use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use axum::{Json, extract::State};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use time::Duration;

use crate::{AppState, auth::extractor::AuthUser, errors::AppError};

const SESSION_DAYS: i64 = 30;

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub password: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AuthMeResponse {
    pub id: uuid::Uuid,
    pub display_name: String,
    pub role: String,
}

pub async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(payload): Json<LoginRequest>,
) -> Result<(CookieJar, Json<AuthMeResponse>), AppError> {
    if payload.password.trim().is_empty() {
        return Err(AppError::BadRequest("비밀번호를 입력해 주세요".to_string()));
    }

    let user = if let Some(display_name) = payload.display_name.as_ref() {
        sqlx::query_as::<_, crate::models::User>(
            "SELECT * FROM users WHERE display_name = $1 AND password_hash IS NOT NULL LIMIT 1",
        )
        .bind(display_name)
        .fetch_optional(&state.pool)
        .await?
    } else {
        sqlx::query_as::<_, crate::models::User>(
            "SELECT * FROM users WHERE password_hash IS NOT NULL ORDER BY created_at ASC LIMIT 1",
        )
        .fetch_optional(&state.pool)
        .await?
    }
    .ok_or(AppError::Unauthorized)?;

    let hash = user.password_hash.ok_or(AppError::Unauthorized)?;
    let parsed_hash = PasswordHash::new(&hash)
        .map_err(|_| AppError::BadRequest("비밀번호 정보를 확인할 수 없습니다".to_string()))?;

    Argon2::default()
        .verify_password(payload.password.as_bytes(), &parsed_hash)
        .map_err(|_| AppError::Unauthorized)?;

    let token = uuid::Uuid::new_v4().to_string();
    let expires_at = Utc::now() + chrono::Duration::days(SESSION_DAYS);

    sqlx::query("INSERT INTO sessions (token, user_id, expires_at) VALUES ($1, $2, $3)")
        .bind(&token)
        .bind(user.id)
        .bind(expires_at)
        .execute(&state.pool)
        .await?;

    let cookie = Cookie::build((state.config.session_cookie_name.clone(), token))
        .path("/")
        .http_only(true)
        .secure(state.config.cookie_secure)
        .same_site(SameSite::Lax)
        .max_age(Duration::days(SESSION_DAYS))
        .build();

    let jar = jar.add(cookie);

    Ok((
        jar,
        Json(AuthMeResponse {
            id: user.id,
            display_name: user.display_name,
            role: user.role,
        }),
    ))
}

pub async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<(CookieJar, Json<serde_json::Value>), AppError> {
    if let Some(token) = jar
        .get(&state.config.session_cookie_name)
        .map(|cookie| cookie.value().to_string())
    {
        sqlx::query("DELETE FROM sessions WHERE token = $1")
            .bind(&token)
            .execute(&state.pool)
            .await?;
    }

    let jar = jar.remove(
        Cookie::build((state.config.session_cookie_name.clone(), ""))
            .path("/")
            .build(),
    );
    Ok((
        jar,
        Json(serde_json::json!({"message": "로그아웃되었습니다"})),
    ))
}

pub async fn me(auth: AuthUser) -> Result<Json<AuthMeResponse>, AppError> {
    Ok(Json(AuthMeResponse {
        id: auth.id,
        display_name: auth.display_name,
        role: auth.role,
    }))
}

#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

pub async fn change_password(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<ChangePasswordRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let new_pw = payload.new_password.trim();
    if new_pw.len() < 8 {
        return Err(AppError::BadRequest(
            "새 비밀번호는 8자 이상이어야 합니다".to_string(),
        ));
    }
    if new_pw == payload.current_password.trim() {
        return Err(AppError::BadRequest(
            "새 비밀번호가 현재 비밀번호와 동일합니다".to_string(),
        ));
    }

    let user = sqlx::query_as::<_, crate::models::User>("SELECT * FROM users WHERE id = $1")
        .bind(auth.id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let hash = user.password_hash.ok_or(AppError::Unauthorized)?;
    let parsed_hash = PasswordHash::new(&hash)
        .map_err(|_| AppError::BadRequest("비밀번호 정보를 확인할 수 없습니다".to_string()))?;

    Argon2::default()
        .verify_password(payload.current_password.as_bytes(), &parsed_hash)
        .map_err(|_| AppError::BadRequest("현재 비밀번호가 올바르지 않습니다".to_string()))?;

    let salt = SaltString::generate(&mut OsRng);
    let new_hash = Argon2::default()
        .hash_password(new_pw.as_bytes(), &salt)
        .map_err(|_| AppError::Internal)?
        .to_string();

    let mut tx = state.pool.begin().await?;

    sqlx::query("UPDATE users SET password_hash = $2, updated_at = now() WHERE id = $1")
        .bind(auth.id)
        .bind(&new_hash)
        .execute(&mut *tx)
        .await?;

    // 현재 세션 외 모든 세션 무효화 (탈취된 세션 차단)
    sqlx::query("DELETE FROM sessions WHERE user_id = $1 AND token <> $2")
        .bind(auth.id)
        .bind(&auth.session_token)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(Json(
        serde_json::json!({"message": "비밀번호가 변경되었습니다"}),
    ))
}
