use std::sync::Arc;
use dashmap::DashMap;
use ds_core::config::AppConfig;
use ds_model::ModelProvider;

#[derive(Clone)]
pub struct AppState {
    pub provider: Arc<dyn ModelProvider>,
    pub rate_map: Arc<DashMap<String, crate::rate_limit::TokenBucket>>, 
    pub cfg: Arc<AppConfig>,
    pub db: sqlx::PgPool,
}

impl AppState {
    pub fn new(provider: Arc<dyn ModelProvider>, cfg: Arc<AppConfig>, db: sqlx::PgPool) -> Self {
        Self { provider, rate_map: Arc::new(DashMap::new()), cfg, db }
    }
    pub fn config(&self) -> &AppConfig { &self.cfg }
}
