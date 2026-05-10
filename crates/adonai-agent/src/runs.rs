use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::{
    chat::ChatMessage,
    definition::AgentDef,
    error::{AgentError, AgentResult},
};

static RUN_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Running,
    Succeeded,
    Failed,
}

impl RunStatus {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
        }
    }

    fn from_str(value: &str) -> AgentResult<Self> {
        match value {
            "running" => Ok(Self::Running),
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            unknown => Err(AgentError::InvalidDefinition(format!(
                "unknown run status `{unknown}`"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentRunRecord {
    pub id: String,
    pub agent_id: String,
    pub agent_name: String,
    pub goal: String,
    pub status: RunStatus,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub final_message: Option<ChatMessage>,
    pub error: Option<String>,
    pub created_at_ms: u128,
    pub updated_at_ms: u128,
}

#[derive(Clone)]
pub struct RunStore {
    db_path: PathBuf,
}

impl RunStore {
    pub fn open(db_path: impl Into<PathBuf>) -> AgentResult<Self> {
        let db_path = db_path.into();
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent).map_err(|source| AgentError::DefinitionRead {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        let store = Self { db_path };
        store.init()?;
        Ok(store)
    }

    pub fn create_run(&self, agent: &AgentDef, goal: &str) -> AgentResult<AgentRunRecord> {
        let now = now_ms();
        let record = AgentRunRecord {
            id: new_run_id(now),
            agent_id: agent.id.0.clone(),
            agent_name: agent.name.clone(),
            goal: goal.to_owned(),
            status: RunStatus::Running,
            provider: None,
            model: None,
            final_message: None,
            error: None,
            created_at_ms: now,
            updated_at_ms: now,
        };

        let conn = self.connection_for_write()?;
        conn.execute(
            "INSERT INTO agent_runs (
                id, agent_id, agent_name, goal, status, provider, model,
                final_message_json, error, created_at_ms, updated_at_ms
            ) VALUES (?1, ?2, ?3, ?4, ?5, NULL, NULL, NULL, NULL, ?6, ?7)",
            params![
                record.id,
                record.agent_id,
                record.agent_name,
                record.goal,
                record.status.as_str(),
                record.created_at_ms.to_string(),
                record.updated_at_ms.to_string(),
            ],
        )
        .map_err(|source| AgentError::RunStoreWrite {
            path: self.db_path.clone(),
            source,
        })?;

        Ok(record)
    }

    pub fn mark_succeeded(
        &self,
        run_id: &str,
        provider: &str,
        model: &str,
        final_message: &ChatMessage,
    ) -> AgentResult<AgentRunRecord> {
        let updated_at_ms = now_ms();
        let final_message_json = serde_json::to_string(final_message)
            .map_err(|error| AgentError::InvalidDefinition(error.to_string()))?;
        let conn = self.connection_for_write()?;
        conn.execute(
            "UPDATE agent_runs
             SET status = ?1, provider = ?2, model = ?3, final_message_json = ?4, error = NULL, updated_at_ms = ?5
             WHERE id = ?6",
            params![
                RunStatus::Succeeded.as_str(),
                provider,
                model,
                final_message_json,
                updated_at_ms.to_string(),
                run_id,
            ],
        )
        .map_err(|source| AgentError::RunStoreWrite {
            path: self.db_path.clone(),
            source,
        })?;

        self.get_run(run_id)
    }

    pub fn mark_failed(&self, run_id: &str, error: &str) -> AgentResult<AgentRunRecord> {
        let updated_at_ms = now_ms();
        let conn = self.connection_for_write()?;
        conn.execute(
            "UPDATE agent_runs
             SET status = ?1, error = ?2, updated_at_ms = ?3
             WHERE id = ?4",
            params![
                RunStatus::Failed.as_str(),
                error,
                updated_at_ms.to_string(),
                run_id,
            ],
        )
        .map_err(|source| AgentError::RunStoreWrite {
            path: self.db_path.clone(),
            source,
        })?;

        self.get_run(run_id)
    }

    pub fn list_runs(&self, limit: u32) -> AgentResult<Vec<AgentRunRecord>> {
        let limit = limit.clamp(1, 100);
        let conn = self.connection_for_read()?;
        let mut statement = conn
            .prepare(
                "SELECT id, agent_id, agent_name, goal, status, provider, model,
                        final_message_json, error, created_at_ms, updated_at_ms
                 FROM agent_runs
                 ORDER BY created_at_ms DESC
                 LIMIT ?1",
            )
            .map_err(|source| AgentError::RunStoreRead {
                path: self.db_path.clone(),
                source,
            })?;

        let rows = statement
            .query_map(params![limit], row_to_record)
            .map_err(|source| AgentError::RunStoreRead {
                path: self.db_path.clone(),
                source,
            })?;

        let mut records = Vec::new();
        for row in rows {
            records.push(row.map_err(|source| AgentError::RunStoreRead {
                path: self.db_path.clone(),
                source,
            })?);
        }

        Ok(records)
    }

    pub fn get_run(&self, run_id: &str) -> AgentResult<AgentRunRecord> {
        let conn = self.connection_for_read()?;
        let record = conn
            .query_row(
                "SELECT id, agent_id, agent_name, goal, status, provider, model,
                        final_message_json, error, created_at_ms, updated_at_ms
                 FROM agent_runs
                 WHERE id = ?1",
                params![run_id],
                row_to_record,
            )
            .optional()
            .map_err(|source| AgentError::RunStoreRead {
                path: self.db_path.clone(),
                source,
            })?
            .ok_or_else(|| AgentError::RunNotFound(run_id.to_owned()))?;

        Ok(record)
    }

    fn init(&self) -> AgentResult<()> {
        let conn = self.connection_for_open()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS agent_runs (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                agent_name TEXT NOT NULL,
                goal TEXT NOT NULL,
                status TEXT NOT NULL,
                provider TEXT,
                model TEXT,
                final_message_json TEXT,
                error TEXT,
                created_at_ms TEXT NOT NULL,
                updated_at_ms TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_agent_runs_created_at
            ON agent_runs(created_at_ms DESC);",
        )
        .map_err(|source| AgentError::RunStoreInit {
            path: self.db_path.clone(),
            source,
        })?;
        Ok(())
    }

    fn connection_for_open(&self) -> AgentResult<Connection> {
        Connection::open(&self.db_path).map_err(|source| AgentError::RunStoreOpen {
            path: self.db_path.clone(),
            source,
        })
    }

    fn connection_for_read(&self) -> AgentResult<Connection> {
        self.connection_for_open()
    }

    fn connection_for_write(&self) -> AgentResult<Connection> {
        self.connection_for_open()
    }
}

fn row_to_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<AgentRunRecord> {
    let final_message_json: Option<String> = row.get(7)?;
    let final_message = final_message_json
        .as_deref()
        .and_then(|json| serde_json::from_str::<ChatMessage>(json).ok());
    let status_raw: String = row.get(4)?;
    let status = RunStatus::from_str(&status_raw)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    let created_at_ms: String = row.get(9)?;
    let updated_at_ms: String = row.get(10)?;

    Ok(AgentRunRecord {
        id: row.get(0)?,
        agent_id: row.get(1)?,
        agent_name: row.get(2)?,
        goal: row.get(3)?,
        status,
        provider: row.get(5)?,
        model: row.get(6)?,
        final_message,
        error: row.get(8)?,
        created_at_ms: created_at_ms.parse::<u128>().unwrap_or_default(),
        updated_at_ms: updated_at_ms.parse::<u128>().unwrap_or_default(),
    })
}

fn new_run_id(now: u128) -> String {
    let sequence = RUN_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("run_{now}_{sequence}")
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent() -> AgentDef {
        AgentDef::from_toml_str(
            r#"
id = "operator-smoke"
name = "Operator Smoke Test"
state_dir = "/tmp/adonai/state/operator-smoke"

[model]
provider = "mock"
name = "test-model"

[loop]
kind = "react"
"#,
        )
        .unwrap()
    }

    #[test]
    fn store_persists_run_lifecycle() {
        let path = std::env::temp_dir().join(format!("adonai-test-{}.db", now_ms()));
        let store = RunStore::open(&path).unwrap();
        let run = store.create_run(&agent(), "Report status").unwrap();

        assert_eq!(run.status, RunStatus::Running);

        let final_message = ChatMessage {
            role: crate::chat::ChatRole::Assistant,
            content: "done".to_owned(),
        };
        let updated = store
            .mark_succeeded(&run.id, "mock", "test-model", &final_message)
            .unwrap();

        assert_eq!(updated.status, RunStatus::Succeeded);
        assert_eq!(updated.final_message, Some(final_message));
        assert_eq!(store.list_runs(10).unwrap().len(), 1);
        assert_eq!(store.get_run(&run.id).unwrap().id, run.id);

        let _ = fs::remove_file(path);
    }
}
