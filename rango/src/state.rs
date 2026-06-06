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

#[derive(Debug, Clone)]
pub struct RangoConfig {
    pub debug: bool,
    pub allowed_hosts: Vec<String>,
    pub templates_dir: String,
    pub static_dir: Option<String>,
    pub database_url: Option<String>,
    pub secret_key: String,
}

impl Default for RangoConfig {
    fn default() -> Self {
        RangoConfig {
            debug: cfg!(debug_assertions),
            allowed_hosts: vec!["127.0.0.1".to_string(), "localhost".to_string()],
            templates_dir: "templates".to_string(),
            static_dir: Some("static".to_string()),
            database_url: None,
            secret_key: "rango-insecure-key-change-in-production".to_string(),
        }
    }
}

static RANGO_CONFIG: OnceLock<RangoConfig> = OnceLock::new();

pub fn init_config(config: RangoConfig) {
    RANGO_CONFIG
        .set(config)
        .expect("RangoConfig already initialized");
}

pub fn config() -> &'static RangoConfig {
    RANGO_CONFIG.get_or_init(RangoConfig::default)
}
