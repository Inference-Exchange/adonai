use serde::{Deserialize, Serialize};

use crate::{
    chat::{ChatCompletionRequest, ChatMessage, ChatProviderRegistry, ChatRole},
    definition::AgentDef,
    error::{AgentError, AgentResult},
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunInput {
    pub goal: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunOutcome {
    pub agent_id: String,
    pub provider: String,
    pub model: String,
    pub final_message: ChatMessage,
}

pub async fn run_once(
    def: &AgentDef,
    registry: &ChatProviderRegistry,
    input: RunInput,
) -> AgentResult<RunOutcome> {
    def.validate()?;
    if input.goal.trim().is_empty() {
        return Err(AgentError::InvalidChatRequest(
            "goal must not be empty".to_owned(),
        ));
    }

    let messages = build_initial_messages(def, &input);

    let request = ChatCompletionRequest {
        provider: def.model.provider.clone(),
        model: def.model.name.clone(),
        messages,
        max_tokens: def.model.max_tokens,
        temperature: def.model.temperature,
    };

    let response = registry.complete(request).await?;

    Ok(RunOutcome {
        agent_id: def.id.0.clone(),
        provider: response.provider,
        model: response.model,
        final_message: response.message,
    })
}

fn build_initial_messages(def: &AgentDef, input: &RunInput) -> Vec<ChatMessage> {
    let mut messages = Vec::with_capacity(2);

    if let Some(system_prompt) = def
        .agent_loop
        .system_prompt
        .as_ref()
        .map(|prompt| prompt.trim())
        .filter(|prompt| !prompt.is_empty())
    {
        messages.push(ChatMessage {
            role: ChatRole::System,
            content: system_prompt.to_owned(),
        });
    }

    messages.push(ChatMessage {
        role: ChatRole::User,
        content: input.goal.clone(),
    });

    messages
}

#[cfg(test)]
mod tests {
    use super::*;

    const NEWS_AGENT_TOML: &str = r#"
id = "news-summariser"
name = "Daily News Summariser"
state_dir = "/tmp/adonai/state/news-summariser"

[model]
provider = "mock"
name = "test-model"

[loop]
kind = "react"
system_prompt = "You are a careful summariser. Quote sources."
max_steps = 4

[[triggers]]
kind = "manual"
"#;

    #[tokio::test]
    async fn run_once_with_mock_returns_assistant_message() {
        let def = AgentDef::from_toml_str(NEWS_AGENT_TOML).unwrap();
        let registry = ChatProviderRegistry::with_default_providers();

        let outcome = run_once(
            &def,
            &registry,
            RunInput {
                goal: "Summarise today's headlines".to_owned(),
            },
        )
        .await
        .unwrap();

        assert_eq!(outcome.agent_id, "news-summariser");
        assert_eq!(outcome.provider, "mock");
        assert_eq!(outcome.final_message.role, ChatRole::Assistant);
        assert!(
            outcome
                .final_message
                .content
                .contains("Summarise today's headlines"),
            "expected mock to echo user goal, got {:?}",
            outcome.final_message.content
        );
    }

    #[test]
    fn build_initial_messages_skips_empty_system_prompt() {
        let toml = r#"
id = "no-prompt"
name = "n"
state_dir = "/tmp/x"

[model]
provider = "mock"
name = "x"

[loop]
kind = "react"
system_prompt = "   "
"#;
        let def = AgentDef::from_toml_str(toml).unwrap();
        let messages = build_initial_messages(
            &def,
            &RunInput {
                goal: "go".to_owned(),
            },
        );

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].role, ChatRole::User);
        assert_eq!(messages[0].content, "go");
    }

    #[tokio::test]
    async fn run_once_rejects_empty_goal() {
        let def = AgentDef::from_toml_str(NEWS_AGENT_TOML).unwrap();
        let registry = ChatProviderRegistry::with_default_providers();

        let error = run_once(
            &def,
            &registry,
            RunInput {
                goal: "   ".to_owned(),
            },
        )
        .await
        .unwrap_err();

        assert!(matches!(error, AgentError::InvalidChatRequest(_)));
    }
}
