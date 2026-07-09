pub mod cache;
mod debug;
pub mod error;
pub mod forms;
pub mod middleware;
mod not_found;
pub mod paginator;
pub mod responses;
pub mod signals;
pub mod state;
pub mod validators;

#[cfg(feature = "auth")]
pub mod auth;
#[cfg(feature = "auth")]
pub mod csrf;
#[cfg(feature = "auth")]
pub mod messages;

#[cfg(feature = "db")]
pub mod db;

#[cfg(all(feature = "db", feature = "templates"))]
pub mod admin;

#[cfg(feature = "templates")]
pub mod template_filters;

pub use serde;
pub use serde_json;

pub use error::{RangoError, RangoResult};
pub use responses::{
    bad_request, created, http_404, json_response, json_response_with_status, no_content, redirect,
    redirect_permanent, text_response,
};
pub use state::{
    config, init_config, DatabaseBackend, DatabaseConfig, RangoConfig, RangoState, SecurityConfig,
    SessionConfig, StateWrapper,
};

#[cfg(all(feature = "db", feature = "templates"))]
pub use admin::RangoAdmin;

#[cfg(feature = "templates")]
pub use responses::render;

#[cfg(feature = "db")]
pub use db::{
    aggregate, aggregate_float, backend, db, execute, init_db, placeholder, query, query_as,
    run_migrations, with_transaction, AdminField, ColumnDef, ModelAdmin, Page, QuerySet,
    RangoAdminMetadata, RangoAdminOps, RangoModel, RangoSchema, SqlValue, Q,
};

pub use cache::{cache, Cache};
pub use forms::Form;
pub use paginator::Paginator;
pub use signals::{Signal, SignalRegistry};
pub use validators::{ValidationErrors, Validator};

#[cfg(feature = "auth")]
pub use messages::{Message, MessageLevel};

pub mod macros {
    pub use rango_macros::context;
    pub use rango_macros::login_required;
    #[cfg(feature = "db")]
    pub use rango_macros::model;
    pub use rango_macros::urls;
    pub use rango_macros::view;
}

