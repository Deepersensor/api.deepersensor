use tracing_subscriber::{fmt, EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use ds_core::config::AppConfig;

pub fn init_tracing(cfg: &AppConfig) {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    // Use JSON formatting consistently for now to avoid type incompatibilities
    let fmt_layer = fmt::layer().json().with_target(false);
    tracing_subscriber::registry().with(env_filter).with(fmt_layer).init();
}

pub const REQUEST_ID_HEADER: &str = "x-request-id";