use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

#[cfg(all(feature = "templates", not(debug_assertions)))]
use crate::error::RangoError;
use crate::RangoError;

// ─── Template engine (cached environment) ────────────────────────────────────

/// Cached MiniJinja environment for template rendering.
/// Re-created on each call in debug mode (live reload), cached in release.
#[cfg(all(feature = "templates", debug_assertions))]
fn build_env() -> minijinja::Environment<'static> {
    let mut env = minijinja::Environment::new();
    env.set_loader(minijinja::path_loader(
        &crate::state::config().templates_dir,
    ));
    // Enable useful built-ins
    env.set_undefined_behavior(minijinja::UndefinedBehavior::Chainable);
    crate::template_filters::register(&mut env);
    env
}

#[cfg(all(feature = "templates", not(debug_assertions)))]
static TEMPLATE_ENV: OnceLock<minijinja::Environment<'static>> = OnceLock::new();

#[cfg(all(feature = "templates", not(debug_assertions)))]
fn build_env() -> &'static minijinja::Environment<'static> {
    TEMPLATE_ENV.get_or_init(|| {
        let mut env = minijinja::Environment::new();
        minijinja::include_source_bundle!(env, "templates");
        env.set_undefined_behavior(minijinja::UndefinedBehavior::Chainable);
        crate::template_filters::register(&mut env);
        env
    })
}

/// Render a Jinja2/MiniJinja template with the given context.
///
/// In debug mode: templates are loaded from disk on every request (live reload).
/// In release mode: templates are cached in memory for performance.
#[cfg(feature = "templates")]
pub fn render(template_name: &str, context: serde_json::Value) -> Result<Response, RangoError> {
    use crate::RangoError;

    #[cfg(debug_assertions)]
    let env = build_env();
    #[cfg(not(debug_assertions))]
    let env = build_env();

    let tmpl = env
        .get_template(template_name)
        .map_err(|e| RangoError::TemplateNotFound(format!("{}: {}", template_name, e)))?;

    let html = tmpl
        .render(context)
        .map_err(|e| RangoError::RenderError(e.to_string()))?;

    Ok(Html(html).into_response())
}

/// Render a template and return a pre-built response (panics on error in debug, logs in release).
#[cfg(feature = "templates")]
pub fn render_or_500(template_name: &str, context: serde_json::Value) -> Response {
    match render(template_name, context) {
        Ok(r) => r,
        Err(e) => e.into_response(),
    }
}

// ─── JSON responses ───────────────────────────────────────────────────────────

/// Respond with a JSON body (200 OK).
pub fn json_response(data: serde_json::Value) -> Response {
    axum::response::Json(data).into_response()
}

/// Respond with a JSON body and a custom status code.
pub fn json_response_with_status(status: StatusCode, data: serde_json::Value) -> Response {
    (status, axum::response::Json(data)).into_response()
}

/// Respond with 201 Created and a JSON body.
pub fn created(data: serde_json::Value) -> Response {
    json_response_with_status(StatusCode::CREATED, data)
}

/// Respond with 204 No Content (empty body).
pub fn no_content() -> Response {
    StatusCode::NO_CONTENT.into_response()
}

/// Respond with 400 Bad Request and a JSON error message.
pub fn bad_request(message: &str) -> Response {
    json_response_with_status(
        StatusCode::BAD_REQUEST,
        serde_json::json!({ "error": message }),
    )
}

/// Respond with 401 Unauthorized and a JSON error message.
pub fn unauthorized(message: &str) -> Response {
    json_response_with_status(
        StatusCode::UNAUTHORIZED,
        serde_json::json!({ "error": message }),
    )
}

/// Respond with 403 Forbidden.
pub fn forbidden(message: &str) -> Response {
    json_response_with_status(
        StatusCode::FORBIDDEN,
        serde_json::json!({ "error": message }),
    )
}

/// Respond with 404 Not Found.
pub fn not_found(message: &str) -> Response {
    json_response_with_status(
        StatusCode::NOT_FOUND,
        serde_json::json!({ "error": message }),
    )
}

/// Respond with 429 Too Many Requests.
pub fn rate_limited() -> Response {
    json_response_with_status(
        StatusCode::TOO_MANY_REQUESTS,
        serde_json::json!({ "error": "Too many requests. Please slow down." }),
    )
}

// ─── Error helpers ────────────────────────────────────────────────────────────

/// Create a 404 Not Found RangoError.
pub fn http_404(message: &str) -> RangoError {
    RangoError::NotFound(message.to_string())
}

// ─── Redirects ────────────────────────────────────────────────────────────────

/// Redirect to a URL (302 Found).
pub fn redirect(url: &str) -> Response {
    use axum::http::{header, StatusCode};
    (StatusCode::FOUND, [(header::LOCATION, url.to_string())], "").into_response()
}

/// Redirect to a URL permanently (301 Moved Permanently).
pub fn redirect_permanent(url: &str) -> Response {
    use axum::http::{header, StatusCode};
    (
        StatusCode::MOVED_PERMANENTLY,
        [(header::LOCATION, url.to_string())],
        "",
    )
        .into_response()
}

/// Redirect after POST (303 See Other — proper PRG pattern).
pub fn redirect_see_other(url: &str) -> Response {
    use axum::http::{header, StatusCode};
    (
        StatusCode::SEE_OTHER,
        [(header::LOCATION, url.to_string())],
        "",
    )
        .into_response()
}

// ─── Text / HTML responses ────────────────────────────────────────────────────

/// Respond with plain text HTML content (200 OK).
pub fn text_response(content: &str) -> Response {
    Html(content.to_string()).into_response()
}

/// Respond with plain text (text/plain, 200 OK).
pub fn plain_text(content: &str) -> Response {
    (
        StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; charset=utf-8",
        )],
        content.to_string(),
    )
        .into_response()
}
