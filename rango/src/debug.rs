use crate::state::config;
use axum::{
    http::{Request, StatusCode},
    middleware::Next,
    response::{Html, IntoResponse, Response},
};

const DEBUG_TEMPLATE: &str = include_str!("static/debug.html");

pub async fn debug_error_middleware(req: Request<axum::body::Body>, next: Next) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let headers = req.headers().clone();

    let response = next.run(req).await;

    if response.status().is_server_error() && config().debug {
        let message = response
            .extensions()
            .get::<crate::error::RangoError>()
            .map(|e| e.to_string())
            .unwrap_or_else(|| "An internal server error occurred.".to_string());

        return (
            response.status(),
            render_debug_page(
                response.status(),
                &message,
                &method.to_string(),
                &uri.to_string(),
                &format!("{:#?}", headers),
            ),
        )
            .into_response();
    }

    response
}

pub fn render_debug_page(
    status: StatusCode,
    message: &str,
    method: &str,
    uri: &str,
    headers: &str,
) -> Html<String> {
    let html = DEBUG_TEMPLATE
        .replace("{status}", &status.to_string())
        .replace("{status_code}", &status.as_u16().to_string())
        .replace("{message}", message)
        .replace("{method}", method)
        .replace("{uri}", uri)
        .replace("{headers}", headers)
        .replace("{os}", std::env::consts::OS)
        .replace("{arch}", std::env::consts::ARCH)
        .replace(
            "{cwd}",
            &std::env::current_dir()
                .unwrap_or_default()
                .display()
                .to_string(),
        )
        .replace("{rango_version}", env!("CARGO_PKG_VERSION"));

    Html(html)
}
