use tracing_subscriber::{fmt, EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use ds_core::config::AppConfig;

pub fn init_tracing(cfg: &AppConfig) {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    // Box the fmt layer to erase concrete types so conditional selection is possible
    let fmt_layer_boxed: Box<dyn tracing_subscriber::layer::Layer<tracing_subscriber::Registry> + Send + Sync> = if cfg.logging.log_format.as_str() == "json" {
        Box::new(fmt::layer().json().with_target(false))
    } else {
        Box::new(fmt::layer().with_target(false))
    };
    tracing_subscriber::registry().with(env_filter).with(fmt_layer_boxed).init();
}

pub const REQUEST_ID_HEADER: &str = "x-request-id";