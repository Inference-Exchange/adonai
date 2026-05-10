use std::{fs, path::Path};

use serde::{Deserialize, Serialize};

use crate::error::{AgentError, AgentResult};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AgentId(pub String);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentDef {
    pub id: AgentId,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub model: ModelRef,
    #[serde(rename = "loop")]
    pub agent_loop: LoopSpec,
    #[serde(default)]
    pub tools: Vec<ToolRef>,
    #[serde(default)]
    pub triggers: Vec<Trigger>,
    pub state_dir: String,
    #[serde(default)]
    pub secrets: Vec<SecretRef>,
    #[serde(default)]
    pub resources: ResourceLimits,
    #[serde(default)]
    pub lifecycle: LifecycleHandlers,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelRef {
    pub provider: String,
    pub name: String,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoopSpec {
    pub kind: LoopKind,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub max_steps: Option<u32>,
    #[serde(default)]
    pub spec_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LoopKind {
    React,
    Graph,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolRef {
    pub name: String,
    pub kind: ToolKind,
    #[serde(default)]
    pub config: toml::Table,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ToolKind {
    HttpFetch,
    Mcp,
    Builtin,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trigger {
    pub kind: TriggerKind,
    #[serde(default)]
    pub cron: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TriggerKind {
    Manual,
    Cron,
    Webhook,
    FileWatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretRef {
    pub name: String,
    pub keychain_key: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub cpu_pct: Option<u8>,
    pub ram_mb: Option<u32>,
    pub gpu: Option<GpuShare>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GpuShare {
    Shared,
    Exclusive,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecycleHandlers {
    pub on_start: Option<String>,
    pub on_stop: Option<String>,
    #[serde(default)]
    pub on_crash: CrashAction,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CrashAction {
    #[default]
    Restart,
    Halt,
    Backoff(RestartBackoff),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestartBackoff {
    pub strategy: BackoffStrategy,
    pub max_attempts: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackoffStrategy {
    Exponential,
    Linear,
}

impl AgentDef {
    pub fn from_toml_str(source: &str) -> AgentResult<Self> {
        let def: Self = toml::from_str(source).map_err(|source| AgentError::DefinitionParse {
            path: "<inline>".into(),
            source,
        })?;
        def.validate()?;
        Ok(def)
    }

    pub fn from_path(path: &Path) -> AgentResult<Self> {
        if !path.exists() {
            return Err(AgentError::DefinitionNotFound(path.to_owned()));
        }

        let source = fs::read_to_string(path).map_err(|source| AgentError::DefinitionRead {
            path: path.to_owned(),
            source,
        })?;

        let def: Self = toml::from_str(&source).map_err(|source| AgentError::DefinitionParse {
            path: path.to_owned(),
            source,
        })?;

        def.validate()?;
        Ok(def)
    }

    pub fn validate(&self) -> AgentResult<()> {
        if self.id.0.trim().is_empty() {
            return Err(AgentError::InvalidDefinition(
                "agent id must not be empty".into(),
            ));
        }
        if self.name.trim().is_empty() {
            return Err(AgentError::InvalidDefinition(
                "agent name must not be empty".into(),
            ));
        }
        if self.model.provider.trim().is_empty() || self.model.name.trim().is_empty() {
            return Err(AgentError::InvalidDefinition(
                "model.provider and model.name are required".into(),
            ));
        }
        if self.state_dir.trim().is_empty() {
            return Err(AgentError::InvalidDefinition(
                "state_dir must not be empty".into(),
            ));
        }
        for trigger in &self.triggers {
            if matches!(trigger.kind, TriggerKind::Cron) && trigger.cron.is_none() {
                return Err(AgentError::InvalidDefinition(
                    "cron trigger requires a cron expression".into(),
                ));
            }
            if matches!(trigger.kind, TriggerKind::Webhook | TriggerKind::FileWatch)
                && trigger.path.is_none()
            {
                return Err(AgentError::InvalidDefinition(
                    "webhook and file-watch triggers require a path".into(),
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NEWS_SUMMARISER_TOML: &str = r#"
id = "news-summariser"
name = "Daily News Summariser"
description = "Pull the latest stories from a feed and produce a daily digest."
state_dir = "~/.adonai/state/news-summariser"

[model]
provider = "ollama"
name = "llama3.2:3b"
max_tokens = 1024
temperature = 0.2

[loop]
kind = "react"
system_prompt = "You are a careful summariser. Quote sources."
max_steps = 8

[[tools]]
name = "fetch"
kind = "http-fetch"

[[triggers]]
kind = "cron"
cron = "0 7 * * *"

[[triggers]]
kind = "manual"

[resources]
cpu_pct = 50
ram_mb = 4096

[lifecycle]
on_crash = "restart"
"#;

    #[test]
    fn parses_news_summariser_template() {
        let def = AgentDef::from_toml_str(NEWS_SUMMARISER_TOML).expect("parses");

        assert_eq!(def.id.0, "news-summariser");
        assert_eq!(def.name, "Daily News Summariser");
        assert_eq!(def.model.provider, "ollama");
        assert_eq!(def.model.name, "llama3.2:3b");
        assert_eq!(def.agent_loop.kind, LoopKind::React);
        assert_eq!(def.agent_loop.max_steps, Some(8));
        assert_eq!(def.tools.len(), 1);
        assert_eq!(def.tools[0].kind, ToolKind::HttpFetch);
        assert_eq!(def.triggers.len(), 2);
        assert!(matches!(def.triggers[0].kind, TriggerKind::Cron));
        assert_eq!(def.triggers[0].cron.as_deref(), Some("0 7 * * *"));
        assert!(matches!(def.triggers[1].kind, TriggerKind::Manual));
        assert_eq!(def.resources.cpu_pct, Some(50));
        assert!(matches!(def.lifecycle.on_crash, CrashAction::Restart));
    }

    #[test]
    fn rejects_empty_id() {
        let invalid = r#"
id = ""
name = "x"
state_dir = "/tmp/x"

[model]
provider = "ollama"
name = "llama3.2:3b"

[loop]
kind = "react"
"#;
        let result = AgentDef::from_toml_str(invalid);
        assert!(matches!(result, Err(AgentError::InvalidDefinition(_))));
    }

    #[test]
    fn rejects_cron_trigger_without_expression() {
        let invalid = r#"
id = "x"
name = "x"
state_dir = "/tmp/x"

[model]
provider = "ollama"
name = "llama3.2:3b"

[loop]
kind = "react"

[[triggers]]
kind = "cron"
"#;
        let result = AgentDef::from_toml_str(invalid);
        assert!(matches!(result, Err(AgentError::InvalidDefinition(_))));
    }

    #[test]
    fn round_trips_through_toml() {
        let def = AgentDef::from_toml_str(NEWS_SUMMARISER_TOML).unwrap();
        let serialised = toml::to_string(&def).expect("serialises");
        let reparsed = AgentDef::from_toml_str(&serialised).expect("re-parses");
        assert_eq!(def, reparsed);
    }
}
