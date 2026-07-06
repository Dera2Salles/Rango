//! Rango CSRF Protection Module
//!
//! Provides CSRF token generation and validation using the Synchronizer Token Pattern.
//!
//! # Security
//! - Tokens are 32 bytes (256 bits) of OS-provided randomness.
//! - Comparison uses `subtle::ConstantTimeEq` to prevent timing attacks.
//! - Tokens are bound to the user's session.

#[cfg(feature = "auth")]
use tower_sessions::Session;
#[cfg(feature = "auth")]
use crate::error::RangoError;

#[cfg(feature = "auth")]
const CSRF_KEY: &str = "rango_csrf_token";

/// Get (or create) the CSRF token for this session.
///
/// The token is stored in the session and should be embedded in forms
/// as a hidden field or sent as `X-CSRF-Token` header for AJAX requests.
#[cfg(feature = "auth")]
pub async fn get_csrf_token(session: &Session) -> Result<String, RangoError> {
    if let Some(token) = session
        .get::<String>(CSRF_KEY)
        .await
        .map_err(|e| RangoError::Internal(e.to_string()))?
    {
        return Ok(token);
    }
    let token = generate_csrf_token();
    session
        .insert(CSRF_KEY, &token)
        .await
        .map_err(|e| RangoError::Internal(e.to_string()))?;
    Ok(token)
}

/// Regenerate the CSRF token. Call after login/logout to prevent CSRF token fixation.
#[cfg(feature = "auth")]
pub async fn regenerate_csrf_token(session: &Session) -> Result<String, RangoError> {
    let token = generate_csrf_token();
    session
        .insert(CSRF_KEY, &token)
        .await
        .map_err(|e| RangoError::Internal(e.to_string()))?;
    Ok(token)
}

/// Validate a CSRF token using constant-time comparison.
///
/// # Security
/// Uses `subtle::ConstantTimeEq` to prevent timing attacks that could
/// allow an attacker to brute-force the token one byte at a time.
#[cfg(feature = "auth")]
pub async fn validate_csrf(session: &Session, token: &str) -> bool {
    if let Ok(Some(expected)) = session.get::<String>(CSRF_KEY).await {
        // Constant-time comparison prevents timing attacks
        use subtle::ConstantTimeEq;
        let expected_bytes = expected.as_bytes();
        let token_bytes = token.as_bytes();
        // Length must match first (short-circuit on length difference is acceptable
        // since the attacker already knows the token length from the HTML source)
        if expected_bytes.len() == token_bytes.len() {
            return expected_bytes.ct_eq(token_bytes).into();
        }
    }
    false
}

/// Generate a cryptographically secure CSRF token.
#[cfg(feature = "auth")]
fn generate_csrf_token() -> String {
    use argon2::password_hash::rand_core::{OsRng, RngCore};
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}
