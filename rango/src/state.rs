//! Rango global configuration.
//!
//! `RangoConfig` is the single source of truth for all framework settings,
//! including database connection, pooling, migrations, CORS, templates, etc.
//!
//! Configure it once before calling `rango::start()`:
//!
//! ```rust,ignore
//! rango::init_config(RangoConfig {
//!     database: Some(DatabaseConfig::sqlite("rango.db")),
//!     secret_key: std::env::var("RANGO_SECRET_KEY").unwrap_or_default(),
//!     ..RangoConfig::default()
//! });
//! rango::start(router).run().await;
//! ```
//!
//! Or let Rango read everything from environment variables:
//!
//! ```rust,ignore
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

    /// Whether the backend supports `RETURNING` (INSERT ... RETURNING id).
    pub fn supports_returning(&self) -> bool {
        matches!(self, DatabaseBackend::Postgres)
    }

    /// Whether the backend uses numbered placeholders ($1, $2, ...).
    pub fn uses_numbered_placeholders(&self) -> bool {
        matches!(self, DatabaseBackend::Postgres)
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

    /// Idle connection lifetime in seconds (default: 600 = 10 min).
    pub idle_timeout_secs: Option<u64>,

    /// Maximum connection lifetime in seconds (default: 1800 = 30 min).
    pub max_lifetime_secs: Option<u64>,

    /// Path to the migrations directory (default: `./migrations`).
    /// Set to `None` to skip automatic migrations on startup.
    pub migrations_path: Option<String>,

    /// Whether to run migrations automatically on startup (default: true).
    pub auto_migrate: bool,

    /// Log all executed SQL statements (default: false).
    pub log_statements: bool,

    /// Read-only replica URL — used for SELECT queries (optional).
    pub read_replica_url: Option<String>,
}

impl DatabaseConfig {
    /// Create a minimal config from just a URL, with sensible defaults.
    pub fn from_url(url: &str) -> Self {
        DatabaseConfig {
            url: url.to_string(),
            max_connections: 5,
            min_connections: 1,
            connect_timeout_secs: 30,
            idle_timeout_secs: Some(600),
            max_lifetime_secs: Some(1800),
            migrations_path: Some("./migrations".to_string()),
            auto_migrate: true,
            log_statements: false,
            read_replica_url: None,
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

    /// Set min pool connections.
    pub fn min_connections(mut self, n: u32) -> Self {
        self.min_connections = n;
        self
    }

    /// Set connection timeout.
    pub fn connect_timeout(mut self, secs: u64) -> Self {
        self.connect_timeout_secs = secs;
        self
    }

    /// Enable SQL statement logging.
    pub fn log_statements(mut self) -> Self {
        self.log_statements = true;
        self
    }

    /// Set a read replica URL.
    pub fn with_read_replica(mut self, url: &str) -> Self {
        self.read_replica_url = Some(url.to_string());
        self
    }

    /// Parse from environment variables:
    /// - `DATABASE_URL`          — required
    /// - `DB_MAX_CONNECTIONS`    — optional, default 5
    /// - `DB_MIN_CONNECTIONS`    — optional, default 1
    /// - `DB_CONNECT_TIMEOUT`    — optional, default 30
    /// - `DB_MIGRATIONS_PATH`    — optional, default ./migrations
    /// - `DB_AUTO_MIGRATE`       — optional, "false" to disable
    /// - `DB_LOG_STATEMENTS`     — optional, "true" to enable
    /// - `DATABASE_READ_URL`     — optional, read replica
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
        let log_statements = std::env::var("DB_LOG_STATEMENTS")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);
        let read_replica_url = std::env::var("DATABASE_READ_URL").ok();

        Some(DatabaseConfig {
            url,
            max_connections,
            min_connections,
            connect_timeout_secs,
            idle_timeout_secs: Some(600),
            max_lifetime_secs: Some(1800),
            migrations_path,
            auto_migrate,
            log_statements,
            read_replica_url,
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

    /// Allowed host headers. Requests with a different `Host` header are rejected with 400.
    /// Default: `["127.0.0.1", "localhost"]`.
    pub allowed_hosts: Vec<String>,

    /// Directory for Jinja2/MiniJinja templates (default: `"templates"`).
    pub templates_dir: String,

    /// Directory for static files (default: `Some("static")`).
    pub static_dir: Option<String>,

    /// Database connection settings.
    /// Set to `None` to run without a database.
    pub database: Option<DatabaseConfig>,

    /// Secret key used for signing sessions, tokens, etc.
    ///
    /// # Security
    /// **MUST be changed in production** to a long random string.
    /// The default value is intentionally insecure and will print a warning.
    pub secret_key: String,

    /// CORS: allow all origins (default: false).
    ///
    /// # Security
    /// Only enable in development or public APIs.
    pub cors_allow_all: bool,

    /// Specific allowed CORS origins (used when `cors_allow_all` is false).
    pub cors_allowed_origins: Vec<String>,

    /// Address to bind the server (default: `"127.0.0.1:8000"`).
    /// Can be overridden at startup time with `.bind()` on `RangoBuilder`.
    pub bind_addr: String,

    /// Session cookie settings.
    pub session: SessionConfig,

    /// Security headers configuration.
    pub security: SecurityConfig,
}

/// Session configuration.
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Require HTTPS for session cookies (default: false in debug, true in release).
    pub secure: bool,
    /// Session cookie name (default: `"rango_session"`).
    pub cookie_name: String,
    /// Session max age in seconds (default: 86400 = 1 day).
    pub max_age_secs: u64,
    /// HttpOnly cookie (default: true — prevents JS access).
    pub http_only: bool,
    /// SameSite policy: "Strict", "Lax", or "None".
    pub same_site: String,
}

impl Default for SessionConfig {
    fn default() -> Self {
        SessionConfig {
            secure: !cfg!(debug_assertions),
            cookie_name: "rango_session".to_string(),
            max_age_secs: 86400,
            http_only: true,
            same_site: "Lax".to_string(),
        }
    }
}

/// Security headers configuration.
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Enable `X-Content-Type-Options: nosniff` (default: true).
    pub content_type_nosniff: bool,
    /// Enable `X-Frame-Options: DENY` (default: true).
    pub x_frame_options: bool,
    /// Strict-Transport-Security max age in seconds. `None` = disabled.
    pub hsts_max_age: Option<u64>,
    /// Content-Security-Policy header value. `None` = disabled.
    pub csp: Option<String>,
    /// Enable `X-XSS-Protection: 1; mode=block` (default: true).
    pub xss_protection: bool,
    /// Enable `Referrer-Policy: same-origin` (default: true).
    pub referrer_policy: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        SecurityConfig {
            content_type_nosniff: true,
            x_frame_options: true,
            hsts_max_age: None, // Disabled by default; enable in prod
            csp: None,
            xss_protection: true,
            referrer_policy: true,
        }
    }
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
            cors_allowed_origins: Vec::new(),
            bind_addr: "127.0.0.1:8000".to_string(),
            session: SessionConfig::default(),
            security: SecurityConfig::default(),
        }
    }
}

