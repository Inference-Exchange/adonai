use std::{env, path::Path, process::Command};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineAdapterId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EngineKind {
    Ollama,
    LlamaCpp,
    Mlx,
    Vllm,
    Sglang,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EngineHealth {
    Available,
    BinaryMissing,
    ApiUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineCapability {
    pub name: String,
    pub supported: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineProvenance {
    pub binary_path: Option<String>,
    pub version: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineInstalledModel {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineStatus {
    pub adapter_id: EngineAdapterId,
    pub kind: EngineKind,
    pub health: EngineHealth,
    pub capabilities: Vec<EngineCapability>,
    pub installed_models: Vec<EngineInstalledModel>,
    pub provenance: EngineProvenance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineProbe {
    pub engines: Vec<EngineStatus>,
    pub recommendations: Vec<EngineRecommendation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EngineRecommendationLevel {
    Preferred,
    Viable,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineRecommendation {
    pub adapter_id: EngineAdapterId,
    pub level: EngineRecommendationLevel,
    pub reason: String,
}

pub fn probe_engines() -> EngineProbe {
    let engines = vec![
        probe_ollama(),
        probe_llama_cpp(),
        probe_mlx(),
        probe_vllm(),
        probe_sglang(),
    ];

    EngineProbe {
        recommendations: recommend_engines(&engines),
        engines,
    }
}

fn probe_ollama() -> EngineStatus {
    let binary_path = find_binary("ollama");
    let model_probe = binary_path.as_deref().map(list_ollama_models);
    let (health, installed_models) = match model_probe {
        None => (EngineHealth::BinaryMissing, Vec::new()),
        Some(Ok(models)) => (EngineHealth::Available, models),
        Some(Err(())) => (EngineHealth::ApiUnavailable, Vec::new()),
    };

    EngineStatus {
        adapter_id: EngineAdapterId("ollama.local".to_owned()),
        kind: EngineKind::Ollama,
        health,
        capabilities: vec![
            EngineCapability {
                name: "openai-compatible-chat".to_owned(),
                supported: false,
            },
            EngineCapability {
                name: "ollama-native-api".to_owned(),
                supported: binary_path.is_some(),
            },
        ],
        installed_models,
        provenance: EngineProvenance {
            binary_path,
            version: command_version("ollama", &["--version"]),
            source: "PATH lookup plus `ollama list`; no engine binary is bundled by Adonai"
                .to_owned(),
        },
    }
}

fn probe_llama_cpp() -> EngineStatus {
    let binary_path = find_binary("llama-server").or_else(|| find_binary("llama-cli"));
    let health = if binary_path.is_some() {
        EngineHealth::Available
    } else {
        EngineHealth::BinaryMissing
    };

    EngineStatus {
        adapter_id: EngineAdapterId("llama-cpp.local".to_owned()),
        kind: EngineKind::LlamaCpp,
        health,
        capabilities: vec![
            EngineCapability {
                name: "gguf".to_owned(),
                supported: binary_path.is_some(),
            },
            EngineCapability {
                name: "managed-process".to_owned(),
                supported: false,
            },
        ],
        installed_models: Vec::new(),
        provenance: EngineProvenance {
            binary_path,
            version: command_version("llama-server", &["--version"])
                .or_else(|| command_version("llama-cli", &["--version"])),
            source: "PATH lookup; no engine binary is bundled by Adonai".to_owned(),
        },
    }
}

fn probe_mlx() -> EngineStatus {
    let binary_path = find_binary("mlx-lm").or_else(|| find_binary("mlx_lm.server"));
    let apple_silicon = cfg!(target_os = "macos") && cfg!(target_arch = "aarch64");
    let health = if binary_path.is_some() {
        EngineHealth::Available
    } else {
        EngineHealth::BinaryMissing
    };

    EngineStatus {
        adapter_id: EngineAdapterId("mlx.local".to_owned()),
        kind: EngineKind::Mlx,
        health,
        capabilities: vec![
            EngineCapability {
                name: "apple-silicon-unified-memory".to_owned(),
                supported: apple_silicon,
            },
            EngineCapability {
                name: "managed-process".to_owned(),
                supported: false,
            },
        ],
        installed_models: Vec::new(),
        provenance: EngineProvenance {
            binary_path,
            version: command_version("mlx-lm", &["--version"])
                .or_else(|| command_version("mlx_lm.server", &["--version"])),
            source: "PATH lookup; no MLX runtime is bundled by Adonai".to_owned(),
        },
    }
}

fn probe_vllm() -> EngineStatus {
    let binary_path = find_binary("vllm");
    let health = if binary_path.is_some() {
        EngineHealth::Available
    } else {
        EngineHealth::BinaryMissing
    };

    EngineStatus {
        adapter_id: EngineAdapterId("vllm.local".to_owned()),
        kind: EngineKind::Vllm,
        health,
        capabilities: vec![
            EngineCapability {
                name: "openai-compatible-chat".to_owned(),
                supported: binary_path.is_some(),
            },
            EngineCapability {
                name: "continuous-batching".to_owned(),
                supported: binary_path.is_some(),
            },
            EngineCapability {
                name: "paged-attention".to_owned(),
                supported: binary_path.is_some(),
            },
        ],
        installed_models: Vec::new(),
        provenance: EngineProvenance {
            binary_path,
            version: command_version("vllm", &["--version"]),
            source: "PATH lookup; no vLLM runtime is bundled by Adonai".to_owned(),
        },
    }
}

fn probe_sglang() -> EngineStatus {
    let binary_path = find_binary("sglang").or_else(|| find_binary("python"));
    let health = if binary_path.is_some() {
        EngineHealth::Available
    } else {
        EngineHealth::BinaryMissing
    };

    EngineStatus {
        adapter_id: EngineAdapterId("sglang.local".to_owned()),
        kind: EngineKind::Sglang,
        health,
        capabilities: vec![
            EngineCapability {
                name: "openai-compatible-chat".to_owned(),
                supported: false,
            },
            EngineCapability {
                name: "disaggregated-prefill-decode".to_owned(),
                supported: false,
            },
        ],
        installed_models: Vec::new(),
        provenance: EngineProvenance {
            binary_path,
            version: command_version("python", &["-m", "sglang.launch_server", "--version"]),
            source: "PATH lookup; Adonai has not verified an installed SGLang server".to_owned(),
        },
    }
}

fn recommend_engines(engines: &[EngineStatus]) -> Vec<EngineRecommendation> {
    engines
        .iter()
        .map(|engine| {
            let (level, reason) = match engine.kind {
                EngineKind::Mlx if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") => (
                    EngineRecommendationLevel::Preferred,
                    "Best future native path for Apple Silicon unified memory; adapter not implemented yet.",
                ),
                EngineKind::Ollama if engine.health == EngineHealth::Available => (
                    EngineRecommendationLevel::Preferred,
                    "Installed and easiest local execution path for first user testing.",
                ),
                EngineKind::LlamaCpp if engine.health == EngineHealth::Available => (
                    EngineRecommendationLevel::Viable,
                    "Installed GGUF baseline; useful for direct model-file execution.",
                ),
                EngineKind::Vllm if engine.health == EngineHealth::Available => (
                    EngineRecommendationLevel::Viable,
                    "Installed high-throughput server engine; strongest on Linux GPU servers.",
                ),
                EngineKind::Sglang if engine.health == EngineHealth::Available => (
                    EngineRecommendationLevel::Viable,
                    "Possible advanced serving engine; Adonai has not verified launch semantics yet.",
                ),
                _ => (
                    EngineRecommendationLevel::Unsupported,
                    "Engine is not installed or not yet supported by an Adonai process adapter.",
                ),
            };

            EngineRecommendation {
                adapter_id: engine.adapter_id.clone(),
                level,
                reason: reason.to_owned(),
            }
        })
        .collect()
}

fn find_binary(binary: &str) -> Option<String> {
    let paths = env::var_os("PATH")?;

    env::split_paths(&paths)
        .map(|directory| directory.join(binary))
        .find(|candidate| is_executable_file(candidate))
        .map(|path| path.to_string_lossy().into_owned())
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };

    if !metadata.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        metadata.permissions().mode() & 0o111 != 0
    }

    #[cfg(not(unix))]
    {
        true
    }
}

fn command_version(binary: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(binary).args(args).output().ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8(output.stdout).ok()?;
    let stderr = String::from_utf8(output.stderr).ok()?;
    let version = if stdout.trim().is_empty() {
        stderr.trim()
    } else {
        stdout.trim()
    };

    if version.is_empty() {
        None
    } else {
        Some(version.to_owned())
    }
}

fn list_ollama_models(binary: &str) -> Result<Vec<EngineInstalledModel>, ()> {
    let output = Command::new(binary).arg("list").output().map_err(|_| ())?;
    if !output.status.success() {
        return Err(());
    }

    let stdout = String::from_utf8(output.stdout).map_err(|_| ())?;
    Ok(parse_ollama_list(&stdout))
}

fn parse_ollama_list(stdout: &str) -> Vec<EngineInstalledModel> {
    stdout
        .lines()
        .skip(1)
        .filter_map(|line| line.split_whitespace().next())
        .filter(|name| !name.trim().is_empty())
        .map(|name| EngineInstalledModel {
            name: name.to_owned(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probes_keep_adapter_identity_stable() {
        let probe = probe_engines();
        let ids = probe
            .engines
            .iter()
            .map(|engine| engine.adapter_id.0.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            ids,
            vec![
                "ollama.local",
                "llama-cpp.local",
                "mlx.local",
                "vllm.local",
                "sglang.local"
            ]
        );
        assert_eq!(probe.recommendations.len(), probe.engines.len());
    }

    #[test]
    fn parses_ollama_list_models() {
        let models = parse_ollama_list(
            "NAME                ID              SIZE      MODIFIED\nllama3.2:3b         abc123          2.0 GB    2 hours ago\nqwen2.5:7b          def456          4.7 GB    1 day ago\n",
        );

        assert_eq!(
            models,
            vec![
                EngineInstalledModel {
                    name: "llama3.2:3b".to_owned()
                },
                EngineInstalledModel {
                    name: "qwen2.5:7b".to_owned()
                }
            ]
        );
    }
}
