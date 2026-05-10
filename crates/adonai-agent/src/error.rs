use std::path::PathBuf;

use thiserror::Error;

pub type AgentResult<T> = Result<T, AgentError>;

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("agent definition file not found: {0}")]
    DefinitionNotFound(PathBuf),

    #[error("failed to read agent definition {path}: {source}")]
    DefinitionRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse agent definition {path}: {source}")]
    DefinitionParse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("invalid agent definition: {0}")]
    InvalidDefinition(String),

    #[error("chat provider not found: {0}")]
    ChatProviderNotFound(String),

    #[error("invalid chat request: {0}")]
    InvalidChatRequest(String),

    #[error("chat provider {provider} request failed: {source}")]
    ChatProviderRequest {
        provider: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("failed to open run store {path}: {source}")]
    RunStoreOpen {
        path: PathBuf,
        #[source]
        source: rusqlite::Error,
    },

    #[error("failed to initialize run store {path}: {source}")]
    RunStoreInit {
        path: PathBuf,
        #[source]
        source: rusqlite::Error,
    },

    #[error("failed to write run store {path}: {source}")]
    RunStoreWrite {
        path: PathBuf,
        #[source]
        source: rusqlite::Error,
    },

    #[error("failed to read run store {path}: {source}")]
    RunStoreRead {
        path: PathBuf,
        #[source]
        source: rusqlite::Error,
    },

    #[error("agent run not found: {0}")]
    RunNotFound(String),
}
