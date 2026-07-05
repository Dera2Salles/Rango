//! Rango global configuration.
//!
//! `RangoConfig` is the single source of truth for all framework settings,
//! including database connection, pooling, migrations, CORS, templates, etc.
//!
//! Configure it once before calling `rango::start()`:
//!
//! ```rust
//! rango::init_config(RangoConfig {
//!     database_url: Some("postgres://user:pass@localhost/mydb".into()),
//!     ..RangoConfig::default()
//! });
//! rango::start(router).run().await;
//! ```
//!
//! Or let Rango read everything from environment variables:
//!
//! ```rust
//! rango::init_config(RangoConfig::from_env());
//! ```

use std::sync::{Arc, OnceLock};

pub trait RangoState: Clone + Send + Sync + 'static {}

#[derive(Clone)]
pub struct StateWrapper<S: RangoState> {
    pub inner: Arc<S>,
}

impl<S: RangoState> StateWrapper<S> {
    pub fn new(state: S) -> Self {
        StateWrapper {
            inner: Arc::new(state),
        }
    }
}

/// Supported database backends.
#[derive(Debug, Clone, PartialEq)]
pub enum DatabaseBackend {
    /// SQLite — `sqlite://path/to/db.sqlite3` or `sqlite::memory:`
    Sqlite,
    /// PostgreSQL — `postgres://user:pass@host/dbname`
    Postgres,
    /// MySQL / MariaDB — `mysql://user:pass@host/dbname`
    Mysql,
    /// Any backend auto-detected from the URL prefix.
    Any,
}

impl DatabaseBackend {
    /// Detect backend from a database URL string.
    pub fn from_url(url: &str) -> Self {
        if url.starts_with("sqlite") {
            DatabaseBackend::Sqlite
        } else if url.starts_with("postgres") || url.starts_with("postgresql") {
            DatabaseBackend::Postgres
        } else if url.starts_with("mysql") || url.starts_with("mariadb") {
            DatabaseBackend::Mysql
        } else {
            DatabaseBackend::Any
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            DatabaseBackend::Sqlite => "SQLite",
            DatabaseBackend::Postgres => "PostgreSQL",
            DatabaseBackend::Mysql => "MySQL/MariaDB",
            DatabaseBackend::Any => "Unknown",
        }
    }
}

/// Full database connection configuration.
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    /// Connection URL.
    /// Examples:
    /// - `sqlite://rango.db`
    /// - `sqlite::memory:`
    /// - `postgres://user:password@localhost:5432/mydb`
    /// - `mysql://user:password@localhost:3306/mydb`
    pub url: String,

    /// Maximum number of connections in the pool (default: 5).
    pub max_connections: u32,

    /// Minimum number of idle connections to keep open (default: 1).
    pub min_connections: u32,

    /// Connection timeout in seconds (default: 30).
    pub connect_timeout_secs: u64,

    /// Path to the migrations directory (default: `./migrations`).
    /// Set to `None` to skip automatic migrations on startup.
    pub migrations_path: Option<String>,

    /// Whether to run migrations automatically on startup (default: true).
    pub auto_migrate: bool,
}

impl DatabaseConfig {
    /// Create a minimal config from just a URL, with sensible defaults.
    pub fn from_url(url: &str) -> Self {
        DatabaseConfig {
            url: url.to_string(),
            max_connections: 5,
            min_connections: 1,
            connect_timeout_secs: 30,
            migrations_path: Some("./migrations".to_string()),
            auto_migrate: true,
        }
    }

    /// SQLite shortcut.
    pub fn sqlite(path: &str) -> Self {
        let url = if path == ":memory:" {
            "sqlite::memory:".to_string()
        } else if path.starts_with("sqlite") {
            path.to_string()
        } else {
            format!("sqlite://{}", path)
        };
        Self::from_url(&url)
    }

    /// PostgreSQL shortcut.
    pub fn postgres(host: &str, port: u16, user: &str, password: &str, db: &str) -> Self {
        let url = format!("postgres://{}:{}@{}:{}/{}", user, password, host, port, db);
        Self::from_url(&url)
    }

    /// MySQL shortcut.
    pub fn mysql(host: &str, port: u16, user: &str, password: &str, db: &str) -> Self {
        let url = format!("mysql://{}:{}@{}:{}/{}", user, password, host, port, db);
        Self::from_url(&url)
    }

    /// Detect the backend from the URL.
    pub fn backend(&self) -> DatabaseBackend {
        DatabaseBackend::from_url(&self.url)
    }

    /// Disable automatic migrations.
    pub fn no_auto_migrate(mut self) -> Self {
        self.auto_migrate = false;
        self
    }

    /// Set a custom migrations path.
    pub fn migrations(mut self, path: &str) -> Self {
        self.migrations_path = Some(path.to_string());
        self
    }

    /// Set max pool connections.
    pub fn max_connections(mut self, n: u32) -> Self {
        self.max_connections = n;
        self
    }

