use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::{AgentError, AgentResult};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub provider: String,
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub provider: String,
    pub model: String,
    pub message: ChatMessage,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metrics: Option<ChatCompletionMetrics>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionMetrics {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eval_duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokens_per_second: Option<f64>,
}

#[async_trait]
pub trait ChatProvider: Send + Sync {
    fn id(&self) -> &'static str;

    async fn complete(&self, request: ChatCompletionRequest)
    -> AgentResult<ChatCompletionResponse>;
}

#[derive(Clone, Default)]
pub struct ChatProviderRegistry {
    providers: HashMap<String, Arc<dyn ChatProvider>>,
}

impl ChatProviderRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_default_providers() -> Self {
        let mut registry = Self::new();
        registry.register(MockChatProvider);
        registry.register(OllamaChatProvider::default());
        registry
    }

    pub fn register(&mut self, provider: impl ChatProvider + 'static) {
        self.providers
            .insert(provider.id().to_owned(), Arc::new(provider));
    }

    pub fn provider_ids(&self) -> Vec<String> {
        let mut ids = self.providers.keys().cloned().collect::<Vec<_>>();
        ids.sort();
        ids
    }

    pub async fn complete(
        &self,
        request: ChatCompletionRequest,
    ) -> AgentResult<ChatCompletionResponse> {
        validate_chat_request(&request)?;

        let provider = self
            .providers
            .get(&request.provider)
            .ok_or_else(|| AgentError::ChatProviderNotFound(request.provider.clone()))?;

        provider.complete(request).await
    }
}

pub struct MockChatProvider;

#[async_trait]
impl ChatProvider for MockChatProvider {
    fn id(&self) -> &'static str {
        "mock"
    }

    async fn complete(
        &self,
        request: ChatCompletionRequest,
    ) -> AgentResult<ChatCompletionResponse> {
        let last_user_message = request
            .messages
            .iter()
            .rev()
            .find(|message| message.role == ChatRole::User)
            .map(|message| message.content.as_str())
            .unwrap_or("no user message");

        Ok(ChatCompletionResponse {
            provider: self.id().to_owned(),
            model: request.model,
            message: ChatMessage {
                role: ChatRole::Assistant,
                content: format!("mock response: {last_user_message}"),
            },
            metrics: None,
        })
    }
}

#[derive(Clone)]
pub struct OllamaChatProvider {
    base_url: String,
    client: reqwest::Client,
}

impl Default for OllamaChatProvider {
    fn default() -> Self {
        Self {
            base_url: "http://127.0.0.1:11434".to_owned(),
            client: reqwest::Client::new(),
        }
    }
}

impl OllamaChatProvider {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl ChatProvider for OllamaChatProvider {
    fn id(&self) -> &'static str {
        "ollama"
    }

    async fn complete(
        &self,
        request: ChatCompletionRequest,
    ) -> AgentResult<ChatCompletionResponse> {
        let url = format!("{}/api/chat", self.base_url.trim_end_matches('/'));
        let ollama_request = OllamaChatRequest::from_chat_request(&request);
        let response = self
            .client
            .post(url)
            .json(&ollama_request)
            .send()
            .await
            .map_err(|source| AgentError::ChatProviderRequest {
                provider: self.id().to_owned(),
                source,
            })?
            .error_for_status()
            .map_err(|source| AgentError::ChatProviderRequest {
                provider: self.id().to_owned(),
                source,
            })?
            .json::<OllamaChatResponse>()
            .await
            .map_err(|source| AgentError::ChatProviderRequest {
                provider: self.id().to_owned(),
                source,
            })?;

        let metrics = response.metrics();

        Ok(ChatCompletionResponse {
            provider: self.id().to_owned(),
            model: request.model,
            message: response.message,
            metrics,
        })
    }
}

#[derive(Debug, Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
}

impl OllamaChatRequest {
    fn from_chat_request(request: &ChatCompletionRequest) -> Self {
        let options = match (request.temperature, request.max_tokens) {
            (None, None) => None,
            (temperature, max_tokens) => Some(OllamaOptions {
                temperature,
                num_predict: max_tokens,
            }),
        };

        Self {
            model: request.model.clone(),
            messages: request.messages.clone(),
            stream: false,
            options,
        }
    }
}

