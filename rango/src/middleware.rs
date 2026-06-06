use axum::{
    middleware::Next,
    response::Response,
    extract::Request,
    http::StatusCode,

};
use axum::http::Method;
use std::time::Instant;
use tracing::info;
use tower_http::cors::{CorsLayer, Any};
use tower_http::services::ServeDir;


pub async fn logger_middleware(req : Request, next : Next)->Response{
    let method = req.methode().clone();
    let uri = req.uri().clone();
    let start = Instant::now();

    let response = next.run(req).await;

    let duration = start.elapsed();
    let status = response.status();

    info!("{} {} -> {} ({}:.2)ms", method, uri, status.as_u16(), duration.as_secs_f64() * 1000.0);
    response
}

pub fn cors_layer()-> CorsLayer{
    CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::PATCH])
        .allow_headers(Any)
        .allow_origin(Any)
}

pub fn cors_layer_for(origins : Vec<&' static str>)-> CorsLayer{
    use axum::http::HeaderValue;
    let origins: Vec<HeaderValue> = origins
        .iter()
        .filter_map(|o| o.parse().ok())
        .collet();
    CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers(Any)
        .allow_origin(origins)
}

pub fn static_files_service(dir : &str)-> ServeDir{
    ServeDir::new(dir)
}

pub async fn require_auth(req : Request, next : Next)-> Result<Response, StatusCode>{
    let auth_header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v|.to_str().ok());

    match auth_header{
        Some(token) if token.start_with("Bearer") => Ok(next.run(req).await),
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}
