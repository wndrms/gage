mod auth;
mod card_rules;
mod config;
mod db;
mod errors;
mod import;
mod models;
mod routes;
mod services;
mod telegram;

use std::sync::Arc;

use anyhow::Result;
use axum::{
    Router,
    http::{
        Method,
        header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, ORIGIN},
    },
};
use config::AppConfig;
use sqlx::PgPool;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Arc<AppConfig>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,ledger_backend=debug".to_string()),
        )
        .init();

    let config = Arc::new(AppConfig::from_env()?);
    tokio::fs::create_dir_all(&config.import_dir).await?;

    let pool = db::connect(&config).await?;
    db::migrate(&pool).await?;
    services::seed::seed_defaults(&pool, &config.admin_password).await?;

    let state = AppState {
        pool: pool.clone(),
        config: config.clone(),
    };

    import::watcher::spawn_import_watcher(state.clone()).await?;

    let cors = CorsLayer::new()
        .allow_origin(config.frontend_origin.parse::<axum::http::HeaderValue>()?)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([ACCEPT, AUTHORIZATION, CONTENT_TYPE, ORIGIN])
        .allow_credentials(true);

    let app = Router::new()
        .route("/health", axum::routing::get(routes::health))
        .nest("/api", routes::api_router())
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state);

    let addr = config.addr()?;
    tracing::info!("서버 시작: {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
