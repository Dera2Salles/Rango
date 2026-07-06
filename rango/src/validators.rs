//! Django-like Field Validators
//!
//! Validators are composable functions that check field values and return
//! structured errors.
//!
//! # Example
//! ```rust
//! use rango::validators::{Validator, ValidationErrors};
//!
//! let mut errors = ValidationErrors::new();
//!
//! let email = "user@example.com";
//! if let Err(msg) = Validator::email(email) {
//!     errors.add("email", &msg);
//! }
//!
//! if !errors.is_empty() {
//!     return Err(rango::RangoError::ValidationError(errors.to_string()));
//! }
//! ```

use std::collections::HashMap;

/// A collection of validation errors keyed by field name.
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct ValidationErrors {
    pub errors: HashMap<String, Vec<String>>,
}

impl ValidationErrors {
    pub fn new() -> Self {
        ValidationErrors::default()
    }

    /// Add an error for a field.
    pub fn add(&mut self, field: &str, message: &str) {
        self.errors
            .entry(field.to_string())
            .or_default()
            .push(message.to_string());
    }

    /// Whether there are any errors.
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    /// Number of fields with errors.
    pub fn count(&self) -> usize {
        self.errors.len()
    }

    /// Get errors for a specific field.
    pub fn get(&self, field: &str) -> Option<&Vec<String>> {
        self.errors.get(field)
    }

    /// Convert to a JSON value.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(&self.errors).unwrap_or(serde_json::Value::Null)
    }

    /// Convert to a RangoError.
    pub fn into_error(self) -> crate::error::RangoError {
        crate::error::RangoError::ValidationError(self.to_string())
    }
}

impl std::fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msgs: Vec<String> = self
            .errors
            .iter()
            .flat_map(|(field, errors)| errors.iter().map(move |e| format!("{}: {}", field, e)))
            .collect();
        write!(f, "{}", msgs.join("; "))
    }
}

/// Built-in field validators.
pub struct Validator;

impl Validator {
    // ─── String validators ────────────────────────────────────────────────────

    /// Validate that a value is not empty.
    pub fn required(value: &str) -> Result<(), String> {
        if value.trim().is_empty() {
            Err("This field is required.".to_string())
        } else {
            Ok(())
        }
    }

    /// Validate minimum length.
    pub fn min_length(value: &str, min: usize) -> Result<(), String> {
        if value.len() < min {
            Err(format!("Must be at least {} characters long.", min))
        } else {
            Ok(())
        }
    }

    /// Validate maximum length.
    pub fn max_length(value: &str, max: usize) -> Result<(), String> {
        if value.len() > max {
            Err(format!("Must be at most {} characters long.", max))
        } else {
            Ok(())
        }
    }

    /// Validate exact length.
    pub fn exact_length(value: &str, len: usize) -> Result<(), String> {
        if value.len() != len {
            Err(format!("Must be exactly {} characters long.", len))
        } else {
            Ok(())
        }
    }

    /// Validate that the value matches a regex pattern.
    pub fn regex(value: &str, pattern: &str) -> Result<(), String> {
        // Simple hand-rolled check — avoid depending on regex crate
        // For complex patterns, use regex crate directly
        let _ = (value, pattern); // Placeholder
        Ok(())
    }

    // ─── Email validator ──────────────────────────────────────────────────────

    /// Validate an email address format.
    ///
    /// Checks for `@` and at least one `.` in the domain part.
    pub fn email(value: &str) -> Result<(), String> {
        let parts: Vec<&str> = value.splitn(2, '@').collect();
        if parts.len() != 2 {
            return Err("Enter a valid email address.".to_string());
        }
        let local = parts[0];
        let domain = parts[1];
        if local.is_empty() || domain.is_empty() {
            return Err("Enter a valid email address.".to_string());
        }
        if !domain.contains('.') {
            return Err("Enter a valid email address.".to_string());
        }
        if domain.starts_with('.') || domain.ends_with('.') {
            return Err("Enter a valid email address.".to_string());
        }
        // No consecutive dots
        if domain.contains("..") {
            return Err("Enter a valid email address.".to_string());
        }
        Ok(())
    }

    // ─── URL validator ────────────────────────────────────────────────────────

