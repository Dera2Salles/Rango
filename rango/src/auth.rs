//! Rango Auth Module
//!
//! Provides login/logout, session management, and password hashing.
//!
//! # Security
//! - Passwords are hashed with Argon2id (winner of the Password Hashing Competition).
//! - CSRF tokens use constant-time comparison to prevent timing attacks.
//! - Sessions use `tower_sessions` with configurable security settings.

#[cfg(feature = "auth")]
use tower_sessions::Session;
#[cfg(feature = "auth")]
use crate::error::RangoError;
#[cfg(feature = "auth")]
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2, Algorithm, Version, Params,
};

#[cfg(feature = "auth")]
const USER_ID_KEY: &str = "rango_user_id";

#[cfg(feature = "auth")]
const USER_DATA_KEY: &str = "rango_user_data";

/// Log in a user by storing their ID in the session.
#[cfg(feature = "auth")]
pub async fn login(session: &Session, user_id: i64) -> Result<(), RangoError> {
    // Rotate the session ID on login to prevent session fixation attacks.
    session.cycle_id().await.map_err(|e| RangoError::Internal(e.to_string()))?;
    session
        .insert(USER_ID_KEY, user_id)
        .await
        .map_err(|e| RangoError::Internal(e.to_string()))?;
    Ok(())
}

/// Log in a user and also store arbitrary user data in the session.
#[cfg(feature = "auth")]
pub async fn login_with_data<T: serde::Serialize>(
    session: &Session,
    user_id: i64,
    user_data: &T,
) -> Result<(), RangoError> {
    session.cycle_id().await.map_err(|e| RangoError::Internal(e.to_string()))?;
    session
        .insert(USER_ID_KEY, user_id)
        .await
        .map_err(|e| RangoError::Internal(e.to_string()))?;
    let data = serde_json::to_value(user_data)
        .map_err(|e| RangoError::Internal(e.to_string()))?;
    session
        .insert(USER_DATA_KEY, data)
        .await
        .map_err(|e| RangoError::Internal(e.to_string()))?;
    Ok(())
}

/// Log out the current user (destroys the session).
#[cfg(feature = "auth")]
pub async fn logout(session: &Session) -> Result<(), RangoError> {
    session
        .flush()
        .await
        .map_err(|e| RangoError::Internal(e.to_string()))?;
    Ok(())
}

/// Get the current user ID from the session.
#[cfg(feature = "auth")]
pub async fn get_user_id(session: &Session) -> Result<Option<i64>, RangoError> {
    session
        .get(USER_ID_KEY)
        .await
        .map_err(|e| RangoError::Internal(e.to_string()))
}

/// Get arbitrary user data stored in the session.
#[cfg(feature = "auth")]
pub async fn get_user_data(session: &Session) -> Result<Option<serde_json::Value>, RangoError> {
    session
        .get(USER_DATA_KEY)
        .await
        .map_err(|e| RangoError::Internal(e.to_string()))
}

/// Check if the current request is authenticated.
#[cfg(feature = "auth")]
pub async fn is_authenticated(session: &Session) -> bool {
    matches!(get_user_id(session).await, Ok(Some(_)))
}

/// Hash a password using Argon2id with secure default parameters.
///
/// # Security
/// Uses Argon2id variant with:
/// - Memory cost: 65536 KB (64 MB)
/// - Iterations: 2
/// - Parallelism: 1
///
/// These are OWASP-recommended parameters for interactive logins.
#[cfg(feature = "auth")]
pub fn hash_password(password: &str) -> Result<String, RangoError> {
    let salt = SaltString::generate(&mut OsRng);
    // OWASP recommended Argon2id parameters
    let params = Params::new(65536, 2, 1, None)
        .map_err(|e| RangoError::Internal(e.to_string()))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| RangoError::Internal(e.to_string()))?
        .to_string();
    Ok(password_hash)
}

/// Verify a password against a stored hash.
///
/// # Security
/// Uses constant-time comparison internally (via Argon2).
/// Returns `false` for any error (invalid hash format, wrong password, etc.)
#[cfg(feature = "auth")]
pub fn verify_password(password: &str, password_hash: &str) -> bool {
    let parsed_hash = match PasswordHash::new(password_hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

/// Generate a cryptographically secure random token.
#[cfg(feature = "auth")]
pub fn generate_token(length: usize) -> String {
    use argon2::password_hash::rand_core::RngCore;
    let mut bytes = vec![0u8; length];
    OsRng.fill_bytes(&mut bytes);
    hex::encode(&bytes[..length / 2])
}

/// Check if a password meets minimum security requirements.
/// Returns `Ok(())` if valid, `Err(message)` if not.
#[cfg(feature = "auth")]
pub fn validate_password_strength(password: &str) -> Result<(), String> {
    if password.len() < 8 {
        return Err("Password must be at least 8 characters long.".to_string());
    }
    if !password.chars().any(|c| c.is_ascii_uppercase()) {
        return Err("Password must contain at least one uppercase letter.".to_string());
    }
    if !password.chars().any(|c| c.is_ascii_lowercase()) {
        return Err("Password must contain at least one lowercase letter.".to_string());
    }
    if !password.chars().any(|c| c.is_ascii_digit()) {
        return Err("Password must contain at least one digit.".to_string());
    }
    Ok(())
}
