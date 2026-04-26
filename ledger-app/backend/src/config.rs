use std::{env, net::SocketAddr, path::PathBuf};

use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub host: String,
    pub port: u16,
    pub admin_password: String,
    pub session_cookie_name: String,
    pub cookie_secure: bool,
    pub frontend_origin: String,
    pub import_dir: PathBuf,
    pub telegram_bot_token: Option<String>,
    pub telegram_webhook_secret: Option<String>,
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        let database_url =
            env::var("DATABASE_URL").context("DATABASE_URL 환경변수가 필요합니다")?;
        let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = env::var("PORT")
            .ok()
            .and_then(|v| v.parse::<u16>().ok())
            .unwrap_or(8080);
        let admin_password =
            env::var("ADMIN_PASSWORD").context("ADMIN_PASSWORD 환경변수가 필요합니다")?;
        if admin_password.len() < 8 {
            anyhow::bail!("ADMIN_PASSWORD는 8자 이상이어야 합니다");
        }
        let session_cookie_name =
            env::var("SESSION_COOKIE_NAME").unwrap_or_else(|_| "ledger_session".to_string());
        let cookie_secure = env::var("COOKIE_SECURE")
            .ok()
            .and_then(|v| v.parse::<bool>().ok())
            .unwrap_or(false);
        let frontend_origin =
            env::var("FRONTEND_ORIGIN").unwrap_or_else(|_| "http://localhost:5173".to_string());
        let import_dir = PathBuf::from(
            env::var("LEDGER_IMPORT_DIR").unwrap_or_else(|_| "./data/imports".to_string()),
        );
        let telegram_bot_token = env::var("TELEGRAM_BOT_TOKEN")
            .ok()
            .filter(|s| !s.is_empty());
        let telegram_webhook_secret = env::var("TELEGRAM_WEBHOOK_SECRET")
            .ok()
            .filter(|s| !s.is_empty());

        Ok(Self {
            database_url,
            host,
            port,
            admin_password,
            session_cookie_name,
            cookie_secure,
            frontend_origin,
            import_dir,
            telegram_bot_token,
            telegram_webhook_secret,
        })
    }

    pub fn addr(&self) -> Result<SocketAddr> {
        format!("{}:{}", self.host, self.port)
            .parse()
            .context("서버 주소를 파싱할 수 없습니다")
    }
}