pub use axum;
pub use axum::{
    extract::{Json, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
pub use serde_json::json;
#[cfg(feature = "db")]
pub use sqlx;
#[cfg(feature = "auth")]
pub use tower_sessions;

use axum::middleware::from_fn;
use axum::Router;
use tower_http::trace::TraceLayer;

use crate::not_found::default_404_handler;

// ─── RangoBuilder ─────────────────────────────────────────────────────────────

pub struct RangoBuilder {
    router: Router,
    /// Override the bind address from config.
    addr_override: Option<String>,
    /// Override static dir from config.
    static_override: Option<(String, String)>,
    /// Override CORS from config.
    cors_override: Option<bool>,
    /// Enable security headers middleware.
    security_headers: bool,
    /// Enable host validation middleware.
    host_validation: bool,
}

impl RangoBuilder {
    pub fn new(router: Router) -> Self {
        RangoBuilder {
            router,
            addr_override: None,
            static_override: None,
            cors_override: None,
            security_headers: true,
            host_validation: true,
        }
    }

    /// Override the bind address (otherwise taken from `RangoConfig.bind_addr`).
    pub fn bind(mut self, addr: &str) -> Self {
        self.addr_override = Some(addr.to_string());
        self
    }

    /// Override the static file directory (otherwise taken from `RangoConfig.static_dir`).
    pub fn with_static(mut self, url_prefix: &str, fs_path: &str) -> Self {
        self.static_override = Some((url_prefix.to_string(), fs_path.to_string()));
        self
    }

    /// Enable CORS for all origins (overrides `RangoConfig.cors_allow_all`).
    pub fn with_cors(mut self) -> Self {
        self.cors_override = Some(true);
        self
    }

    /// Disable security headers middleware.
    pub fn without_security_headers(mut self) -> Self {
        self.security_headers = false;
        self
    }

    /// Disable host header validation.
    pub fn without_host_validation(mut self) -> Self {
        self.host_validation = false;
        self
    }

    /// Start the server. Reads all settings from `RangoConfig` (set via `init_config()`).
    /// Any `.bind()` / `.with_static()` / `.with_cors()` calls override the config.
    pub async fn run(mut self) {
        let cfg = config();

        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                    if cfg.debug {
                        "rango=debug,tower_http=debug".into()
                    } else {
                        "rango=info,tower_http=warn".into()
                    }
                }),
            )
            .init();

        // ── Database ──────────────────────────────────────────────────────────
        #[cfg(feature = "db")]
        if let Some(ref db_cfg) = cfg.database {
            let backend = db_cfg.backend();
            println!(
                "🗄️  Connecting to {} (max_conn={}, timeout={}s) …",
                backend.name(),
                db_cfg.max_connections,
                db_cfg.connect_timeout_secs,
            );
            db::init_db_with_config(db_cfg)
                .await
                .unwrap_or_else(|e| panic!("Rango: DB connection failed: {}", e));

            if db_cfg.auto_migrate {
                if let Some(ref path) = db_cfg.migrations_path {
                    db::run_migrations(path)
                        .await
                        .unwrap_or_else(|e| panic!("Rango: migrations failed: {}", e));
                }
            }
        }

        // ── Static files ──────────────────────────────────────────────────────
        let static_cfg = self.static_override.or_else(|| {
            cfg.static_dir
                .as_ref()
                .map(|dir| ("/static".to_string(), dir.clone()))
        });
        if let Some((prefix, path)) = static_cfg {
            self.router = self
                .router
                .nest_service(&prefix, middleware::static_files_service(&path));
        }

        // ── CORS ──────────────────────────────────────────────────────────────
        let cors_enabled = self.cors_override.unwrap_or(cfg.cors_allow_all);
        if cors_enabled {
            self.router = self.router.layer(middleware::cors_layer());
        } else if !cfg.cors_allowed_origins.is_empty() {
            // Use origin-specific CORS
            use axum::http::HeaderValue;
            let origins: Vec<HeaderValue> = cfg
                .cors_allowed_origins
                .iter()
                .filter_map(|o| o.parse().ok())
                .collect();
            use tower_http::cors::{Any, CorsLayer};
            self.router = self.router.layer(
                CorsLayer::new()
                    .allow_methods([
                        axum::http::Method::GET,
                        axum::http::Method::POST,
                        axum::http::Method::PUT,
                        axum::http::Method::DELETE,
                        axum::http::Method::PATCH,
                        axum::http::Method::OPTIONS,
                    ])
                    .allow_headers(Any)
                    .allow_origin(origins),
            );
        }

        // ── Sessions ──────────────────────────────────────────────────────────
        #[cfg(feature = "auth")]
        {
            use tower_sessions::{cookie::SameSite, MemoryStore, SessionManagerLayer};
            let session_store = MemoryStore::default();
            let same_site = match cfg.session.same_site.as_str() {
                "Strict" => SameSite::Strict,
                "None" => SameSite::None,
                _ => SameSite::Lax,
            };
            let session_layer = SessionManagerLayer::new(session_store)
                .with_secure(cfg.session.secure)
                .with_http_only(cfg.session.http_only)
                .with_same_site(same_site)
                .with_name(&cfg.session.cookie_name);
            self.router = self.router.layer(session_layer);
        }

        // ── Host validation ───────────────────────────────────────────────────
        if self.host_validation && !cfg.allowed_hosts.is_empty() {
            self.router = self
                .router
                .layer(from_fn(middleware::host_validation_middleware));
        }

        // ── Security headers ──────────────────────────────────────────────────
        if self.security_headers {
            self.router = self
                .router
                .layer(from_fn(middleware::security_headers_middleware));
        }

        // ── Middleware stack ──────────────────────────────────────────────────
        self.router = self
            .router
            .layer(from_fn(middleware::logger_middleware))
            .layer(TraceLayer::new_for_http())
            .fallback(default_404_handler);

        if cfg.debug {
            println!("🛠️  Rango Debugger enabled");
            self.router = self
                .router
                .layer(from_fn(crate::debug::debug_error_middleware));
        }

        // ── Bind & serve ──────────────────────────────────────────────────────
        let addr = self
            .addr_override
            .as_deref()
            .unwrap_or(&cfg.bind_addr)
            .to_string();

        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .unwrap_or_else(|e| panic!("Cannot bind Rango on {}: {}", addr, e));

        println!("Rango running on http://{}", addr);
        if cfg.debug {
            println!("   Debug mode: ON");
            println!("   Database: {}", cfg.database_url().unwrap_or("none"));
        }

        axum::serve(listener, self.router).await.unwrap();
    }
}

// ─── Entry points ─────────────────────────────────────────────────────────────

/// Start the server using settings from `RangoConfig`.
pub fn start(router: Router) -> RangoBuilder {
    RangoBuilder::new(router)
}

/// Convenience: start with a custom address (ignores `RangoConfig.bind_addr`).
pub async fn run(router: Router, addr: &str) {
    RangoBuilder::new(router).bind(addr).run().await;
}
