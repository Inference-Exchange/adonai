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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub provider: String,
    pub model: String,
    pub message: ChatMessage,
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

        Ok(ChatCompletionResponse {
            provider: self.id().to_owned(),
            model: request.model,
            message: response.message,
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
