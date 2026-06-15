use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use thiserror::Error;

#[derive(Debug, Error, Clone)]
pub enum RangoError {
    #[error("Template not found : {0}")]
    TemplateNotFound(String),

    #[error("Template rendering error : {0}")]
    RenderError(String),

    #[error("Database error : {0}")]
    DatabaseError(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Not found : {0}")]
    NotFound(String),

    #[error("Internal error : {0}")]
    Internal(String),
}

impl IntoResponse for RangoError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            RangoError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            RangoError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            RangoError::TemplateNotFound(t) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Template not found: {}", t),
            ),
            RangoError::RenderError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Rendering error: {}", e),
            ),
            RangoError::DatabaseError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("DB Error: {}", e),
            ),
            RangoError::Internal(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.clone()),
        };

        let body = Html(format!(
            r#"<!DOCTYPE html>
<html>
<head><title>Rango — Error {code}</title>
<style>
  body {{ font-family: sans-serif; padding: 2rem; background: #0f172a; color: #f8fafc; text-align: center; display: flex; flex-direction: column; align-items: center; justify-content: center; height: 100vh; margin: 0; }}
  h1 {{ color: #e94560; font-size: 4rem; margin: 0; }}
  p {{ color: #94a3b8; font-size: 1.2rem; }}
  .container {{ max-width: 600px; }}
</style>
</head>
<body>
  <div class="container">
    <h1>🤠 Oops!</h1>
    <h2>Error {code}</h2>
    <p>{msg}</p>
    <a href="/" style="color: #e94560; text-decoration: none; font-weight: bold;">Back to Home</a>
  </div>
</body>
</html>"#,
            code = status.as_u16(),
            msg = if status.is_server_error() {
                "An internal server error occurred."
            } else {
                &message
            }
        ));

        let mut res = (status, body).into_response();
        res.extensions_mut().insert(self);
        res
    }
}

pub type RangoResult<T> = Result<T, RangoError>;

impl From<anyhow::Error> for RangoError {
    fn from(error: anyhow::Error) -> Self {
        RangoError::Internal(error.to_string())
    }
}
