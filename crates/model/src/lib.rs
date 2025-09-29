use async_stream::try_stream;
use futures_core::Stream;
use serde::{Deserialize, Serialize};
use std::{pin::Pin, time::Duration};
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

pub struct OllamaProvider {
    base: String,
    client: reqwest::Client,
    timeout: Duration,
}

impl OllamaProvider {
    pub fn new(base: impl Into<String>, timeout: Duration) -> Self { Self { base: base.into(), client: reqwest::Client::new(), timeout } }
}

#[async_trait::async_trait]
impl ModelProvider for OllamaProvider {
    async fn list_models(&self) -> ModelResult<Vec<String>> {
        let url = format!("{}/api/tags", self.base);
        let resp = self.client
            .get(&url)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| {
                tracing::error!(error = %e, url = %url, "ollama list_models request failed");
                ModelError::Upstream(e.to_string())
            })?;
        
        if !resp.status().is_success() {
            tracing::error!(status = %resp.status(), "ollama returned non-success status");
            return Err(ModelError::Upstream(format!("HTTP {}", resp.status())));
        }
        
        let v: serde_json::Value = resp.json().await.map_err(|e| {
            tracing::error!(error = %e, "failed to parse ollama response");
            ModelError::Upstream(e.to_string())
        })?;
        
        let mut names = Vec::new();
        if let Some(arr) = v.get("models").and_then(|m| m.as_array()) {
            for m in arr {
                if let Some(name) = m.get("name").and_then(|n| n.as_str()) {
                    names.push(name.to_string());
                }
            }
        }
        
        tracing::debug!(count = names.len(), "ollama models retrieved");
        Ok(names)
    }

    async fn chat_stream(&self, req: ChatRequest) -> ModelResult<ChatStream> {
        let url = format!("{}/api/chat", self.base);
        let model = req.model.clone();
        
        // Build Ollama-specific request body
        let ollama_messages: Vec<serde_json::Value> = req.messages
            .iter()
            .map(|m| serde_json::json!({
                "role": m.role,
                "content": m.content,
            }))
            .collect();
        
        let body = serde_json::json!({
            "model": model,
            "messages": ollama_messages,
            "stream": true,
        });
        
        tracing::debug!(model = %model, messages = req.messages.len(), "starting ollama chat stream");
        
        let resp = self.client
            .post(&url)
            .json(&body)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| {
                tracing::error!(error = %e, url = %url, "ollama chat request failed");
                ModelError::Upstream(e.to_string())
            })?;
        
        if !resp.status().is_success() {
            tracing::error!(status = %resp.status(), "ollama chat returned non-success status");
            return Err(ModelError::Upstream(format!("HTTP {}", resp.status())));
        }
        
        let byte_stream = resp.bytes_stream();
        
        let stream = try_stream! {
            use futures_util::StreamExt;
            
            let mut buffer = bytes::BytesMut::new();
            tokio::pin!(byte_stream);
            
            while let Some(chunk) = byte_stream.next().await {
                let bytes = chunk.map_err(|e| ModelError::Upstream(e.to_string()))?;
                buffer.extend_from_slice(&bytes);
                
                // Process complete JSON lines
                while let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
                    let line_bytes = buffer.split_to(newline_pos + 1);
                    let line = String::from_utf8_lossy(&line_bytes[..line_bytes.len()-1]);
                    
                    if line.trim().is_empty() {
                        continue;
                    }
                    
                    let v: serde_json::Value = serde_json::from_str(&line)
                        .map_err(|e| ModelError::Other(format!("JSON parse error: {}", e)))?;
                    
                    let content = v.get("message")
                        .and_then(|m| m.get("content"))
                        .and_then(|c| c.as_str())
                        .unwrap_or("");
                    
                    let done = v.get("done").and_then(|d| d.as_bool()).unwrap_or(false);
                    
                    yield ChatChunk {
                        model: model.clone(),
                        content: content.to_string(),
                        done,
                    };
                    
                    if done {
                        break;
                    }
                }
            }
        };
        
        Ok(Box::pin(stream))
    }
}
