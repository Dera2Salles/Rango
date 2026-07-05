use axum::http::Method;
use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};
use std::time::Instant;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tracing::info;

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

pub fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::PATCH,
        ])
        .allow_headers(Any)
        .allow_origin(Any)
}

pub fn cors_layer_for(origins: Vec<&'static str>) -> CorsLayer {
    use axum::http::HeaderValue;
    let origins: Vec<HeaderValue> = origins.iter().filter_map(|o| o.parse().ok()).collect();

    CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers(Any)
        .allow_origin(origins)
}

pub fn static_files_service(dir: &str) -> ServeDir {
    ServeDir::new(dir)
}

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

#[cfg(feature = "auth")]
pub async fn csrf_middleware(
    session: tower_sessions::Session,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let method = req.method();
    if method == Method::POST || method == Method::PUT || method == Method::DELETE {
        let token = req.headers().get("X-CSRF-Token").and_then(|v| v.to_str().ok());
        if let Some(token) = token {
            if crate::csrf::validate_csrf(&session, token).await {
                return Ok(next.run(req).await);
            }
        }
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(next.run(req).await)
}
