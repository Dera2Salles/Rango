use axum::{
    http::StatusCode,
    response::{Html, IntoResponse},
};

pub const DEFAULT_404_HTML: &str = include_str!("static/404.html");

pub async fn default_404_handler() -> impl IntoResponse {
    #[cfg(feature = "templates")]
    {
        if let Ok(response) = crate::responses::render("404.html", serde_json::json!({})) {
            return (StatusCode::NOT_FOUND, response).into_response();
        }
    }
    (StatusCode::NOT_FOUND, Html(DEFAULT_404_HTML)).into_response()
}