impl RangoConfig {
    /// Build config entirely from environment variables.
    ///
    /// Variables read:
    /// | Variable             | Default                        |
    /// |----------------------|--------------------------------|
    /// | `RANGO_DEBUG`        | `true` in debug builds         |
    /// | `RANGO_SECRET_KEY`   | insecure placeholder           |
    /// | `RANGO_ADDR`         | `127.0.0.1:8000`               |
    /// | `RANGO_TEMPLATES`    | `templates`                    |
    /// | `RANGO_STATIC`       | `static`                       |
    /// | `RANGO_ALLOWED_HOSTS`| `127.0.0.1,localhost`          |
    /// | `DATABASE_URL`       | (none — DB disabled if absent) |
    /// | `DB_MAX_CONNECTIONS` | `5`                            |
    /// | `DB_AUTO_MIGRATE`    | `true`                         |
    /// | `DB_MIGRATIONS_PATH` | `./migrations`                 |
    /// | `SESSION_SECURE`     | false in debug, true in release|
    /// | `SESSION_MAX_AGE`    | `86400`                        |
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
            cors_allowed_origins: std::env::var("RANGO_CORS_ORIGINS")
                .map(|v| v.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_default(),
            allowed_hosts: std::env::var("RANGO_ALLOWED_HOSTS")
                .map(|v| v.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_else(|_| vec!["127.0.0.1".to_string(), "localhost".to_string()]),
            session: SessionConfig {
                secure: std::env::var("SESSION_SECURE")
                    .map(|v| v == "true" || v == "1")
                    .unwrap_or(!cfg!(debug_assertions)),
                cookie_name: std::env::var("SESSION_COOKIE_NAME")
                    .unwrap_or_else(|_| "rango_session".to_string()),
                max_age_secs: std::env::var("SESSION_MAX_AGE")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(86400),
                http_only: true,
                same_site: std::env::var("SESSION_SAME_SITE").unwrap_or_else(|_| "Lax".to_string()),
            },
            security: SecurityConfig::default(),
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

    /// Whether the secret key is the default insecure value.
    pub fn is_secret_key_insecure(&self) -> bool {
        self.secret_key == "rango-insecure-key-change-in-production" || self.secret_key.len() < 32
    }

    /// Validate the configuration and return a list of warnings.
    pub fn validate(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        if self.is_secret_key_insecure() {
            warnings.push(
                "⚠️  SECRET_KEY is insecure or too short! Set RANGO_SECRET_KEY to a random 64+ char string in production.".to_string()
            );
        }
        if !self.debug && self.cors_allow_all {
            warnings.push(
                "⚠️  cors_allow_all is enabled in non-debug mode. This allows any origin."
                    .to_string(),
            );
        }
        if !self.debug && !self.session.secure {
            warnings.push(
                "⚠️  Session cookies are not marked Secure in non-debug mode. Enable session.secure in production.".to_string()
            );
        }
        if self.allowed_hosts.iter().any(|h| h == "*") {
            warnings.push(
                "⚠️  allowed_hosts contains '*' — all host headers are accepted. Set specific hosts in production.".to_string()
            );
        }
        warnings
    }
}

static RANGO_CONFIG: OnceLock<RangoConfig> = OnceLock::new();

/// Initialize the global config. Call once before `rango::start()`.
/// Panics if called more than once.
pub fn init_config(config: RangoConfig) {
    // Print security warnings
    for warning in config.validate() {
        eprintln!("{}", warning);
    }
    RANGO_CONFIG
        .set(config)
        .expect("RangoConfig already initialized — call init_config() only once.");
}

/// Access the global config. Initializes with defaults if never set.
pub fn config() -> &'static RangoConfig {
    RANGO_CONFIG.get_or_init(RangoConfig::default)
}