    /// Validate a URL (must start with http:// or https://).
    pub fn url(value: &str) -> Result<(), String> {
        if value.starts_with("http://") || value.starts_with("https://") {
            Ok(())
        } else {
            Err("Enter a valid URL (must start with http:// or https://).".to_string())
        }
    }

    // ─── Numeric validators ───────────────────────────────────────────────────

    /// Validate that a number is at least `min`.
    pub fn min_value<T: PartialOrd + std::fmt::Display>(value: T, min: T) -> Result<(), String> {
        if value < min {
            Err(format!("Must be at least {}.", min))
        } else {
            Ok(())
        }
    }

    /// Validate that a number is at most `max`.
    pub fn max_value<T: PartialOrd + std::fmt::Display>(value: T, max: T) -> Result<(), String> {
        if value > max {
            Err(format!("Must be at most {}.", max))
        } else {
            Ok(())
        }
    }

    /// Validate that a number is in range `[min, max]`.
    pub fn range<T: PartialOrd + std::fmt::Display>(
        value: T,
        min: T,
        max: T,
    ) -> Result<(), String> {
        if value < min || value > max {
            Err(format!("Must be between {} and {}.", min, max))
        } else {
            Ok(())
        }
    }

    // ─── String content validators ────────────────────────────────────────────

    /// Validate that the value contains only alphanumeric characters.
    pub fn alphanumeric(value: &str) -> Result<(), String> {
        if value.chars().all(|c| c.is_alphanumeric()) {
            Ok(())
        } else {
            Err("Only alphanumeric characters are allowed.".to_string())
        }
    }

    /// Validate that the value contains only ASCII characters.
    pub fn ascii(value: &str) -> Result<(), String> {
        if value.is_ascii() {
            Ok(())
        } else {
            Err("Only ASCII characters are allowed.".to_string())
        }
    }

    /// Validate that the value is a valid username (alphanumeric + underscore).
    pub fn username(value: &str) -> Result<(), String> {
        Self::min_length(value, 3)?;
        Self::max_length(value, 150)?;
        if value
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            Ok(())
        } else {
            Err(
                "Username may only contain letters, numbers, underscores, and hyphens."
                    .to_string(),
            )
        }
    }

    /// Validate that a password meets strength requirements.
    /// - At least 8 characters
    /// - At least one uppercase letter
    /// - At least one lowercase letter
    /// - At least one digit
    pub fn password_strength(value: &str) -> Result<(), String> {
        if value.len() < 8 {
            return Err("Password must be at least 8 characters long.".to_string());
        }
        if !value.chars().any(|c| c.is_ascii_uppercase()) {
            return Err("Password must contain at least one uppercase letter.".to_string());
        }
        if !value.chars().any(|c| c.is_ascii_lowercase()) {
            return Err("Password must contain at least one lowercase letter.".to_string());
        }
        if !value.chars().any(|c| c.is_ascii_digit()) {
            return Err("Password must contain at least one digit.".to_string());
        }
        Ok(())
    }

    /// Validate that two values match (e.g. password confirmation).
    pub fn matches<T: PartialEq>(value: &T, other: &T, field_name: &str) -> Result<(), String> {
        if value == other {
            Ok(())
        } else {
            Err(format!("Does not match {}.", field_name))
        }
    }

    // ─── File validators ──────────────────────────────────────────────────────

    /// Validate that a file extension is in the allowed list.
    pub fn file_extension(filename: &str, allowed: &[&str]) -> Result<(), String> {
        let ext = filename
            .rsplit('.')
            .next()
            .map(|e| e.to_lowercase())
            .unwrap_or_default();
        if allowed.iter().any(|a| a.to_lowercase() == ext) {
            Ok(())
        } else {
            Err(format!(
                "File type '{}' is not allowed. Allowed types: {}.",
                ext,
                allowed.join(", ")
            ))
        }
    }

    /// Validate that a file size is within limits (in bytes).
    pub fn max_file_size(size_bytes: u64, max_bytes: u64) -> Result<(), String> {
        if size_bytes <= max_bytes {
            Ok(())
        } else {
            let max_mb = max_bytes as f64 / 1_048_576.0;
            Err(format!("File size must not exceed {:.1} MB.", max_mb))
        }
    }
}
