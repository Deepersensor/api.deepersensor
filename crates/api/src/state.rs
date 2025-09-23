use std::sync::Arc;
use dashmap::DashMap;
use ds_core::config::AppConfig;
use ds_model::ModelProvider;

#[derive(Clone)]
pub struct AppState {
    pub provider: Arc<dyn ModelProvider>,
    pub rate_map: Arc<DashMap<String, crate::rate_limit::TokenBucket>>, 
    pub cfg: Arc<AppConfig>,
}

impl AppState {
    pub fn new(provider: Arc<dyn ModelProvider>, cfg: Arc<AppConfig>) -> Self {
        Self { provider, rate_map: Arc::new(DashMap::new()), cfg }
    }
    pub fn config(&self) -> &AppConfig { &self.cfg }
}
