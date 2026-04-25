use std::future::Future;

use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};
use axum_extra::extract::cookie::CookieJar;
use uuid::Uuid;

use crate::{AppState, errors::AppError};

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: Uuid,
    pub display_name: String,
    pub session_token: String,
}

impl<S> FromRequestParts<S> for AuthUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        let app_state = AppState::from_ref(state);
        let headers = parts.headers.clone();

        async move {
            let jar = CookieJar::from_headers(&headers);
            let cookie_name = &app_state.config.session_cookie_name;

            let token = jar
                .get(cookie_name)
                .map(|cookie| cookie.value().to_string())
                .ok_or(AppError::Unauthorized)?;

            let user = sqlx::query_as::<_, crate::models::User>(
                r#"
                SELECT u.*
                FROM users u
                INNER JOIN sessions s ON s.user_id = u.id
                WHERE s.token = $1 AND s.expires_at > now()
                LIMIT 1
                "#,
            )
            .bind(&token)
            .fetch_optional(&app_state.pool)
            .await?
            .ok_or(AppError::Unauthorized)?;

            Ok(AuthUser {
                id: user.id,
                display_name: user.display_name,
                session_token: token,
            })
        }
    }
}
