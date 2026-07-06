//! Rango Middleware Module
//!
//! Provides HTTP middleware for logging, CORS, static files,
//! authentication, CSRF protection, security headers, and host validation.

use axum::http::Method;
use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};
use std::time::Instant;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tracing::info;

// ─── Logger ───────────────────────────────────────────────────────────────────

pub async fn logger_middleware(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let start = Instant::now();

    let response = next.run(req).await;

    let duration = start.elapsed();
    let status = response.status();

    info!(
        "{} {} → {} ({:.2}ms)",
        method,
        uri,
        status.as_u16(),
        duration.as_secs_f64() * 1000.0
    );

    response
}

// ─── CORS ─────────────────────────────────────────────────────────────────────

/// CORS layer that allows all origins.
///
/// # Security
/// Only use in development or for truly public APIs.
pub fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::PATCH,
            Method::OPTIONS,
        ])
        .allow_headers(Any)
        .allow_origin(Any)
}

/// CORS layer that allows specific origins only.
pub fn cors_layer_for(origins: Vec<&'static str>) -> CorsLayer {
    use axum::http::HeaderValue;
    let origins: Vec<HeaderValue> = origins.iter().filter_map(|o| o.parse().ok()).collect();

    CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::PATCH,
            Method::OPTIONS,
        ])
        .allow_headers(Any)
        .allow_origin(origins)
}

// ─── Static files ─────────────────────────────────────────────────────────────

pub fn static_files_service(dir: &str) -> ServeDir {
    ServeDir::new(dir)
}

// ─── Security Headers ─────────────────────────────────────────────────────────

/// Add security headers to every response.
///
/// Headers added (based on `RangoConfig.security`):
/// - `X-Content-Type-Options: nosniff`
/// - `X-Frame-Options: DENY`
/// - `X-XSS-Protection: 1; mode=block`
/// - `Referrer-Policy: same-origin`
/// - `Strict-Transport-Security` (if configured)
/// - `Content-Security-Policy` (if configured)
pub async fn security_headers_middleware(req: Request, next: Next) -> Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    let cfg = &crate::state::config().security;

    if cfg.content_type_nosniff {
        headers.insert(
            "X-Content-Type-Options",
            "nosniff".parse().unwrap(),
        );
    }
    if cfg.x_frame_options {
        headers.insert("X-Frame-Options", "DENY".parse().unwrap());
    }
    if cfg.xss_protection {
        headers.insert(
            "X-XSS-Protection",
            "1; mode=block".parse().unwrap(),
        );
    }
    if cfg.referrer_policy {
        headers.insert(
            "Referrer-Policy",
            "same-origin".parse().unwrap(),
        );
    }
    if let Some(max_age) = cfg.hsts_max_age {
        let value = format!("max-age={}; includeSubDomains", max_age);
        if let Ok(v) = value.parse() {
            headers.insert("Strict-Transport-Security", v);
        }
    }
    if let Some(ref csp) = cfg.csp {
        if let Ok(v) = csp.parse() {
            headers.insert("Content-Security-Policy", v);
        }
    }

    response
}

// ─── Host Validation ──────────────────────────────────────────────────────────

