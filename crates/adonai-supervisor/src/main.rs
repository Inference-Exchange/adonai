use std::{net::IpAddr, path::PathBuf, str::FromStr};

use adonai_agent::{
    AgentDef, AgentError, AgentRunRecord, ChatCompletionRequest, ChatCompletionResponse,
    ChatProviderRegistry, RunInput, RunStore, run_once,
};
use adonai_core::{
    BindAddress, EndpointExposure, EndpointPolicy, ModelPlanRequest, ModelRunPlan,
    SupervisorSnapshot, plan_model_run,
};
use axum::{
    Json, Router,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt};

const DEFAULT_PORT: u16 = 49231;

#[derive(Clone)]
struct AppState {
    endpoint_policy: EndpointPolicy,
    chat_providers: ChatProviderRegistry,
    run_store: RunStore,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    product: String,
    status: String,
}

#[derive(Debug, Serialize)]
struct ApiErrorBody {
    error: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();

    let endpoint_policy = endpoint_policy_from_args(std::env::args().skip(1))?;
    if endpoint_policy.exposure != EndpointExposure::LoopbackOnly {
        info!(
            exposure = ?endpoint_policy.exposure,
            "Adonai supervisor started with explicit non-loopback exposure"
        );
    }

    let bind = endpoint_policy.bind.socket_addr();
    let state = AppState {
        endpoint_policy,
        chat_providers: ChatProviderRegistry::with_default_providers(),
        run_store: RunStore::open(run_store_path())?,
    };
    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/status", get(status))
        .route("/v1/hardware", get(hardware))
        .route("/v1/engines", get(engines))
        .route("/v1/models/plan", post(plan_model))
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/agents/runs", post(run_agent).get(list_runs))
        .route("/v1/agents/runs/{run_id}", get(get_run))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = TcpListener::bind(bind).await?;
    info!(address = %bind, "Adonai supervisor listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).init();
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        product: "Adonai".to_owned(),
        status: "ready".to_owned(),
    })
}

