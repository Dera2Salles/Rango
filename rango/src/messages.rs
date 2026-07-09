//! Django-like Messages Framework
//!
//! Provides one-shot "flash" messages that survive a redirect and are
//! displayed exactly once — mirroring `django.contrib.messages`.
//!
//! Messages are stored in the user's session, so this module requires the
//! `auth` feature (which pulls in `tower-sessions`).
//!
//! # Example
//! ```rust,ignore
//! use rango::messages;
//!
//! #[view(method = "POST")]
//! pub async fn create_post(session: tower_sessions::Session, /* ... */) {
//!     // ... save the post ...
//!     messages::success(&session, "Post created successfully!").await.ok();
//!     rango::redirect("/posts")
//! }
//! ```
//!
//! In your base template, render messages once:
//! ```jinja
//! {% for message in messages %}
//!   <div class="alert alert-{{ message.level }}">{{ message.text }}</div>
//! {% endfor %}
//! ```
//! (pass `messages: messages::get_messages(&session).await?` into your template context.)

#[cfg(feature = "auth")]
use tower_sessions::Session;
#[cfg(feature = "auth")]
use crate::error::RangoError;

#[cfg(feature = "auth")]
const MESSAGES_KEY: &str = "rango_messages";

/// Severity level of a flash message — mirrors Django's message levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageLevel {
    Debug,
    Info,
    Success,
    Warning,
    Error,
}

impl std::fmt::Display for MessageLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            MessageLevel::Debug => "debug",
            MessageLevel::Info => "info",
            MessageLevel::Success => "success",
            MessageLevel::Warning => "warning",
            MessageLevel::Error => "error",
        };
        write!(f, "{}", s)
    }
}

/// A single flash message.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Message {
    pub level: MessageLevel,
    pub text: String,
}

impl Message {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

/// Queue a message with an arbitrary level.
#[cfg(feature = "auth")]
pub async fn add_message(session: &Session, level: MessageLevel, text: &str) -> Result<(), RangoError> {
    let mut current: Vec<Message> = session
        .get(MESSAGES_KEY)
        .await
        .map_err(|e| RangoError::Internal(e.to_string()))?
        .unwrap_or_default();
    current.push(Message {
        level,
        text: text.to_string(),
    });
    session
        .insert(MESSAGES_KEY, current)
        .await
        .map_err(|e| RangoError::Internal(e.to_string()))?;
    Ok(())
}

/// Queue a debug-level message.
#[cfg(feature = "auth")]
pub async fn debug(session: &Session, text: &str) -> Result<(), RangoError> {
    add_message(session, MessageLevel::Debug, text).await
}

/// Queue an info-level message.
#[cfg(feature = "auth")]
pub async fn info(session: &Session, text: &str) -> Result<(), RangoError> {
    add_message(session, MessageLevel::Info, text).await
}

/// Queue a success-level message.
#[cfg(feature = "auth")]
pub async fn success(session: &Session, text: &str) -> Result<(), RangoError> {
    add_message(session, MessageLevel::Success, text).await
}

/// Queue a warning-level message.
#[cfg(feature = "auth")]
pub async fn warning(session: &Session, text: &str) -> Result<(), RangoError> {
    add_message(session, MessageLevel::Warning, text).await
}

/// Queue an error-level message.
#[cfg(feature = "auth")]
pub async fn error(session: &Session, text: &str) -> Result<(), RangoError> {
    add_message(session, MessageLevel::Error, text).await
}

/// Retrieve and clear all pending messages for this session (one-shot "flash" semantics).
/// Call this once per request, typically right before rendering your template.
#[cfg(feature = "auth")]
pub async fn get_messages(session: &Session) -> Result<Vec<Message>, RangoError> {
    let current: Vec<Message> = session
        .get(MESSAGES_KEY)
        .await
        .map_err(|e| RangoError::Internal(e.to_string()))?
        .unwrap_or_default();
    if !current.is_empty() {
        session
            .remove::<Vec<Message>>(MESSAGES_KEY)
            .await
            .map_err(|e| RangoError::Internal(e.to_string()))?;
    }
    Ok(current)
}

/// Retrieve pending messages as a `serde_json::Value` array, ready to drop
/// straight into a template context (e.g. `context! { messages => messages_json }`).
#[cfg(feature = "auth")]
pub async fn get_messages_json(session: &Session) -> Result<serde_json::Value, RangoError> {
    let messages = get_messages(session).await?;
    Ok(serde_json::Value::Array(
        messages.iter().map(|m| m.to_json()).collect(),
    ))
}
