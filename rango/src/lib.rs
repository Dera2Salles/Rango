mod debug;
pub mod error;
pub mod middleware;
mod not_found;
pub mod responses;
pub mod state;

#[cfg(feature = "db")]
pub mod db;

#[cfg(all(feature = "db", feature = "templates"))]
pub mod admin;

pub use serde;
pub use serde_json;

pub use error::{RangoError, RangoResult};
pub use responses::{
    http_404, json_response, json_response_with_status, redirect, redirect_permanent, text_response,
};
pub use state::{
    config, init_config,
    RangoConfig, DatabaseConfig, DatabaseBackend,
    RangoState, StateWrapper,
};

#[cfg(all(feature = "db", feature = "templates"))]
pub use admin::RangoAdmin;

#[cfg(feature = "templates")]
pub use responses::render;

#[cfg(feature = "db")]
pub use db::{
    db, backend, placeholder,
    execute, init_db, query, query_as, run_migrations,
    aggregate, aggregate_float, with_transaction,
    RangoModel, RangoSchema, QuerySet, ColumnDef,
    AdminField, RangoAdminMetadata, RangoAdminOps, ModelAdmin,
};


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
}

impl RangoBuilder {
    pub fn new(router: Router) -> Self {
        RangoBuilder {
            router,
            addr_override: None,
            static_override: None,
            cors_override: None,
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

    /// Start the server. Reads all settings from `RangoConfig` (set via `init_config()`).
    /// Any `.bind()` / `.with_static()` / `.with_cors()` calls override the config.
    pub async fn run(mut self) {
        let cfg = config();

        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| {
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

        println!("🤠 Rango running on http://{}", addr);

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