async fn status(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Json<SupervisorSnapshot> {
    Json(SupervisorSnapshot::collect(state.endpoint_policy))
}

async fn hardware(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Json<adonai_core::HardwareProfile> {
    Json(SupervisorSnapshot::collect(state.endpoint_policy).hardware)
}

async fn engines(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Json<adonai_core::EngineProbe> {
    Json(SupervisorSnapshot::collect(state.endpoint_policy).engines)
}

async fn plan_model(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(request): Json<ModelPlanRequest>,
) -> Json<ModelRunPlan> {
    let snapshot = SupervisorSnapshot::collect(state.endpoint_policy);
    Json(plan_model_run(request, &snapshot.engines))
}

async fn chat_completions(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(request): Json<ChatCompletionRequest>,
) -> Result<Json<ChatCompletionResponse>, ApiError> {
    let response = state.chat_providers.complete(request).await?;

    Ok(Json(response))
}

#[derive(Debug, Deserialize)]
struct RunAgentRequest {
    agent: AgentDef,
    goal: String,
}

async fn run_agent(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(request): Json<RunAgentRequest>,
) -> Result<Json<AgentRunRecord>, ApiError> {
    let run = state.run_store.create_run(&request.agent, &request.goal)?;
    let outcome = match run_once(
        &request.agent,
        &state.chat_providers,
        RunInput { goal: request.goal },
    )
    .await
    {
        Ok(outcome) => outcome,
        Err(error) => {
            let failed = state.run_store.mark_failed(&run.id, &error.to_string())?;
            return Ok(Json(failed));
        }
    };

    let record = state.run_store.mark_succeeded(
        &run.id,
        &outcome.provider,
        &outcome.model,
        &outcome.final_message,
        outcome.metrics.as_ref(),
    )?;

    Ok(Json(record))
}

#[derive(Debug, Deserialize)]
struct ListRunsQuery {
    limit: Option<u32>,
}

async fn list_runs(
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::extract::Query(query): axum::extract::Query<ListRunsQuery>,
) -> Result<Json<Vec<AgentRunRecord>>, ApiError> {
    Ok(Json(state.run_store.list_runs(query.limit.unwrap_or(25))?))
}

async fn get_run(
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::extract::Path(run_id): axum::extract::Path<String>,
) -> Result<Json<AgentRunRecord>, ApiError> {
    Ok(Json(state.run_store.get_run(&run_id)?))
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    message: String,
}

impl From<AgentError> for ApiError {
    fn from(error: AgentError) -> Self {
        let status = match error {
            AgentError::ChatProviderNotFound(_) => StatusCode::NOT_FOUND,
            AgentError::RunNotFound(_) => StatusCode::NOT_FOUND,
            AgentError::InvalidChatRequest(_) => StatusCode::BAD_REQUEST,
            AgentError::ChatProviderRequest { .. } => StatusCode::BAD_GATEWAY,
            AgentError::RunStoreOpen { .. }
            | AgentError::RunStoreInit { .. }
            | AgentError::RunStoreWrite { .. }
            | AgentError::RunStoreRead { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            AgentError::DefinitionNotFound(_)
            | AgentError::DefinitionRead { .. }
            | AgentError::DefinitionParse { .. }
            | AgentError::InvalidDefinition(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        Self {
            status,
            message: error.to_string(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ApiErrorBody {
                error: self.message,
            }),
        )
            .into_response()
    }
}

fn endpoint_policy_from_args(
    args: impl IntoIterator<Item = String>,
) -> Result<EndpointPolicy, String> {
    let mut host = IpAddr::from([127, 0, 0, 1]);
    let mut port = DEFAULT_PORT;
    let mut allow_lan = false;
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--host" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--host requires an IP address".to_owned())?;
                host = IpAddr::from_str(&value)
                    .map_err(|error| format!("invalid --host value `{value}`: {error}"))?;
            }
            "--port" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--port requires a numeric port".to_owned())?;
                port = value
                    .parse::<u16>()
                    .map_err(|error| format!("invalid --port value `{value}`: {error}"))?;
            }
            "--allow-lan" => {
                allow_lan = true;
            }
            "--help" | "-h" => {
                return Err(help_text());
            }
            unknown => {
                return Err(format!(
                    "{unknown} is not a supported Adonai supervisor argument"
                ));
            }
        }
    }

    if !host.is_loopback() && !allow_lan {
        return Err(
            "non-loopback host requires --allow-lan so network exposure is explicit".to_owned(),
        );
    }

    Ok(EndpointPolicy::from_bind(BindAddress { host, port }))
}

fn help_text() -> String {
    [
        "Adonai supervisor",
        "",
        "Options:",
        "  --host <ip>      Bind IP address. Defaults to 127.0.0.1.",
        "  --port <port>    Bind port. Defaults to 49231.",
        "  --allow-lan      Required when --host is not loopback.",
    ]
    .join("\n")
}

async fn shutdown_signal() {
    if tokio::signal::ctrl_c().await.is_err() {
        return;
    }

    info!("shutdown signal received");
}

fn run_store_path() -> PathBuf {
    if let Ok(path) = std::env::var("ADONAI_RUN_DB") {
        return PathBuf::from(path);
    }

    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_owned());
    PathBuf::from(home).join(".adonai/state/runs.db")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_is_loopback_only() {
        let policy = endpoint_policy_from_args(Vec::<String>::new()).unwrap();

        assert_eq!(policy.exposure, EndpointExposure::LoopbackOnly);
        assert_eq!(policy.bind.socket_addr().to_string(), "127.0.0.1:49231");
    }

    #[test]
    fn lan_bind_requires_explicit_flag() {
        let result = endpoint_policy_from_args(["--host", "0.0.0.0"].map(str::to_owned));

        assert!(result.is_err());
    }

    #[test]
    fn lan_bind_is_allowed_when_explicit() {
        let policy =
            endpoint_policy_from_args(["--host", "0.0.0.0", "--allow-lan"].map(str::to_owned))
                .unwrap();

        assert_eq!(policy.exposure, EndpointExposure::LanExplicit);
        assert_eq!(policy.bind.socket_addr().to_string(), "0.0.0.0:49231");
    }

    #[tokio::test]
    async fn run_agent_routes_through_runtime() {
        let db_path = std::env::temp_dir().join("adonai-supervisor-run-agent-test.db");
        let _ = std::fs::remove_file(&db_path);
        let state = AppState {
            endpoint_policy: EndpointPolicy::from_bind(BindAddress {
                host: IpAddr::from([127, 0, 0, 1]),
                port: DEFAULT_PORT,
            }),
            chat_providers: ChatProviderRegistry::with_default_providers(),
            run_store: RunStore::open(&db_path).unwrap(),
        };
        let agent = AgentDef::from_toml_str(
            r#"
id = "operator-smoke"
name = "Operator Smoke Test"
state_dir = "/tmp/adonai/state/operator-smoke"

[model]
provider = "mock"
name = "test-model"

[loop]
kind = "react"
system_prompt = "You are terse."
max_steps = 1

[[triggers]]
kind = "manual"
"#,
        )
        .unwrap();

        let response = run_agent(
            axum::extract::State(state),
            Json(RunAgentRequest {
                agent,
                goal: "Report status".to_owned(),
            }),
        )
        .await
        .unwrap();

        assert_eq!(response.0.agent_id, "operator-smoke");
        assert_eq!(response.0.provider.as_deref(), Some("mock"));
        assert_eq!(response.0.status, adonai_agent::RunStatus::Succeeded);
        assert!(
            response
                .0
                .final_message
                .as_ref()
                .is_some_and(|message| message.content.contains("Report status"))
        );

        let _ = std::fs::remove_file(db_path);
    }
}
