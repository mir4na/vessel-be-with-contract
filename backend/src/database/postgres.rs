use anyhow::Result;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tracing::info;

use crate::config::Config;

pub type DbPool = PgPool;

/// Create a new PostgreSQL connection pool
pub async fn create_pool(config: &Config) -> Result<DbPool> {
    let pool = PgPoolOptions::new()
        .max_connections(25)
        .min_connections(5)
        .idle_timeout(std::time::Duration::from_secs(30))
        .max_lifetime(std::time::Duration::from_secs(600))
        .acquire_timeout(std::time::Duration::from_secs(10))
        .connect(&config.database_url)
        .await?;

    info!("Connected to PostgreSQL database");
    Ok(pool)
}

// Transaction helper removed - use pool.begin() directly when needed
