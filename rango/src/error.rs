use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use thiserror::Error;

#[derive(Debug, Error)]
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
                format!("Template introuvable : {}", t),
            ),
            RangoError::RenderError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Erreur de rendu : {}", e),
            ),
            RangoError::DatabaseError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Erreur DB : {}", e),
            ),
            RangoError::Internal(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.clone()),
        };
        let body = Html(format!(
            r#"<!DOCTYPE html>
<html>
<head><title>Rango — Erreur {code}</title>
<style>
  body {{ font-family: monospace; padding: 2rem; background: #1a1a2e; color: #eee; }}
  h1 {{ color: #e94560; }}
  pre {{ background: #16213e; padding: 1rem; border-radius: 6px; }}
</style>
</head>
<body>
  <h1>🤠 Rango Error {code}</h1>
  <pre>{msg}</pre>
</body>
</html>"#,
            code = status.as_u16(),
            msg = message
        ));

        (status, body).into_response()
    }
}

pub type RangoResult<T> = Result<T, RangoError>;

impl From<anyhow::Error> for RangoError {
    fn from(error: anyhow::Error) -> Self {
        RangoError::Internal(error.to_string())
    }
}
