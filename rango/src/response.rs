use axum::response::{Html, IntoResponse, Response};

#[cfg(features = "templates")]
use minijinja::Environment;

use crate::error::RangoError;

#[cfg(features = "templates")]
fn build_env() -> Environment<'static> {
    let mut env = Environment::new();
    env.set_loader(minijinja::path_loader("templates"));
    env
}

#[cfg(feature = "templates")]
pub fn render(template_name: &str, context: serde_json::Value) -> Result<Response, RangoError> {
    let env = build_env();

    let template = env
        .get_template(template_name)
        .map_err(|_| RangoError::TemplateNotFound(template_name.to_string()))?;
    let html = template
        .render(context)
        .map_err(|e| RangoError::RenderError(e.to_string()))?;

    Ok(Html(html).into_response())
}

pub fn json_response(data: serde_json::Value) -> Response {
    axum::response::Json(data).into_response()
}

pub fn json_response_with_status(
    status: axum::http::StatusCode,
    data: serde_json::Value,
) -> Response {
    (status, axum::response::Json(data)).into_response()
}

pub fn http_404(message: &str) -> RangoError {
    RangoError::NotFound(message.to_string())
}

pub fn redirect(url: &str) -> Response {
    use axum::http::{header, StatusCode};
    (StatusCode::FOUND, [(header::LOCATION, url.to_string())], "").into_response()
}

pub fn redirect_permanent(url: &str) -> Response {
    use axum::http::{header, StatusCode};
    (
        StatusCOde::MOVED_PERMANENTLY,
        [(header::LOCATION, url.to_string())],
        "",
    )
        .into_response()
}

pub fn text_response(content: &str) -> Response {
    Html(content.to_string()).into_response()
}
