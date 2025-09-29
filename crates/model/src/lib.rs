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
                tracing::error!(error = %e, url = %url, \"ollama list_models request failed\");
                ModelError::Upstream(e.to_string())
            })?;
        
        if !resp.status().is_success() {
            tracing::error!(status = %resp.status(), \"ollama returned non-success status\");
            return Err(ModelError::Upstream(format!(\"HTTP {}\", resp.status())));
        }
        
        let v: serde_json::Value = resp.json().await.map_err(|e| {
            tracing::error!(error = %e, \"failed to parse ollama response\");
            ModelError::Upstream(e.to_string())
        })?;
        
        let mut names = Vec::new();
        if let Some(arr) = v.get(\"models\").and_then(|m| m.as_array()) {
            for m in arr {
                if let Some(name) = m.get(\"name\").and_then(|n| n.as_str()) {
                    names.push(name.to_string());
                }
            }
        }
        
        tracing::debug!(count = names.len(), \"ollama models retrieved\");
        Ok(names)
    }

    async fn chat_stream(&self, req: ChatRequest) -> ModelResult<ChatStream> {
        let url = format!(\"{}/api/chat\", self.base);
        let model = req.model.clone();
        
        // Build Ollama-specific request body\n        let ollama_messages: Vec<serde_json::Value> = req.messages\n            .iter()\n            .map(|m| serde_json::json!({\n                \"role\": m.role,\n                \"content\": m.content,\n            }))\n            .collect();\n        \n        let body = serde_json::json!({\n            \"model\": model,\n            \"messages\": ollama_messages,\n            \"stream\": true,\n        });\n        \n        tracing::debug!(model = %model, messages = req.messages.len(), \"starting ollama chat stream\");\n        \n        let resp = self.client\n            .post(&url)\n            .json(&body)\n            .timeout(self.timeout)\n            .send()\n            .await\n            .map_err(|e| {\n                tracing::error!(error = %e, url = %url, \"ollama chat request failed\");\n                ModelError::Upstream(e.to_string())\n            })?;\n        \n        if !resp.status().is_success() {\n            tracing::error!(status = %resp.status(), \"ollama chat returned non-success status\");\n            return Err(ModelError::Upstream(format!(\"HTTP {}\", resp.status())));\n        }\n        \n        let byte_stream = resp.bytes_stream();\n        \n        let stream = try_stream! {\n            use futures_util::StreamExt;\n            use bytes::Buf;\n            \n            let mut buffer = bytes::BytesMut::new();\n            tokio::pin!(byte_stream);\n            \n            while let Some(chunk) = byte_stream.next().await {\n                let bytes = chunk.map_err(|e| ModelError::Upstream(e.to_string()))?;\n                buffer.extend_from_slice(&bytes);\n                \n                // Process complete JSON lines\n                while let Some(newline_pos) = buffer.iter().position(|&b| b == b'\\n') {\n                    let line_bytes = buffer.split_to(newline_pos + 1);\n                    let line = String::from_utf8_lossy(&line_bytes[..line_bytes.len()-1]);\n                    \n                    if line.trim().is_empty() {\n                        continue;\n                    }\n                    \n                    let v: serde_json::Value = serde_json::from_str(&line)\n                        .map_err(|e| ModelError::Other(format!(\"JSON parse error: {}\", e)))?;\n                    \n                    let content = v.get(\"message\")\n                        .and_then(|m| m.get(\"content\"))\n                        .and_then(|c| c.as_str())\n                        .unwrap_or(\"\");\n                    \n                    let done = v.get(\"done\").and_then(|d| d.as_bool()).unwrap_or(false);\n                    \n                    yield ChatChunk {\n                        model: model.clone(),\n                        content: content.to_string(),\n                        done,\n                    };\n                    \n                    if done {\n                        break;\n                    }\n                }\n            }\n        };\n        \n        Ok(Box::pin(stream))\n    }\n}
