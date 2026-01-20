use deadpool_redis::{Config as RedisConfig, Pool, Runtime};
use tracing::info;
use anyhow::Result;

use crate::config::Config;

pub type RedisPool = Pool;

/// Create a new Redis connection pool
pub async fn create_redis_pool(config: &Config) -> Result<RedisPool> {
    let redis_url = if config.redis_password.is_empty() {
        format!(
            "redis://{}:{}/{}",
            config.redis_host, config.redis_port, config.redis_db
        )
    } else {
        format!(
            "redis://:{}@{}:{}/{}",
            config.redis_password, config.redis_host, config.redis_port, config.redis_db
        )
    };

    let redis_config = RedisConfig::from_url(redis_url);
    let pool = redis_config.create_pool(Some(Runtime::Tokio1))?;

    // Test connection
    let mut conn = pool.get().await?;
    redis::cmd("PING")
        .query_async::<_, String>(&mut conn)
        .await?;

    info!("Connected to Redis");
    Ok(pool)
}
