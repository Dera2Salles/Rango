mod debug;
pub mod error;
pub mod middleware;
mod not_found;
pub mod responses;
pub mod state;

#[cfg(feature = "db")]
pub mod db;

pub use error::{RangoError, RangoResult};
pub use responses::{
    http_404, json_response, json_response_with_status, redirect, redirect_permanent, text_response,
};
pub use state::{config, init_config, RangoConfig, RangoState, StateWrapper};

#[cfg(feature = "templates")]
pub use responses::render;

#[cfg(feature = "db")]
pub use db::{db, init_db, RangoModel};

pub mod macros {
    pub use rango_macros::context;
    pub use rango_macros::login_required;
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

use axum::middleware::from_fn;
use axum::Router;
use tower_http::trace::TraceLayer;

use crate::not_found::default_404_handler;

pub struct RangoBuilder {
    router: Router,
    addr: String,
    static_dir: Option<(String, String)>,
    enable_cors: bool,
}

impl RangoBuilder {
    pub fn new(router: Router) -> Self {
        RangoBuilder {
            router,
            addr: "127.0.0.1:8000".to_string(),
            static_dir: None,
            enable_cors: false,
        }
    }

    pub fn bind(mut self, addr: &str) -> Self {
        self.addr = addr.to_string();
        self
    }

    pub fn with_static(mut self, url_prefix: &str, fs_path: &str) -> Self {
        self.static_dir = Some((url_prefix.to_string(), fs_path.to_string()));
        self
    }

    pub fn with_cors(mut self) -> Self {
        self.enable_cors = true;
        self
    }

    pub async fn run(mut self) {
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "rango=debug,tower_http=debug".into()),
            )
            .init();

        if let Some((prefix, path)) = self.static_dir {
            self.router = self
                .router
                .nest_service(&prefix, middleware::static_files_service(&path));
        }

        if self.enable_cors {
            self.router = self.router.layer(middleware::cors_layer());
        }

        self.router = self
            .router
            .layer(from_fn(middleware::logger_middleware))
            .layer(TraceLayer::new_for_http());

        self.router = self.router.fallback(default_404_handler);

        if config().debug {
            println!("🛠️  Rango Debugger enabled");
            self.router = self
                .router
                .layer(from_fn(crate::debug::debug_error_middleware));
        }

        let listener = tokio::net::TcpListener::bind(&self.addr)
            .await
            .unwrap_or_else(|e| panic!("Cannot run Rango on {} : {}", self.addr, e));

        println!("🤠 Rango running on http://{}", self.addr);

        axum::serve(listener, self.router).await.unwrap();
    }
}

pub async fn run(router: Router, addr: &str) {
    RangoBuilder::new(router).bind(addr).run().await;
}

pub fn start(router: Router) -> RangoBuilder {
    RangoBuilder::new(router)
}
