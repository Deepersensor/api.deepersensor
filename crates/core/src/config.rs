use serde::Deserialize;
use std::{env, time::Duration};

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub app: AppSection,
    pub logging: LoggingSection,
    pub security: SecuritySection,
    pub rate_limit: RateLimitSection,
    pub ollama: OllamaSection,
    pub redis: RedisSection,
    pub http: HttpSection,
    pub cors: CorsSection,
    pub database: DatabaseSection,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppSection {
    pub env: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub public_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingSection {
    pub log_format: String,
    pub request_id_header: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SecuritySection {
    pub jwt_secret: String,
    pub jwt_issuer: String,
    pub jwt_access_ttl_secs: u64,
    pub jwt_refresh_ttl_secs: u64,
    pub allowed_origins: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitSection {
    pub enabled: bool,
    pub requests_per_minute: u64,
    pub burst: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OllamaSection {
    pub base_url: String,
    pub default_timeout_ms: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RedisSection { pub url: String }

#[derive(Debug, Clone, Deserialize)]
pub struct HttpSection {
    pub read_timeout_secs: u64,
    pub write_timeout_secs: u64,
    pub idle_timeout_secs: u64,
    pub max_request_size_bytes: u64,
    pub trusted_proxy_ips: String,
    pub force_https: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CorsSection {
    pub allow_credentials: bool,
    pub allow_headers: String,
    pub expose_headers: String,
    pub allow_methods: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseSection { pub url: String }

impl AppConfig {
    pub fn load() -> anyhow::Result<Self> {
        // Load .env if present
        let _ = dotenvy::dotenv();
        let builder = config::Config::builder()
            .set_default("app.env", env_or("APP_ENV", "local"))?
            .set_default("app.name", env_or("APP_NAME", "deepersensor-api"))?
            .set_default("app.host", env_or("APP_HOST", "0.0.0.0"))?
            .set_default("app.port", env_or("APP_PORT", "8080"))?
            .set_default("app.public_url", env_or("APP_PUBLIC_URL", "http://localhost:8080"))?
            .set_default("logging.log_format", env_or("LOG_FORMAT", "text"))?
            .set_default("logging.request_id_header", env_or("REQUEST_ID_HEADER", "X-Request-Id"))?
            .set_default("security.jwt_secret", env_or("JWT_SECRET", "dev_insecure_change_me"))?
            .set_default("security.jwt_issuer", env_or("JWT_ISSUER", "deepersensor"))?
            .set_default("security.jwt_access_ttl_secs", env_or("JWT_ACCESS_TTL_SECS", "900"))?
            .set_default("security.jwt_refresh_ttl_secs", env_or("JWT_REFRESH_TTL_SECS", "1209600"))?
            .set_default("security.allowed_origins", env_or("ALLOWED_ORIGINS", "http://localhost:3000"))?
            .set_default("rate_limit.enabled", env_or("RATE_LIMIT_ENABLED", "true"))?
            .set_default("rate_limit.requests_per_minute", env_or("RATE_LIMIT_REQUESTS_PER_MINUTE", "60"))?
            .set_default("rate_limit.burst", env_or("RATE_LIMIT_BURST", "20"))?
            .set_default("ollama.base_url", env_or("OLLAMA_BASE_URL", "http://localhost:11434"))?
            .set_default("ollama.default_timeout_ms", env_or("OLLAMA_DEFAULT_TIMEOUT_MS", "30000"))?
            .set_default("redis.url", env_or("REDIS_URL", "redis://127.0.0.1:6379/0"))?
            .set_default("http.read_timeout_secs", env_or("SERVER_READ_TIMEOUT_SECS", "15"))?
            .set_default("http.write_timeout_secs", env_or("SERVER_WRITE_TIMEOUT_SECS", "30"))?
            .set_default("http.idle_timeout_secs", env_or("SERVER_IDLE_TIMEOUT_SECS", "120"))?
            .set_default("http.max_request_size_bytes", env_or("MAX_REQUEST_SIZE_BYTES", "1048576"))?
            .set_default("http.trusted_proxy_ips", env_or("TRUSTED_PROXY_IPS", "127.0.0.1,::1"))?
            .set_default("http.force_https", env_or("FORCE_HTTPS", "false"))?
            .set_default("cors.allow_credentials", env_or("CORS_ALLOW_CREDENTIALS", "false"))?
            .set_default("cors.allow_headers", env_or("CORS_ALLOW_HEADERS", "Authorization,Content-Type"))?
            .set_default("cors.expose_headers", env_or("CORS_EXPOSE_HEADERS", "Authorization,Content-Type"))?
            .set_default("cors.allow_methods", env_or("CORS_ALLOW_METHODS", "GET,POST,OPTIONS"))?
            .set_default("database.url", env_or("DATABASE_URL", "postgres://postgres:postgres@localhost:5432/deepersensor"))?;

        let cfg = builder.build()?;
        Ok(cfg.try_deserialize()?)
    }

    pub fn is_production(&self) -> bool { self.app.env == "production" }
    pub fn database_url(&self) -> &str { &self.database.url }
    pub fn access_ttl(&self) -> Duration { Duration::from_secs(self.security.jwt_access_ttl_secs) }
    pub fn refresh_ttl(&self) -> Duration { Duration::from_secs(self.security.jwt_refresh_ttl_secs) }
}

fn env_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}
