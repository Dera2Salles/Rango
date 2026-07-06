//! Django-like Forms
//!
//! Provides a simple form validation system similar to Django's `forms.Form`.
//!
//! # Example
//! ```rust
//! use rango::forms::Form;
//! use rango::validators::Validator;
//! use std::collections::HashMap;
//!
//! let data: HashMap<String, String> = /* from request */ HashMap::new();
//! let mut form = Form::new(data);
//!
//! let username = form.field("username")
//!     .required()
//!     .min_length(3)
//!     .max_length(150)
//!     .validate_username()
//!     .get();
//!
//! let email = form.field("email")
//!     .required()
//!     .validate_email()
//!     .get();
//!
//! if form.is_valid() {
//!     println!("username={:?}, email={:?}", username, email);
//! } else {
//!     return Err(form.errors().into_error());
//! }
//! ```

use std::collections::HashMap;
use crate::validators::{ValidationErrors, Validator};

/// A Django-like form for validating and cleaning request data.
pub struct Form {
    data: HashMap<String, String>,
    errors: ValidationErrors,
}

impl Form {
    /// Create a new form from raw string data (e.g. from a POST body).
    pub fn new(data: HashMap<String, String>) -> Self {
        Form {
            data,
            errors: ValidationErrors::new(),
        }
    }

    /// Create a form from JSON data.
    pub fn from_json(value: &serde_json::Value) -> Self {
        let mut data = HashMap::new();
        if let serde_json::Value::Object(map) = value {
            for (k, v) in map {
                let s = match v {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    _ => v.to_string(),
                };
                data.insert(k.clone(), s);
            }
        }
        Form {
            data,
            errors: ValidationErrors::new(),
        }
    }

    /// Get a field builder for the given field name.
    pub fn field<'a>(&'a mut self, name: &'a str) -> FieldBuilder<'a> {
        let value = self.data.get(name).cloned().unwrap_or_default();
        FieldBuilder {
            form: self,
            name,
            value,
        }
    }

    /// Get the raw value of a field.
    pub fn get(&self, field: &str) -> Option<&str> {
        self.data.get(field).map(|s| s.as_str())
    }

    /// Get the raw value or an empty string.
    pub fn get_or_empty(&self, field: &str) -> &str {
        self.data.get(field).map(|s| s.as_str()).unwrap_or("")
    }

    /// Add an error manually.
    pub fn add_error(&mut self, field: &str, message: &str) {
        self.errors.add(field, message);
    }

    /// Whether the form has no validation errors.
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    /// Get all validation errors.
    pub fn errors(&self) -> &ValidationErrors {
        &self.errors
    }

    /// Get errors as JSON.
    pub fn errors_json(&self) -> serde_json::Value {
        self.errors.to_json()
    }

    /// Get the underlying data map.
    pub fn data(&self) -> &HashMap<String, String> {
        &self.data
    }
}

/// A fluent builder for validating a single field.
pub struct FieldBuilder<'a> {
    form: &'a mut Form,
    name: &'a str,
    value: String,
}

impl<'a> FieldBuilder<'a> {
    /// Mark the field as required (non-empty).
    pub fn required(self) -> Self {
        if let Err(msg) = Validator::required(&self.value) {
            self.form.errors.add(self.name, &msg);
        }
        self
    }

    /// Validate minimum length.
    pub fn min_length(self, min: usize) -> Self {
        if let Err(msg) = Validator::min_length(&self.value, min) {
            self.form.errors.add(self.name, &msg);
        }
        self
    }

    /// Validate maximum length.
    pub fn max_length(self, max: usize) -> Self {
        if let Err(msg) = Validator::max_length(&self.value, max) {
            self.form.errors.add(self.name, &msg);
        }
        self
    }

    /// Validate as email address.
    pub fn validate_email(self) -> Self {
        if !self.value.is_empty() {
            if let Err(msg) = Validator::email(&self.value) {
                self.form.errors.add(self.name, &msg);
            }
        }
        self
    }

    /// Validate as URL.
    pub fn validate_url(self) -> Self {
        if !self.value.is_empty() {
            if let Err(msg) = Validator::url(&self.value) {
                self.form.errors.add(self.name, &msg);
            }
        }
        self
    }

    /// Validate as username (alphanumeric + underscores/hyphens, 3-150 chars).
    pub fn validate_username(self) -> Self {
        if !self.value.is_empty() {
            if let Err(msg) = Validator::username(&self.value) {
                self.form.errors.add(self.name, &msg);
            }
        }
        self
    }

    /// Validate password strength.
    pub fn validate_password(self) -> Self {
        if !self.value.is_empty() {
            if let Err(msg) = Validator::password_strength(&self.value) {
                self.form.errors.add(self.name, &msg);
            }
        }
        self
    }

    /// Validate that this field matches another field's value.
    pub fn matches_field(self, other_field: &str) -> Self {
        let other = self.form.data.get(other_field).cloned().unwrap_or_default();
        if let Err(msg) = Validator::matches(&self.value, &other, other_field) {
            self.form.errors.add(self.name, &msg);
        }
        self
    }

    /// Apply a custom validation function.
    pub fn custom<F: Fn(&str) -> Result<(), String>>(self, f: F) -> Self {
        if let Err(msg) = f(&self.value) {
            self.form.errors.add(self.name, &msg);
        }
        self
    }

    /// Get the field's value.
    pub fn get(self) -> Option<String> {
        if self.value.is_empty() {
            None
        } else {
            Some(self.value)
        }
    }

    /// Get the field's value or a default.
    pub fn get_or(self, default: &str) -> String {
        if self.value.is_empty() {
            default.to_string()
        } else {
            self.value
        }
    }

    /// Parse the field value as a specific type.
    pub fn parse<T: std::str::FromStr>(self) -> Option<T> {
        self.value.parse().ok()
    }
}