    /// Parse from environment variables:
    /// - `DATABASE_URL`          — required
    /// - `DB_MAX_CONNECTIONS`    — optional, default 5
    /// - `DB_MIN_CONNECTIONS`    — optional, default 1
    /// - `DB_CONNECT_TIMEOUT`    — optional, default 30
    /// - `DB_MIGRATIONS_PATH`    — optional, default ./migrations
    /// - `DB_AUTO_MIGRATE`       — optional, "false" to disable
    pub fn from_env() -> Option<Self> {
        let url = std::env::var("DATABASE_URL").ok()?;
        let max_connections = std::env::var("DB_MAX_CONNECTIONS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5);
        let min_connections = std::env::var("DB_MIN_CONNECTIONS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1);
        let connect_timeout_secs = std::env::var("DB_CONNECT_TIMEOUT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30);
        let migrations_path = std::env::var("DB_MIGRATIONS_PATH")
            .ok()
            .or_else(|| Some("./migrations".to_string()));
        let auto_migrate = std::env::var("DB_AUTO_MIGRATE")
            .map(|v| v.to_lowercase() != "false")
            .unwrap_or(true);

        Some(DatabaseConfig {
            url,
            max_connections,
            min_connections,
            connect_timeout_secs,
            migrations_path,
            auto_migrate,
        })
    }
}

/// Global framework configuration.
///
/// Pass to `rango::init_config()` before calling `rango::start()`.
/// All fields have sensible defaults — only override what you need.
#[derive(Debug, Clone)]
pub struct RangoConfig {
    /// Enable debug mode: detailed error pages, verbose logging (default: true in debug builds).
    pub debug: bool,

    /// Allowed host headers (default: `["127.0.0.1", "localhost"]`).
    pub allowed_hosts: Vec<String>,

    /// Directory for Jinja2/MiniJinja templates (default: `"templates"`).
    pub templates_dir: String,

    /// Directory for static files (default: `Some("static")`).
    pub static_dir: Option<String>,

    /// Database connection settings.
    /// Set to `None` to run without a database.
    pub database: Option<DatabaseConfig>,

    /// Secret key used for signing sessions, tokens, etc.
    /// **Always change this in production.**
    pub secret_key: String,

    /// CORS: allow all origins (default: false).
    pub cors_allow_all: bool,

    /// Address to bind the server (default: `"127.0.0.1:8000"`).
    /// Can be overridden at startup time with `.bind()` on `RangoBuilder`.
    pub bind_addr: String,
}

impl Default for RangoConfig {
    fn default() -> Self {
        RangoConfig {
            debug: cfg!(debug_assertions),
            allowed_hosts: vec!["127.0.0.1".to_string(), "localhost".to_string()],
            templates_dir: "templates".to_string(),
            static_dir: Some("static".to_string()),
            database: None,
            secret_key: "rango-insecure-key-change-in-production".to_string(),
            cors_allow_all: false,
            bind_addr: "127.0.0.1:8000".to_string(),
        }
    }
}

impl RangoConfig {
    /// Build config entirely from environment variables.
    ///
    /// Variables read:
    /// | Variable            | Default                        |
    /// |---------------------|--------------------------------|
    /// | `RANGO_DEBUG`       | `true` in debug builds         |
    /// | `RANGO_SECRET_KEY`  | insecure placeholder           |
    /// | `RANGO_ADDR`        | `127.0.0.1:8000`               |
    /// | `RANGO_TEMPLATES`   | `templates`                    |
    /// | `RANGO_STATIC`      | `static`                       |
    /// | `DATABASE_URL`      | (none — DB disabled if absent) |
    /// | `DB_MAX_CONNECTIONS`| `5`                            |
    /// | `DB_AUTO_MIGRATE`   | `true`                         |
    /// | `DB_MIGRATIONS_PATH`| `./migrations`                 |
    pub fn from_env() -> Self {
        RangoConfig {
            debug: std::env::var("RANGO_DEBUG")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(cfg!(debug_assertions)),
            secret_key: std::env::var("RANGO_SECRET_KEY")
                .unwrap_or_else(|_| "rango-insecure-key-change-in-production".to_string()),
            bind_addr: std::env::var("RANGO_ADDR").unwrap_or_else(|_| "127.0.0.1:8000".to_string()),
            templates_dir: std::env::var("RANGO_TEMPLATES")
                .unwrap_or_else(|_| "templates".to_string()),
            static_dir: Some(
                std::env::var("RANGO_STATIC").unwrap_or_else(|_| "static".to_string()),
            ),
            database: DatabaseConfig::from_env(),
            cors_allow_all: std::env::var("RANGO_CORS_ALL")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
            allowed_hosts: std::env::var("RANGO_ALLOWED_HOSTS")
                .map(|v| v.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_else(|_| vec!["127.0.0.1".to_string(), "localhost".to_string()]),
        }
    }

    /// Quick preset: SQLite database, debug on.
    pub fn sqlite(db_path: &str) -> Self {
        RangoConfig {
            database: Some(DatabaseConfig::sqlite(db_path)),
            ..Default::default()
        }
    }

    pub fn postgres(host: &str, port: u16, user: &str, password: &str, db: &str) -> Self {
        RangoConfig {
            database: Some(DatabaseConfig::postgres(host, port, user, password, db)),
            ..Default::default()
        }
    }

    pub fn mysql(host: &str, port: u16, user: &str, password: &str, db: &str) -> Self {
        RangoConfig {
            database: Some(DatabaseConfig::mysql(host, port, user, password, db)),
            ..Default::default()
        }
    }

    /// Shorthand: get the database URL if configured.
    pub fn database_url(&self) -> Option<&str> {
        self.database.as_ref().map(|d| d.url.as_str())
    }
}

static RANGO_CONFIG: OnceLock<RangoConfig> = OnceLock::new();

/// Initialize the global config. Call once before `rango::start()`.
/// Panics if called more than once.
pub fn init_config(config: RangoConfig) {
    RANGO_CONFIG
        .set(config)
        .expect("RangoConfig already initialized — call init_config() only once.");
}

/// Access the global config. Initializes with defaults if never set.
pub fn config() -> &'static RangoConfig {
    RANGO_CONFIG.get_or_init(RangoConfig::default)
}
