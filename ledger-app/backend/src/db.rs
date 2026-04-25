use anyhow::Result;
use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::config::AppConfig;

pub async fn connect(config: &AppConfig) -> Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await?;

    Ok(pool)
}

pub async fn migrate(pool: &PgPool) -> Result<()> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}
