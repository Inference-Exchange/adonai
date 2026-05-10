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
pub struct EngineStatus {
    pub adapter_id: EngineAdapterId,
    pub kind: EngineKind,
    pub health: EngineHealth,
    pub capabilities: Vec<EngineCapability>,
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
    let health = if binary_path.is_some() {
        EngineHealth::Available
    } else {
        EngineHealth::BinaryMissing
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
        provenance: EngineProvenance {
            binary_path,
            version: command_version("ollama", &["--version"]),
            source: "PATH lookup; no engine binary is bundled by Adonai".to_owned(),
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
}
