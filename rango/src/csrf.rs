#[cfg(feature = "auth")]
use tower_sessions::Session;
#[cfg(feature = "auth")]
use rand::Rng;
#[cfg(feature = "auth")]
use crate::error::RangoError;

#[cfg(feature = "auth")]
const CSRF_KEY: &str = "rango_csrf_token";

#[cfg(feature = "auth")]
pub async fn get_csrf_token(session: &Session) -> Result<String, RangoError> {
    if let Some(token) = session.get::<String>(CSRF_KEY).await.map_err(|e| RangoError::Internal(e.to_string()))? {
        return Ok(token);
    }
    let token: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();
    session.insert(CSRF_KEY, &token).await.map_err(|e| RangoError::Internal(e.to_string()))?;
    Ok(token)
}

#[cfg(feature = "auth")]
pub async fn validate_csrf(session: &Session, token: &str) -> bool {
    if let Ok(Some(expected)) = session.get::<String>(CSRF_KEY).await {
        return expected == token;
    }
    false
}
