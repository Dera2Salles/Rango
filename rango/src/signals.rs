//! Django-like Signals System
//!
//! Signals allow decoupled components to react to events without tight coupling.
//!
//! # Example
//! ```rust
//! use rango::signals::Signal;
//!
//! static USER_SAVED: Signal<User> = Signal::new();
//!
//! // Register a listener
//! USER_SAVED.connect(|user| {
//!     println!("User saved: {}", user.name);
//! });
//!
//! // Emit the signal
//! USER_SAVED.send(&my_user);
//! ```

use std::sync::{Arc, RwLock};

/// A typed signal that can be connected to multiple listeners.
///
/// Signals are Send + Sync and can be used as statics.
pub struct Signal<T: Send + Sync + 'static> {
    handlers: RwLock<Vec<Arc<dyn Fn(&T) + Send + Sync>>>,
}

impl<T: Send + Sync + 'static> Signal<T> {
    /// Create a new signal with no listeners.
    pub const fn new() -> Self {
        Signal {
            handlers: RwLock::new(Vec::new()),
        }
    }

    /// Connect a listener to this signal.
    ///
    /// The listener will be called synchronously when `send()` is called.
    /// For async listeners, use `connect_async` (feature: tokio).
    pub fn connect<F>(&self, f: F)
    where
        F: Fn(&T) + Send + Sync + 'static,
    {
        let mut handlers = self.handlers.write().unwrap();
        handlers.push(Arc::new(f));
    }

    /// Emit the signal, calling all connected listeners.
    pub fn send(&self, value: &T) {
        let handlers = self.handlers.read().unwrap();
        for handler in handlers.iter() {
            handler(value);
        }
    }

    /// Remove all listeners.
    pub fn disconnect_all(&self) {
        let mut handlers = self.handlers.write().unwrap();
        handlers.clear();
    }

    /// Number of connected listeners.
    pub fn listener_count(&self) -> usize {
        self.handlers.read().unwrap().len()
    }
}

impl<T: Send + Sync + 'static> Default for Signal<T> {
    fn default() -> Self {
        Signal::new()
    }
}

/// A global signal registry for managing named signals.
///
/// # Example
/// ```rust
/// use rango::signals::SignalRegistry;
///
/// let mut registry = SignalRegistry::new();
/// ```
pub struct SignalRegistry {
    /// Named signal channels for string-typed events.
    channels: std::collections::HashMap<String, Vec<Arc<dyn Fn(serde_json::Value) + Send + Sync>>>,
}

impl SignalRegistry {
    pub fn new() -> Self {
        SignalRegistry {
            channels: std::collections::HashMap::new(),
        }
    }

    /// Connect a listener to a named signal.
    pub fn connect<F>(&mut self, signal: &str, f: F)
    where
        F: Fn(serde_json::Value) + Send + Sync + 'static,
    {
        self.channels
            .entry(signal.to_string())
            .or_default()
            .push(Arc::new(f));
    }

    /// Emit a named signal with a JSON payload.
    pub fn send(&self, signal: &str, value: serde_json::Value) {
        if let Some(handlers) = self.channels.get(signal) {
            for handler in handlers {
                handler(value.clone());
            }
        }
    }
}

impl Default for SignalRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Pre-defined framework signals ────────────────────────────────────────────

/// Pre-save signal — fired before a model is saved to the database.
pub static PRE_SAVE: Signal<serde_json::Value> = Signal::new();

/// Post-save signal — fired after a model is saved to the database.
pub static POST_SAVE: Signal<serde_json::Value> = Signal::new();

/// Pre-delete signal — fired before a model is deleted.
pub static PRE_DELETE: Signal<serde_json::Value> = Signal::new();

/// Post-delete signal — fired after a model is deleted.
pub static POST_DELETE: Signal<serde_json::Value> = Signal::new();

/// Request started signal.
pub static REQUEST_STARTED: Signal<String> = Signal::new();

/// Request finished signal.
pub static REQUEST_FINISHED: Signal<String> = Signal::new();
