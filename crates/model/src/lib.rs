use async_stream::try_stream;
use futures_core::Stream;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ModelError {
    #[error("Upstream request failed: {0}")] Upstream(String),
    #[error("Timeout")] Timeout,
    #[error("Other: {0}")] Other(String),
}

pub type ModelResult<T> = Result<T, ModelError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage { pub role: String, pub content: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest { pub model: String, pub messages: Vec<ChatMessage> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChunk { pub model: String, pub content: String, pub done: bool }

#[async_trait::async_trait]
pub trait ModelProvider: Send + Sync + 'static {
    async fn list_models(&self) -> ModelResult<Vec<String>>;
    async fn chat_stream(&self, req: ChatRequest) -> ModelResult<ChatStream>;
}

pub type ChatStream = Pin<Box<dyn Stream<Item = ModelResult<ChatChunk>> + Send>>;
use std::pin::Pin;

pub struct OllamaProvider {
    base: String,
    client: reqwest::Client,
    timeout: Duration,
}

impl OllamaProvider {
    pub fn new(base: impl Into<String>, timeout: Duration) -> Self {
        Self { base: base.into(), client: reqwest::Client::new(), timeout }
    }
}

#[async_trait::async_trait]
impl ModelProvider for OllamaProvider {
    async fn list_models(&self) -> ModelResult<Vec<String>> {
        // Placeholder: upstream endpoint may differ
        let url = format!("{}/api/tags", self.base);
        let resp = self.client.get(url).timeout(self.timeout).send().await.map_err(|e| ModelError::Upstream(e.to_string()))?;
        let v: serde_json::Value = resp.json().await.map_err(|e| ModelError::Upstream(e.to_string()))?;
        let mut names = Vec::new();
        if let Some(arr) = v.get("models").and_then(|m| m.as_array()) {
            for m in arr { if let Some(name) = m.get("name").and_then(|n| n.as_str()) { names.push(name.to_string()); } }
        }
        Ok(names)
    }

    async fn chat_stream(&self, req: ChatRequest) -> ModelResult<ChatStream> {
        // Placeholder streaming simulation
        let model = req.model.clone();
        let content = req.messages.iter().map(|m| m.content.clone()).collect::<Vec<_>>().join(" ");
        let stream = try_stream! {
            yield ChatChunk { model: model.clone(), content: format!("echo: {}", content), done: false };
            yield ChatChunk { model: model.clone(), content: String::new(), done: true };
        };
        Ok(Box::pin(stream))
    }
}
