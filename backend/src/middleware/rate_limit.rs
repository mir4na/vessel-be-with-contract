use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpResponse,
};
use futures_util::future::{ok, LocalBoxFuture, Ready};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::utils::ApiResponse;

#[derive(Clone)]
#[allow(dead_code)] // Used for rate limiting configuration
pub struct RateLimitConfig {
    pub requests_per_window: u32,
    pub window_duration: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_window: 100,
            window_duration: Duration::from_secs(60),
        }
    }
}

struct RateLimitEntry {
    count: u32,
    window_start: Instant,
}

pub struct RateLimiter {
    config: RateLimitConfig,
    entries: Arc<RwLock<HashMap<String, RateLimitEntry>>>,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn check(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut entries = self.entries.write().await;

        let entry = entries.entry(key.to_string()).or_insert(RateLimitEntry {
            count: 0,
            window_start: now,
        });

        // Reset window if expired
        if now.duration_since(entry.window_start) >= self.config.window_duration {
            entry.count = 0;
            entry.window_start = now;
        }

        // Check limit
        if entry.count >= self.config.requests_per_window {
            return false;
        }

        entry.count += 1;
        true
    }

    pub async fn cleanup_old_entries(&self) {
        let now = Instant::now();
        let mut entries = self.entries.write().await;
        entries.retain(|_, entry| {
            now.duration_since(entry.window_start) < self.config.window_duration * 2
        });
    }
}

pub struct RateLimitMiddleware {
    limiter: Arc<RateLimiter>,
}

impl RateLimitMiddleware {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            limiter: Arc::new(RateLimiter::new(config)),
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for RateLimitMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = RateLimitMiddlewareService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(RateLimitMiddlewareService {
            service,
            limiter: self.limiter.clone(),
        })
    }
}

pub struct RateLimitMiddlewareService<S> {
    service: S,
    limiter: Arc<RateLimiter>,
}

impl<S, B> Service<ServiceRequest> for RateLimitMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // Get client IP for rate limiting
        let client_ip = req
            .connection_info()
            .realip_remote_addr()
            .unwrap_or("unknown")
            .to_string();

        let limiter = self.limiter.clone();
        let fut = self.service.call(req);

        Box::pin(async move {
            if !limiter.check(&client_ip).await {
                let (req, _) = fut.await?.into_parts();
                let response = HttpResponse::TooManyRequests().json(ApiResponse::<()>::error(
                    "Rate limit exceeded. Please try again later.",
                ));
                return Ok(ServiceResponse::new(req, response).map_into_right_body());
            }

            let res = fut.await?;
            Ok(res.map_into_left_body())
        })
    }
}

// IP-based rate limiter for specific endpoints (e.g., auth)
pub struct IpRateLimiter {
    limiter: Arc<RateLimiter>,
}

impl IpRateLimiter {
    pub fn new(requests_per_minute: u32) -> Self {
        Self {
            limiter: Arc::new(RateLimiter::new(RateLimitConfig {
                requests_per_window: requests_per_minute,
                window_duration: Duration::from_secs(60),
            })),
        }
    }

    pub async fn check(&self, ip: &str) -> bool {
        self.limiter.check(ip).await
    }
}

// Cleanup task for rate limiter
pub fn spawn_cleanup_task(limiter: Arc<RateLimiter>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(300)); // Every 5 minutes
        loop {
            interval.tick().await;
            limiter.cleanup_old_entries().await;
        }
    });
}