#[derive(Debug, Serialize)]
struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    message: ChatMessage,
    #[serde(default)]
    total_duration: Option<u64>,
    #[serde(default)]
    eval_count: Option<u32>,
    #[serde(default)]
    eval_duration: Option<u64>,
}

impl OllamaChatResponse {
    fn metrics(&self) -> Option<ChatCompletionMetrics> {
        let total_duration_ms = self.total_duration.map(nanos_to_millis);
        let eval_duration_ms = self.eval_duration.map(nanos_to_millis);
        let tokens_per_second = match (self.eval_count, self.eval_duration) {
            (Some(tokens), Some(duration_ns)) if duration_ns > 0 => {
                Some(tokens as f64 / (duration_ns as f64 / 1_000_000_000.0))
            }
            _ => None,
        };

        if self.eval_count.is_none()
            && total_duration_ms.is_none()
            && eval_duration_ms.is_none()
            && tokens_per_second.is_none()
        {
            return None;
        }

        Some(ChatCompletionMetrics {
            output_tokens: self.eval_count,
            total_duration_ms,
            eval_duration_ms,
            tokens_per_second,
        })
    }
}

fn nanos_to_millis(value: u64) -> u64 {
    value / 1_000_000
}

fn validate_chat_request(request: &ChatCompletionRequest) -> AgentResult<()> {
    if request.provider.trim().is_empty() {
        return Err(AgentError::InvalidChatRequest(
            "provider must not be empty".to_owned(),
        ));
    }
    if request.model.trim().is_empty() {
        return Err(AgentError::InvalidChatRequest(
            "model must not be empty".to_owned(),
        ));
    }
    if request.messages.is_empty() {
        return Err(AgentError::InvalidChatRequest(
            "messages must not be empty".to_owned(),
        ));
    }
    if request
        .messages
        .iter()
        .any(|message| message.content.trim().is_empty())
    {
        return Err(AgentError::InvalidChatRequest(
            "message content must not be empty".to_owned(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(provider: &str) -> ChatCompletionRequest {
        ChatCompletionRequest {
            provider: provider.to_owned(),
            model: "test-model".to_owned(),
            messages: vec![ChatMessage {
                role: ChatRole::User,
                content: "hello".to_owned(),
            }],
            max_tokens: None,
            temperature: None,
        }
    }

    #[tokio::test]
    async fn registry_routes_to_mock_provider() {
        let registry = ChatProviderRegistry::with_default_providers();
        let response = registry.complete(request("mock")).await.unwrap();

        assert_eq!(response.provider, "mock");
        assert_eq!(response.model, "test-model");
        assert_eq!(response.message.role, ChatRole::Assistant);
        assert_eq!(response.message.content, "mock response: hello");
        assert_eq!(response.metrics, None);
    }

    #[test]
    fn ollama_response_reports_tokens_per_second() {
        let response = OllamaChatResponse {
            message: ChatMessage {
                role: ChatRole::Assistant,
                content: "done".to_owned(),
            },
            total_duration: Some(3_000_000_000),
            eval_count: Some(100),
            eval_duration: Some(2_000_000_000),
        };

        let metrics = response.metrics().expect("expected metrics");

        assert_eq!(metrics.output_tokens, Some(100));
        assert_eq!(metrics.total_duration_ms, Some(3000));
        assert_eq!(metrics.eval_duration_ms, Some(2000));
        assert_eq!(metrics.tokens_per_second, Some(50.0));
    }

    #[test]
    fn registry_lists_provider_ids() {
        let registry = ChatProviderRegistry::with_default_providers();

        assert_eq!(registry.provider_ids(), vec!["mock", "ollama"]);
    }

    #[tokio::test]
    async fn registry_rejects_unknown_provider() {
        let registry = ChatProviderRegistry::with_default_providers();
        let result = registry.complete(request("missing")).await;

        assert!(matches!(result, Err(AgentError::ChatProviderNotFound(_))));
    }

    #[tokio::test]
    async fn registry_rejects_empty_messages() {
        let registry = ChatProviderRegistry::with_default_providers();
        let mut request = request("mock");
        request.messages = Vec::new();
        let result = registry.complete(request).await;

        assert!(matches!(result, Err(AgentError::InvalidChatRequest(_))));
    }
}
