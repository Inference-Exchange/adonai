use serde::{Deserialize, Serialize};

use crate::{
    EngineAdapterId, EngineHealth, EngineKind, EngineProbe, EngineStatus, HardwareProfile,
    hardware::scan_hardware,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelPlanRequest {
    pub model: String,
    #[serde(default)]
    pub source: Option<ModelSource>,
    #[serde(default)]
    pub artifact: Option<ModelArtifact>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModelSource {
    Ollama,
    HuggingFace,
    LocalFile,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModelArtifact {
    OllamaModel,
    Safetensors,
    Gguf,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModelMemoryClass {
    Tiny,
    Small,
    Medium,
    Large,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModelPlanActionKind {
    InstallEngine,
    StartEngine,
    PullModel,
    SelectSupportedModel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelPlanAction {
    pub kind: ModelPlanActionKind,
    pub label: String,
    pub command: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelRunPlan {
    pub model: String,
    pub source: ModelSource,
    pub artifact: ModelArtifact,
    pub recommended_engine: Option<EngineAdapterId>,
    pub runnable_now: bool,
    pub memory_class: ModelMemoryClass,
    pub reasons: Vec<String>,
    pub missing: Vec<String>,
    pub warnings: Vec<String>,
    pub next_actions: Vec<ModelPlanAction>,
}

pub fn plan_model_run(request: ModelPlanRequest, engines: &EngineProbe) -> ModelRunPlan {
    let hardware = scan_hardware();
    plan_model_run_with_hardware(request, engines, &hardware)
}

pub fn plan_model_run_with_hardware(
    request: ModelPlanRequest,
    engines: &EngineProbe,
    hardware: &HardwareProfile,
) -> ModelRunPlan {
    let source = request
        .source
        .clone()
        .unwrap_or_else(|| infer_source(&request.model));
    let artifact = request
        .artifact
        .clone()
        .unwrap_or_else(|| infer_artifact(&request.model, &source));
    let memory_class = infer_memory_class(&request.model);
    let mut reasons = Vec::new();
    let mut missing = Vec::new();
    let mut warnings = Vec::new();
    let mut next_actions = Vec::new();

    let preferred = match (&source, &artifact) {
        (ModelSource::Ollama, _) | (_, ModelArtifact::OllamaModel) => "ollama.local",
        (_, ModelArtifact::Gguf) => "llama-cpp.local",
        (ModelSource::HuggingFace, ModelArtifact::Safetensors) if is_apple_silicon(hardware) => {
            "mlx.local"
        }
        (ModelSource::HuggingFace, ModelArtifact::Safetensors) => "vllm.local",
        _ if is_apple_silicon(hardware) => "mlx.local",
        _ => "ollama.local",
    };

    let preferred_status = engines
        .engines
        .iter()
        .find(|engine| engine.adapter_id.0 == preferred);

    let runnable_now = preferred_status.is_some_and(|engine| {
        engine.health == EngineHealth::Available
            && adapter_is_implemented(preferred)
            && model_is_available_now(&request.model, &artifact, engine)
    });

    if runnable_now {
        reasons.push(format!(
            "{preferred} is installed and supports the inferred model path."
        ));
    } else {
        if let Some(engine) = preferred_status {
            if engine.health == EngineHealth::ApiUnavailable {
                missing.push(format!(
                    "{preferred} is installed but its local API is unavailable."
                ));
                next_actions.push(ModelPlanAction {
                    kind: ModelPlanActionKind::StartEngine,
                    label: "Start Ollama".to_owned(),
                    command: Some("ollama serve".to_owned()),
                    reason: "Adonai can only run Ollama models after the local Ollama API is accepting requests.".to_owned(),
                });
            }
            if engine.health == EngineHealth::BinaryMissing {
                next_actions.push(ModelPlanAction {
                    kind: ModelPlanActionKind::InstallEngine,
                    label: "Install Ollama".to_owned(),
                    command: None,
                    reason: "Adonai does not install inference engines yet; install Ollama, then refresh the init flow.".to_owned(),
                });
            }
            if is_missing_ollama_model(&request.model, &artifact, engine) {
                missing.push(format!(
                    "Ollama model `{}` is not installed. Run `ollama pull {}`.",
                    request.model, request.model
                ));
                next_actions.push(ModelPlanAction {
                    kind: ModelPlanActionKind::PullModel,
                    label: format!("Pull {}", request.model),
                    command: Some(format!("ollama pull {}", request.model)),
                    reason:
                        "The selected starter model must exist locally before Adonai can run it."
                            .to_owned(),
                });
            }
        }
        missing.push(format!(
            "{preferred} is not ready as an Adonai execution adapter."
        ));
    }

    match artifact {
        ModelArtifact::Gguf => {
            reasons.push(
                "GGUF is best routed to llama.cpp because it owns that local file format."
                    .to_owned(),
            );
            if preferred == "llama-cpp.local" {
                next_actions.push(ModelPlanAction {
                    kind: ModelPlanActionKind::SelectSupportedModel,
                    label: "Use an Ollama starter model".to_owned(),
                    command: Some("ADONAI_STARTER_MODEL=llama3.2:3b bun run init".to_owned()),
                    reason: "llama.cpp process management is not implemented yet; Ollama is the current first-run execution path.".to_owned(),
                });
            }
        }
        ModelArtifact::Safetensors => {
            reasons.push("Safetensors/Hugging Face artifacts require an engine that understands model architecture, tokenizer, and weight loading.".to_owned());
        }
        ModelArtifact::OllamaModel => {
            reasons.push("Ollama model names are the fastest path to useful local execution while Adonai's native engine manager matures.".to_owned());
        }
        ModelArtifact::Unknown => {
            warnings.push("Artifact format is unknown; Adonai cannot promise execution until it inspects model files.".to_owned());
        }
    }

    if matches!(memory_class, ModelMemoryClass::Large)
        && hardware.memory.total_bytes < 32 * 1024 * 1024 * 1024
    {
        warnings.push("This model appears large for the detected memory; expect quantization or cloud failover to be required.".to_owned());
    }

    if is_apple_silicon(hardware) && preferred == "mlx.local" {
        reasons.push(
            "Apple Silicon unified memory makes MLX the preferred native path once installed."
                .to_owned(),
        );
    }

    ModelRunPlan {
        model: request.model,
        source,
        artifact,
        recommended_engine: Some(EngineAdapterId(preferred.to_owned())),
        runnable_now,
        memory_class,
        reasons,
        missing,
        warnings,
        next_actions,
    }
}

fn adapter_is_implemented(adapter_id: &str) -> bool {
    matches!(adapter_id, "ollama.local")
}

fn model_is_available_now(model: &str, artifact: &ModelArtifact, engine: &EngineStatus) -> bool {
    match (&engine.kind, artifact) {
        (EngineKind::Ollama, ModelArtifact::OllamaModel) => engine
            .installed_models
            .iter()
            .any(|installed| installed.name == model),
        _ => true,
    }
}

fn is_missing_ollama_model(model: &str, artifact: &ModelArtifact, engine: &EngineStatus) -> bool {
    engine.kind == EngineKind::Ollama
        && matches!(artifact, ModelArtifact::OllamaModel)
        && engine.health == EngineHealth::Available
        && !engine
            .installed_models
            .iter()
            .any(|installed| installed.name == model)
}

fn infer_source(model: &str) -> ModelSource {
    if looks_like_local_path(model) {
        ModelSource::LocalFile
    } else if model.contains('/') {
        ModelSource::HuggingFace
    } else {
        ModelSource::Ollama
    }
}

fn infer_artifact(model: &str, source: &ModelSource) -> ModelArtifact {
    let lower = model.to_ascii_lowercase();
    if lower.ends_with(".gguf") {
        ModelArtifact::Gguf
    } else if lower.ends_with(".safetensors") {
        ModelArtifact::Safetensors
    } else if matches!(source, ModelSource::Ollama) {
        ModelArtifact::OllamaModel
    } else if matches!(source, ModelSource::HuggingFace) {
        ModelArtifact::Safetensors
    } else {
        ModelArtifact::Unknown
    }
}

fn infer_memory_class(model: &str) -> ModelMemoryClass {
    let lower = model.to_ascii_lowercase();
    if lower.contains("1b") || lower.contains("2b") || lower.contains("3b") {
        ModelMemoryClass::Tiny
    } else if lower.contains("4b")
        || lower.contains("7b")
        || lower.contains("8b")
        || lower.contains("9b")
    {
        ModelMemoryClass::Small
    } else if lower.contains("14b")
        || lower.contains("27b")
        || lower.contains("30b")
        || lower.contains("32b")
    {
        ModelMemoryClass::Medium
    } else if lower.contains("70b") || lower.contains("120b") || lower.contains("405b") {
        ModelMemoryClass::Large
    } else {
        ModelMemoryClass::Unknown
    }
}

fn looks_like_local_path(model: &str) -> bool {
    model.starts_with('/')
        || model.starts_with("./")
        || model.starts_with("../")
        || model.contains(".gguf")
        || model.contains(".safetensors")
}

fn is_apple_silicon(hardware: &HardwareProfile) -> bool {
    hardware.platform.os == "Darwin" && hardware.platform.architecture == "aarch64"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        EngineCapability, EngineInstalledModel, EngineKind, EngineProvenance, EngineStatus,
        HostPlatform, MemoryProfile, NetworkProfile,
    };

    #[test]
    fn plans_ollama_model_to_installed_ollama() {
        let plan = plan_model_run_with_hardware(
            ModelPlanRequest {
                model: "llama3.2:3b".to_owned(),
                source: None,
                artifact: None,
            },
            &EngineProbe {
                engines: vec![engine_with_models(
                    "ollama.local",
                    EngineKind::Ollama,
                    EngineHealth::Available,
                    vec!["llama3.2:3b"],
                )],
                recommendations: Vec::new(),
            },
            &apple_silicon_8gb(),
        );

        assert_eq!(
            plan.recommended_engine,
            Some(EngineAdapterId("ollama.local".to_owned()))
        );
        assert!(plan.runnable_now);
        assert_eq!(plan.artifact, ModelArtifact::OllamaModel);
        assert!(plan.next_actions.is_empty());
    }

    #[test]
    fn marks_ollama_model_missing_when_binary_is_ready_but_model_is_not_pulled() {
        let plan = plan_model_run_with_hardware(
            ModelPlanRequest {
                model: "llama3.2:3b".to_owned(),
                source: None,
                artifact: None,
            },
            &EngineProbe {
                engines: vec![engine_with_models(
                    "ollama.local",
                    EngineKind::Ollama,
                    EngineHealth::Available,
                    vec!["qwen2.5:7b"],
                )],
                recommendations: Vec::new(),
            },
            &apple_silicon_8gb(),
        );

        assert!(!plan.runnable_now);
        assert!(
            plan.missing
                .iter()
                .any(|item| item.contains("ollama pull llama3.2:3b"))
        );
        assert!(plan.next_actions.iter().any(|action| {
            action.kind == ModelPlanActionKind::PullModel
                && action.command.as_deref() == Some("ollama pull llama3.2:3b")
        }));
    }

    #[test]
    fn adds_start_action_when_ollama_api_is_unavailable() {
        let plan = plan_model_run_with_hardware(
            ModelPlanRequest {
                model: "llama3.2:3b".to_owned(),
                source: None,
                artifact: None,
            },
            &EngineProbe {
                engines: vec![engine(
                    "ollama.local",
                    EngineKind::Ollama,
                    EngineHealth::ApiUnavailable,
                )],
                recommendations: Vec::new(),
            },
            &apple_silicon_8gb(),
        );

        assert!(!plan.runnable_now);
        assert!(plan.next_actions.iter().any(|action| {
            action.kind == ModelPlanActionKind::StartEngine
                && action.command.as_deref() == Some("ollama serve")
        }));
    }

    #[test]
    fn plans_gguf_to_llama_cpp_but_marks_missing() {
        let plan = plan_model_run_with_hardware(
            ModelPlanRequest {
                model: "/models/qwen.gguf".to_owned(),
                source: None,
                artifact: None,
            },
            &EngineProbe {
                engines: vec![engine(
                    "llama-cpp.local",
                    EngineKind::LlamaCpp,
                    EngineHealth::BinaryMissing,
                )],
                recommendations: Vec::new(),
            },
            &apple_silicon_8gb(),
        );

        assert_eq!(
            plan.recommended_engine,
            Some(EngineAdapterId("llama-cpp.local".to_owned()))
        );
        assert!(!plan.runnable_now);
        assert_eq!(plan.artifact, ModelArtifact::Gguf);
        assert!(plan.next_actions.iter().any(|action| {
            action.kind == ModelPlanActionKind::SelectSupportedModel
                && action
                    .command
                    .as_deref()
                    .is_some_and(|command| command.contains("ADONAI_STARTER_MODEL"))
        }));
    }

    fn engine(adapter_id: &str, kind: EngineKind, health: EngineHealth) -> EngineStatus {
        EngineStatus {
            adapter_id: EngineAdapterId(adapter_id.to_owned()),
            kind,
            health,
            capabilities: vec![EngineCapability {
                name: "test".to_owned(),
                supported: true,
            }],
            installed_models: Vec::new(),
            provenance: EngineProvenance {
                binary_path: None,
                version: None,
                source: "test".to_owned(),
            },
        }
    }

    fn engine_with_models(
        adapter_id: &str,
        kind: EngineKind,
        health: EngineHealth,
        models: Vec<&str>,
    ) -> EngineStatus {
        let mut engine = engine(adapter_id, kind, health);
        engine.installed_models = models
            .into_iter()
            .map(|name| EngineInstalledModel {
                name: name.to_owned(),
            })
            .collect();
        engine
    }

    fn apple_silicon_8gb() -> HardwareProfile {
        HardwareProfile {
            platform: HostPlatform {
                os: "Darwin".to_owned(),
                os_version: None,
                kernel_version: None,
                architecture: "aarch64".to_owned(),
                hostname: None,
            },
            cpu_brand: "Apple M1".to_owned(),
            physical_core_count: Some(8),
            memory: MemoryProfile {
                total_bytes: 8 * 1024 * 1024 * 1024,
                available_bytes: None,
            },
            storage: Vec::new(),
            accelerators: Vec::new(),
            network: NetworkProfile {
                default_exposure: "loopback".to_owned(),
            },
        }
    }
}
