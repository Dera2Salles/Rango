//! Django-like Cache Framework
//!
//! A lightweight, in-process, thread-safe cache with per-key TTL —
//! mirroring the ergonomics of `django.core.cache.cache`.
//!
//! For multi-process / multi-server deployments, back this with Redis
//! instead (swap out the `Cache` implementation, keep the same call sites).
//!
//! # Example
//! ```rust
//! use rango::cache::cache;
//! use std::time::Duration;
//!
//! cache().set("home_page_stats", serde_json::json!({ "views": 42 }), Some(Duration::from_secs(60)));
//!
//! if let Some(stats) = cache().get("home_page_stats") {
//!     println!("{}", stats);
//! }
//!
//! // Compute-and-cache in one call:
//! let value = cache().get_or_set("expensive_calc", Duration::from_secs(300), || {
//!     serde_json::json!({ "result": 1 + 1 })
//! });
//! ```

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

struct Entry {
    value: serde_json::Value,
    expires_at: Option<Instant>,
}

/// A thread-safe in-memory cache with optional per-key expiration.
pub struct Cache {
    store: Mutex<HashMap<String, Entry>>,
}

impl Cache {
    fn new() -> Self {
        Cache {
            store: Mutex::new(HashMap::new()),
        }
    }

    /// Store a value under `key`, optionally expiring after `ttl`.
    /// A `None` ttl means the value never expires (until overwritten or cleared).
    pub fn set(&self, key: &str, value: serde_json::Value, ttl: Option<Duration>) {
        let expires_at = ttl.map(|d| Instant::now() + d);
        let mut store = self.store.lock().unwrap();
        store.insert(key.to_string(), Entry { value, expires_at });
    }

    /// Retrieve a value by key. Returns `None` if missing or expired
    /// (expired entries are lazily evicted on access).
    pub fn get(&self, key: &str) -> Option<serde_json::Value> {
        let mut store = self.store.lock().unwrap();
        match store.get(key) {
            Some(entry) => {
                if let Some(exp) = entry.expires_at {
                    if Instant::now() >= exp {
                        store.remove(key);
                        return None;
                    }
                }
                Some(entry.value.clone())
            }
            None => None,
        }
    }

    /// Whether a (non-expired) value exists for `key`.
    pub fn contains(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    /// Remove a key from the cache. No-op if it doesn't exist.
    pub fn delete(&self, key: &str) {
        self.store.lock().unwrap().remove(key);
    }

    /// Remove every entry from the cache.
    pub fn clear(&self) {
        self.store.lock().unwrap().clear();
    }

    /// Number of entries currently stored (including any not-yet-evicted expired ones).
    pub fn len(&self) -> usize {
        self.store.lock().unwrap().len()
    }

    /// Get a cached value, or compute + store it (with `ttl`) if missing/expired.
    /// Like Django's cache pattern: `cache.get_or_set(key, callable, timeout)`.
    pub fn get_or_set<F>(&self, key: &str, ttl: Duration, compute: F) -> serde_json::Value
    where
        F: FnOnce() -> serde_json::Value,
    {
        if let Some(v) = self.get(key) {
            return v;
        }
        let value = compute();
        self.set(key, value.clone(), Some(ttl));
        value
    }

    /// Increment an integer counter stored at `key` (creating it at `0` if absent),
    /// returning the new value. Non-integer existing values are treated as `0`.
    pub fn incr(&self, key: &str, delta: i64) -> i64 {
        let mut store = self.store.lock().unwrap();
        let current = store
            .get(key)
            .and_then(|e| {
                if let Some(exp) = e.expires_at {
                    if Instant::now() >= exp {
                        return None;
                    }
                }
                e.value.as_i64()
            })
            .unwrap_or(0);
        let new_value = current + delta;
        store.insert(
            key.to_string(),
            Entry {
                value: serde_json::json!(new_value),
                expires_at: None,
            },
        );
        new_value
    }
}

static GLOBAL_CACHE: OnceLock<Cache> = OnceLock::new();

/// Access the process-wide cache singleton.
pub fn cache() -> &'static Cache {
    GLOBAL_CACHE.get_or_init(Cache::new)
}
