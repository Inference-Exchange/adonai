use std::{env, path::PathBuf};

use adonai_agent::{
    AgentDef, AgentError, AgentRunRecord, ChatProviderRegistry, RunInput, RunStore, run_once,
};
use adonai_core::{
    BindAddress, EndpointPolicy, ModelArtifact, ModelPlanRequest, ModelRunPlan, SupervisorSnapshot,
    plan_model_run,
};
use thiserror::Error;

const DEFAULT_PORT: u16 = 49231;
const DEFAULT_MODEL: &str = "llama3.2:3b";

#[derive(Debug, Error)]
pub enum CliError {
    #[error("{0}")]
    Usage(String),

    #[error(transparent)]
    Agent(#[from] AgentError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Command {
    Help,
    Run,
    Up,
    Status,
    Doctor,
    Prepare,
    RunProof,
    Report,
}

pub async fn run_cli(args: impl IntoIterator<Item = String>) -> Result<String, CliError> {
    let command = parse_command(args)?;
    if command == Command::Help {
        return Ok(help_text());
    }

    let context = RuntimeContext::collect()?;

    match command {
        Command::Help => Ok(help_text()),
        Command::Run => run(&context).await,
        Command::Up => up(&context).await,
        Command::Status => Ok(status(&context)),
        Command::Doctor => Ok(doctor(&context)),
        Command::Prepare => Ok(prepare(&context)),
        Command::RunProof => run_proof(&context).await.map(format_proof),
        Command::Report => Ok(report(&context)),
    }
}

fn parse_command(args: impl IntoIterator<Item = String>) -> Result<Command, CliError> {
    let args = args.into_iter().collect::<Vec<_>>();

    match args.as_slice() {
        [] => Ok(Command::Up),
        [first] if first == "run" => Ok(Command::Run),
        [first] if first == "up" => Ok(Command::Up),
        [first] if first == "status" => Ok(Command::Status),
        [first] if first == "doctor" => Ok(Command::Doctor),
        [first] if first == "prepare" => Ok(Command::Prepare),
        [first] if first == "report" => Ok(Command::Report),
        [first, second] if first == "run" && second == "proof" => Ok(Command::RunProof),
        [first] if first == "--help" || first == "-h" || first == "help" => Ok(Command::Help),
        _ => Err(CliError::Usage(help_text())),
    }
}

struct RuntimeContext {
    snapshot: SupervisorSnapshot,
    model_plan: ModelRunPlan,
    run_store: RunStore,
}

impl RuntimeContext {
    fn collect() -> Result<Self, CliError> {
        let endpoint_policy = EndpointPolicy::from_bind(BindAddress::loopback(DEFAULT_PORT));
        let snapshot = SupervisorSnapshot::collect(endpoint_policy);
        let model = match env::var("ADONAI_STARTER_MODEL") {
            Ok(model) => model,
            Err(_) => DEFAULT_MODEL.to_owned(),
        };
        let model_plan = plan_model_run(
            ModelPlanRequest {
                model,
                source: None,
                artifact: None,
            },
            &snapshot.engines,
        );
        let run_store = RunStore::open(run_store_path())?;

        Ok(Self {
            snapshot,
            model_plan,
            run_store,
        })
    }
}

async fn run(context: &RuntimeContext) -> Result<String, CliError> {
    let mut lines = vec![
        "Adonai run".to_owned(),
        "The fastest OS to run your own local models.".to_owned(),
        String::new(),
        status(context),
        String::new(),
        doctor(context),
    ];

    if context.model_plan.runnable_now {
        lines.push(String::new());
        lines.push(format_proof(run_proof(context).await?));
    } else {
        lines.push(String::new());
        lines.push("Local generation is not ready yet.".to_owned());
        lines.push("Run `adonai prepare` for the next setup action.".to_owned());
    }

    Ok(lines.join("\n"))
}

async fn up(context: &RuntimeContext) -> Result<String, CliError> {
    let mut lines = vec![
        "Adonai up".to_owned(),
        "The fastest OS to run your own local models.".to_owned(),
        String::new(),
        status(context),
        String::new(),
        doctor(context),
    ];

    if context.model_plan.runnable_now {
        lines.push(String::new());
        lines.push(format_proof(run_proof(context).await?));
    } else {
        lines.push(String::new());
        lines.push("Proof run skipped: model is not runnable yet.".to_owned());
        lines.push("Run `adonai prepare` for the next setup action.".to_owned());
    }

    Ok(lines.join("\n"))
}

fn status(context: &RuntimeContext) -> String {
    let snapshot = &context.snapshot;
    let hardware = &snapshot.hardware;
    let platform = &hardware.platform;
    let endpoint = &snapshot.endpoint_policy.bind;
    let recent_run_count = match context.run_store.list_runs(10) {
        Ok(runs) => runs.len().to_string(),
        Err(error) => format!("unavailable ({error})"),
    };

    [
        "Status".to_owned(),
        "CLI runtime: ready".to_owned(),
        format!("Version: {}", snapshot.version),
        format!(
            "Machine: {} {}, {}, {} GB",
            platform.os,
            platform.architecture,
            hardware.cpu_brand,
            bytes_to_gb(hardware.memory.total_bytes)
        ),
        format!(
            "Accelerators: {}",
            hardware
                .accelerators
                .iter()
                .map(|accelerator| accelerator.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        format!(
            "Default endpoint policy: {}:{} ({:?})",
            endpoint.host, endpoint.port, snapshot.endpoint_policy.exposure
        ),
        format!("Recent runs: {recent_run_count}"),
    ]
    .join("\n")
}

fn doctor(context: &RuntimeContext) -> String {
    let plan = &context.model_plan;
    let mut lines = vec![
        "Doctor".to_owned(),
        format!("Model: {}", plan.model),
        format!(
            "Engine: {}",
            option_text(
                plan.recommended_engine
                    .as_ref()
                    .map(|engine| engine.0.as_str()),
                "none",
            )
        ),
        format!("Runnable now: {}", yes_no(plan.runnable_now)),
    ];

    lines.extend(format_engines(&context.snapshot));

    if !plan.missing.is_empty() {
        lines.push("Missing:".to_owned());
        lines.extend(plan.missing.iter().map(|item| format!("- {item}")));
    }

    if !plan.warnings.is_empty() {
        lines.push("Warnings:".to_owned());
        lines.extend(plan.warnings.iter().map(|item| format!("- {item}")));
    }

    if !plan.next_actions.is_empty() {
        lines.push("Next actions:".to_owned());
        lines.extend(plan.next_actions.iter().map(|action| {
            if let Some(command) = &action.command {
                format!("- {}: {}", action.label, command)
            } else {
                format!("- {}: {}", action.label, action.reason)
            }
        }));
    }

    lines.join("\n")
}

fn prepare(context: &RuntimeContext) -> String {
    let plan = &context.model_plan;
    let mut lines = vec!["Prepare".to_owned()];

    if plan.runnable_now {
        lines.push(format!("{} is ready to run locally.", plan.model));
        lines.push("Run `adonai run proof`.".to_owned());
        return lines.join("\n");
    }

    if plan.next_actions.is_empty() {
        lines.push("No automatic preparation path is implemented for this model route.".to_owned());
        lines.push(
            "Adonai will not install engines or download models without explicit support."
                .to_owned(),
        );
        return lines.join("\n");
    }

    lines.push("Run the next action, then run `adonai up` again.".to_owned());
    for action in &plan.next_actions {
        lines.push(format!("- {}", action.label));
        if let Some(command) = &action.command {
            lines.push(format!("  {command}"));
        }
        lines.push(format!("  {}", action.reason));
    }

    lines.join("\n")
}

async fn run_proof(context: &RuntimeContext) -> Result<AgentRunRecord, CliError> {
    let agent = proof_agent(&context.model_plan);
    let goal = "Summarise the current Adonai local runtime status in one sentence.";
    let run = context.run_store.create_run(&agent, goal)?;
    let registry = ChatProviderRegistry::with_default_providers();

    match run_once(&agent, &registry, RunInput { goal: goal.into() }).await {
        Ok(outcome) => Ok(context.run_store.mark_succeeded(
            &run.id,
            &outcome.provider,
            &outcome.model,
            &outcome.final_message,
        )?),
        Err(error) => Ok(context.run_store.mark_failed(&run.id, &error.to_string())?),
    }
}

fn proof_agent(plan: &ModelRunPlan) -> AgentDef {
    if plan.runnable_now
        && plan.artifact == ModelArtifact::OllamaModel
        && plan
            .recommended_engine
            .as_ref()
            .is_some_and(|engine| engine.0 == "ollama.local")
    {
        return AgentDef {
            id: adonai_agent::AgentId("local-model-proof".to_owned()),
            name: "Local Model Proof".to_owned(),
            description: Some(
                "Verifies Adonai can execute a real local model through the runtime.".to_owned(),
            ),
            model: adonai_agent::ModelRef {
                provider: "ollama".to_owned(),
                name: plan.model.clone(),
                max_tokens: Some(160),
                temperature: Some(0.2),
            },
            agent_loop: adonai_agent::LoopSpec {
                kind: adonai_agent::LoopKind::React,
                system_prompt: Some(
                    "You are a terse local runtime operator. Answer with concrete runtime facts only."
                        .to_owned(),
                ),
                max_steps: Some(1),
                spec_path: None,
            },
            tools: Vec::new(),
            triggers: vec![adonai_agent::Trigger {
                kind: adonai_agent::TriggerKind::Manual,
                cron: None,
                path: None,
            }],
            state_dir: "~/.adonai/state/local-model-proof".to_owned(),
            secrets: Vec::new(),
            resources: adonai_agent::ResourceLimits::default(),
            lifecycle: adonai_agent::LifecycleHandlers::default(),
        };
    }

    AgentDef {
        id: adonai_agent::AgentId("supervisor-smoke".to_owned()),
        name: "Supervisor Smoke Test".to_owned(),
        description: Some(
            "Verifies the Adonai runtime path without claiming local inference.".to_owned(),
        ),
        model: adonai_agent::ModelRef {
            provider: "mock".to_owned(),
            name: "test-model".to_owned(),
            max_tokens: None,
            temperature: Some(0.2),
        },
        agent_loop: adonai_agent::LoopSpec {
            kind: adonai_agent::LoopKind::React,
            system_prompt: Some("You are a terse local runtime operator.".to_owned()),
            max_steps: Some(1),
            spec_path: None,
        },
        tools: Vec::new(),
        triggers: vec![adonai_agent::Trigger {
            kind: adonai_agent::TriggerKind::Manual,
            cron: None,
            path: None,
        }],
        state_dir: "~/.adonai/state/supervisor-smoke".to_owned(),
        secrets: Vec::new(),
        resources: adonai_agent::ResourceLimits::default(),
        lifecycle: adonai_agent::LifecycleHandlers::default(),
    }
}

fn format_proof(run: AgentRunRecord) -> String {
    let proof_label = if run.provider.as_deref() == Some("ollama") {
        "Local proof"
    } else {
        "Supervisor smoke"
    };

    [
        proof_label.to_owned(),
        format!("Run: {}", run.id),
        format!("Status: {:?}", run.status),
        format!(
            "Provider: {}/{}",
            option_text(run.provider.as_deref(), "pending"),
            option_text(run.model.as_deref(), "pending")
        ),
        format!("Message: {}", proof_message(&run)),
    ]
    .join("\n")
}

fn report(context: &RuntimeContext) -> String {
    let snapshot = &context.snapshot;
    let hardware = &snapshot.hardware;
    let platform = &hardware.platform;
    let plan = &context.model_plan;
    let latest_run = context
        .run_store
        .list_runs(1)
        .ok()
        .and_then(|mut runs| runs.drain(..).next());

    let mut lines = vec![
        "Adonai system report".to_owned(),
        format!("Version: {}", snapshot.version),
        format!("OS: {}", platform.os),
        format!("Architecture: {}", platform.architecture),
        format!("CPU: {}", hardware.cpu_brand),
        format!("Memory: {} GB", bytes_to_gb(hardware.memory.total_bytes)),
        format!(
            "Accelerators: {}",
            hardware
                .accelerators
                .iter()
                .map(|accelerator| accelerator.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        format!("Endpoint exposure: {:?}", snapshot.endpoint_policy.exposure),
        String::new(),
        "Engines:".to_owned(),
    ];

    lines.extend(format_engines(snapshot));
    lines.extend([
        String::new(),
        "Model plan:".to_owned(),
        format!("Model: {}", plan.model),
        format!("Source: {:?}", plan.source),
        format!("Artifact: {:?}", plan.artifact),
        format!("Memory class: {:?}", plan.memory_class),
        format!("Runnable now: {}", yes_no(plan.runnable_now)),
    ]);

    if !plan.next_actions.is_empty() {
        lines.push("Next actions:".to_owned());
        lines.extend(plan.next_actions.iter().map(|action| {
            if let Some(command) = &action.command {
                format!("- {}: {}", action.label, command)
            } else {
                format!("- {}: {}", action.label, action.reason)
            }
        }));
    }

    lines.push(String::new());
    lines.push("Latest run:".to_owned());
    if let Some(run) = latest_run {
        lines.push(format!("Run: {}", run.id));
        lines.push(format!("Agent: {}", run.agent_id));
        lines.push(format!("Status: {:?}", run.status));
        lines.push(format!(
            "Provider: {}/{}",
            option_text(run.provider.as_deref(), "pending"),
            option_text(run.model.as_deref(), "pending")
        ));
    } else {
        lines.push("No runs persisted yet.".to_owned());
    }

    lines.join("\n")
}

fn format_engines(snapshot: &SupervisorSnapshot) -> Vec<String> {
    snapshot
        .engines
        .engines
        .iter()
        .map(|engine| {
            let models = if engine.installed_models.is_empty() {
                "no models reported".to_owned()
            } else {
                engine
                    .installed_models
                    .iter()
                    .map(|model| model.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            };

            format!("- {}: {:?} ({models})", engine.adapter_id.0, engine.health)
        })
        .collect()
}

fn proof_message(run: &AgentRunRecord) -> String {
    if let Some(message) = &run.final_message {
        return message.content.clone();
    }

    option_text(run.error.as_deref(), "No final message yet").to_owned()
}

fn run_store_path() -> PathBuf {
    if let Ok(path) = env::var("ADONAI_RUN_DB") {
        return PathBuf::from(path);
    }

    let home = match env::var("HOME") {
        Ok(home) => home,
        Err(_) => ".".to_owned(),
    };
    PathBuf::from(home).join(".adonai/state/runs.db")
}

fn option_text<'a>(value: Option<&'a str>, fallback: &'a str) -> &'a str {
    match value {
        Some(value) => value,
        None => fallback,
    }
}

fn bytes_to_gb(bytes: u64) -> String {
    format!("{:.1}", bytes as f64 / 1024.0 / 1024.0 / 1024.0)
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

fn help_text() -> String {
    [
        "Adonai",
        "The fastest OS to run your own local models.",
        "",
        "Usage:",
        "  adonai run",
        "  adonai up",
        "  adonai status",
        "  adonai doctor",
        "  adonai prepare",
        "  adonai run proof",
        "  adonai report",
        "",
        "Environment:",
        "  ADONAI_STARTER_MODEL  Starter model to plan. Defaults to llama3.2:3b.",
        "  ADONAI_RUN_DB         Run history database path.",
    ]
    .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_default_command_as_up() {
        assert_eq!(parse_command(Vec::<String>::new()).ok(), Some(Command::Up));
        assert_eq!(parse_command(["up".to_owned()]).ok(), Some(Command::Up));
    }

    #[test]
    fn parses_run_proof_command() {
        assert_eq!(parse_command(["run".to_owned()]).ok(), Some(Command::Run));
        assert_eq!(
            parse_command(["run".to_owned(), "proof".to_owned()]).ok(),
            Some(Command::RunProof)
        );
    }

    #[test]
    fn rejects_unknown_command() {
        assert!(matches!(
            parse_command(["chat".to_owned()]),
            Err(CliError::Usage(_))
        ));
    }
}
