use std::{sync::Arc, time::Instant, net::IpAddr};
use dashmap::DashMap;
use ds_core::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Clone)]
pub struct TokenBucket { tokens: Arc<tokio::sync::Mutex<(u64, Instant)>>, rate_per_min: u64, burst: u64 }

impl TokenBucket {
    pub fn new(rate_per_min: u64, burst: u64) -> Self { Self { tokens: Arc::new(tokio::sync::Mutex::new((burst, Instant::now()))), rate_per_min, burst } }
    pub async fn allow(&self) -> bool {
        let per_sec = self.rate_per_min as f64 / 60.0;
        let mut guard = self.tokens.lock().await;
        let (ref mut available, ref mut last) = *guard;
        let now = Instant::now();
        let elapsed = now.duration_since(*last).as_secs_f64();
        if elapsed > 0.0 {
            let refill = (per_sec * elapsed) as u64; 
            if refill > 0 { *available = (*available + refill).min(self.burst); *last = now; }
        }
        if *available > 0 { *available -= 1; true } else { false }
    }
}

pub async fn rate_limit(state: &AppState, ip: IpAddr) -> ApiResult<()> {
    if !state.cfg.rate_limit.enabled { return Ok(()); }
    let key = ip.to_string();
    let entry = state.rate_map.entry(key).or_insert_with(|| TokenBucket::new(state.cfg.rate_limit.requests_per_minute, state.cfg.rate_limit.burst));
    if !entry.allow().await { return Err(ApiError::RateLimited); }
    Ok(())
}

pub fn _rate_map_len(map: &DashMap<String, TokenBucket>) -> usize { map.len() }
