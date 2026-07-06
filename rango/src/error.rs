//! Rango Error Types
//!
//! Centralized error handling for the framework. All framework operations
//! return `RangoResult<T>` which is `Result<T, RangoError>`.
//!
//! # Security
//! In production mode (`debug = false`), server error details are NOT leaked
//! to HTTP responses — only a generic message is shown.

use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use thiserror::Error;

#[derive(Debug, Error, Clone)]
pub enum RangoError {
    #[error("Template not found: {0}")]
    TemplateNotFound(String),

    #[error("Template rendering error: {0}")]
    RenderError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Database not initialized: {0}")]
    DatabaseNotInitialized(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Forbidden")]
    Forbidden,

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Rate limit exceeded")]
    RateLimited,

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),
}

impl RangoError {
    /// HTTP status code for this error.
    pub fn status_code(&self) -> StatusCode {
        match self {
            RangoError::Unauthorized => StatusCode::UNAUTHORIZED,
            RangoError::Forbidden => StatusCode::FORBIDDEN,
            RangoError::NotFound(_) => StatusCode::NOT_FOUND,
            RangoError::BadRequest(_) | RangoError::ValidationError(_) => StatusCode::BAD_REQUEST,
            RangoError::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            RangoError::ServiceUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Whether this error is a client error (4xx).
    pub fn is_client_error(&self) -> bool {
        self.status_code().is_client_error()
    }

    /// Whether this error is a server error (5xx).
    pub fn is_server_error(&self) -> bool {
        self.status_code().is_server_error()
    }

    /// Convert to a JSON error response.
    pub fn to_json_response(&self) -> Response {
        let status = self.status_code();
        let debug = crate::state::config().debug;

        let message = if status.is_server_error() && !debug {
            "An internal server error occurred.".to_string()
        } else {
            self.to_string()
        };

        let body = serde_json::json!({
            "error": true,
            "status": status.as_u16(),
            "message": message,
        });

        (status, axum::response::Json(body)).into_response()
    }
}

impl IntoResponse for RangoError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let debug = crate::state::config().debug;

        // In production, don't expose server error details
        let user_message = if status.is_server_error() && !debug {
            "An internal server error occurred.".to_string()
        } else {
            match &self {
                RangoError::NotFound(msg) => msg.clone(),
                RangoError::BadRequest(msg) => msg.clone(),
                RangoError::ValidationError(msg) => format!("Validation error: {}", msg),
                RangoError::Unauthorized => "Authentication required.".to_string(),
                RangoError::Forbidden => "You do not have permission to access this resource.".to_string(),
                RangoError::RateLimited => "Too many requests. Please slow down.".to_string(),
                _ => self.to_string(),
            }
        };

        // Log server errors
        if status.is_server_error() {
            tracing::error!("Server error [{}]: {}", status.as_u16(), self);
        } else if status.is_client_error() {
            tracing::warn!("Client error [{}]: {}", status.as_u16(), self);
        }

        let body = Html(format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Rango — Error {code}</title>
  <style>
    @import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;600;700&display=swap');
    *, *::before, *::after {{ box-sizing: border-box; }}
    body {{
      font-family: 'Inter', sans-serif;
      padding: 2rem;
      background: #0f172a;
      color: #f8fafc;
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      min-height: 100vh;
      margin: 0;
    }}
    .container {{ max-width: 600px; text-align: center; }}
    h1 {{ color: #e94560; font-size: 5rem; margin: 0; line-height: 1; }}
    h2 {{ color: #94a3b8; font-size: 1.5rem; font-weight: 600; margin: 0.5rem 0; }}
    p {{ color: #64748b; font-size: 1rem; line-height: 1.6; margin: 1rem 0; }}
    .badge {{
      display: inline-block;
      background: #1e293b;
      border: 1px solid #334155;
      border-radius: 6px;
      padding: 0.25rem 0.75rem;
      font-size: 0.8rem;
      color: #94a3b8;
      margin-bottom: 1.5rem;
    }}
    a {{
      display: inline-block;
      color: #f8fafc;
      background: #e94560;
      text-decoration: none;
      font-weight: 600;
      padding: 0.6rem 1.5rem;
      border-radius: 8px;
      margin-top: 1.5rem;
      transition: opacity 0.2s;
    }}
    a:hover {{ opacity: 0.85; }}
  </style>
</head>
<body>
  <div class="container">
    <h1>{code}</h1>
    <div class="badge">{title}</div>
    <h2>An error occurred</h2>
    <p>{msg}</p>
    <a href="/">← Back to Home</a>
  </div>
</body>
</html>"#,
            code = status.as_u16(),
            title = status.canonical_reason().unwrap_or("Error"),
            msg = html_escape(&user_message),
        ));

        let mut res = (status, body).into_response();
        // Store the error in extensions for the debug middleware
        res.extensions_mut().insert(self);
        res
    }
}

/// Escape HTML special characters to prevent XSS in error messages.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

pub type RangoResult<T> = Result<T, RangoError>;

impl From<anyhow::Error> for RangoError {
    fn from(error: anyhow::Error) -> Self {
        RangoError::Internal(error.to_string())
    }
}

#[cfg(feature = "db")]
impl From<sqlx::Error> for RangoError {
    fn from(error: sqlx::Error) -> Self {
        match &error {
            sqlx::Error::RowNotFound => RangoError::NotFound("Record not found".to_string()),
            _ => RangoError::DatabaseError(error.to_string()),
        }
    }
}

impl From<std::io::Error> for RangoError {
    fn from(error: std::io::Error) -> Self {
        RangoError::Internal(error.to_string())
    }
}