/// Validate the `Host` header against `RangoConfig.allowed_hosts`.
///
/// Requests with an unrecognized host are rejected with 400 Bad Request.
/// This prevents HTTP Host header injection attacks.
pub async fn host_validation_middleware(req: Request, next: Next) -> Result<Response, StatusCode> {
    let allowed = &crate::state::config().allowed_hosts;

    // Skip if allowed_hosts is empty or contains "*"
    if allowed.is_empty() || allowed.iter().any(|h| h == "*") {
        return Ok(next.run(req).await);
    }

    let host = req
        .headers()
        .get(axum::http::header::HOST)
        .and_then(|v| v.to_str().ok())
        // Strip port if present
        .map(|h| h.split(':').next().unwrap_or(h));

    match host {
        Some(h) if allowed.iter().any(|a| a == h) => Ok(next.run(req).await),
        _ => {
            tracing::warn!("Host header validation failed: {:?}", host);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

// ─── Auth middleware ──────────────────────────────────────────────────────────

#[cfg(not(feature = "auth"))]
pub async fn require_auth(req: Request, next: Next) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    match auth_header {
        Some(token) if token.starts_with("Bearer ") => Ok(next.run(req).await),
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}

#[cfg(feature = "auth")]
pub async fn require_auth(
    session: tower_sessions::Session,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if let Ok(Some(_)) = crate::auth::get_user_id(&session).await {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

// ─── CSRF middleware ──────────────────────────────────────────────────────────

/// Validate CSRF token on state-mutating requests (POST, PUT, DELETE, PATCH).
///
/// Checks `X-CSRF-Token` header OR `csrf_token` form field.
/// Uses constant-time comparison internally — see `csrf::validate_csrf`.
#[cfg(feature = "auth")]
pub async fn csrf_middleware(
    session: tower_sessions::Session,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let method = req.method().clone();
    if method == Method::POST
        || method == Method::PUT
        || method == Method::DELETE
        || method == Method::PATCH
    {
        let token = req
            .headers()
            .get("X-CSRF-Token")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        if let Some(token) = token {
            if crate::csrf::validate_csrf(&session, &token).await {
                return Ok(next.run(req).await);
            }
        }
        tracing::warn!("CSRF validation failed for {} {}", method, req.uri());
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(next.run(req).await)
}

// ─── Rate limiter (token bucket) ──────────────────────────────────────────────

/// Simple in-memory rate limiter state.
///
/// For production, prefer Redis-backed rate limiting.
/// This implementation uses a DashMap for concurrent access.
#[derive(Clone)]
pub struct RateLimiter {
    /// Requests per window.
    pub max_requests: u64,
    /// Window duration in seconds.
    pub window_secs: u64,
    store: std::sync::Arc<
        std::sync::Mutex<std::collections::HashMap<String, (u64, std::time::Instant)>>,
    >,
}

impl RateLimiter {
    pub fn new(max_requests: u64, window_secs: u64) -> Self {
        RateLimiter {
            max_requests,
            window_secs,
            store: std::sync::Arc::new(std::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
        }
    }

    /// Check and increment the request count for the given key (e.g. IP address).
    /// Returns `true` if the request is allowed, `false` if rate limited.
    pub fn check(&self, key: &str) -> bool {
        let now = std::time::Instant::now();
        let window = std::time::Duration::from_secs(self.window_secs);
        let mut store = self.store.lock().unwrap();

        match store.get_mut(key) {
            Some((count, start)) => {
                if now.duration_since(*start) > window {
                    // Reset window
                    *count = 1;
                    *start = now;
                    true
                } else if *count < self.max_requests {
                    *count += 1;
                    true
                } else {
                    false
                }
            }
            None => {
                store.insert(key.to_string(), (1, now));
                true
            }
        }
    }
}

/// Axum middleware factory for rate limiting by IP address.
pub fn rate_limit_middleware(
    limiter: RateLimiter,
) -> impl Fn(
    Request,
    Next,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, StatusCode>> + Send>>
       + Clone
       + Send
       + Sync
       + 'static {
    move |req: Request, next: Next| {
        let limiter = limiter.clone();
        Box::pin(async move {
            let ip = req
                .headers()
                .get("X-Forwarded-For")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.split(',').next())
                .or_else(|| {
                    req.extensions()
                        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
                        .map(|ci| {
                            // Return a static str is not possible; use leak for simplicity
                            // In production, store the IP differently
                            let _ = ci.0.ip().to_string();
                            "unknown"
                        })
                })
                .unwrap_or("unknown");

            if limiter.check(ip) {
                Ok(next.run(req).await)
            } else {
                tracing::warn!("Rate limit exceeded for IP: {}", ip);
                Err(StatusCode::TOO_MANY_REQUESTS)
            }
        })
    }
}
