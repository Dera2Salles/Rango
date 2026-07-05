#[cfg(feature = "auth")]
use tower_sessions::Session;
#[cfg(feature = "auth")]
use crate::error::RangoError;
#[cfg(feature = "auth")]
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

#[cfg(feature = "auth")]
const USER_ID_KEY: &str = "rango_user_id";

#[cfg(feature = "auth")]
pub async fn login(session: &Session, user_id: i64) -> Result<(), RangoError> {
    session.insert(USER_ID_KEY, user_id).await.map_err(|e| RangoError::Internal(e.to_string()))?;
    Ok(())
}

#[cfg(feature = "auth")]
pub async fn logout(session: &Session) -> Result<(), RangoError> {
    session.remove_value(USER_ID_KEY).await.map_err(|e| RangoError::Internal(e.to_string()))?;
    Ok(())
}

#[cfg(feature = "auth")]
pub async fn get_user_id(session: &Session) -> Result<Option<i64>, RangoError> {
    session.get(USER_ID_KEY).await.map_err(|e| RangoError::Internal(e.to_string()))
}

#[cfg(feature = "auth")]
pub fn hash_password(password: &str) -> Result<String, RangoError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| RangoError::Internal(e.to_string()))?
        .to_string();
    Ok(password_hash)
}

#[cfg(feature = "auth")]
pub fn verify_password(password: &str, password_hash: &str) -> bool {
    let parsed_hash = match PasswordHash::new(password_hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default().verify_password(password.as_bytes(), &parsed_hash).is_ok()
}
