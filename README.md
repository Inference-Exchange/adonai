# Adonai

The agent OS for owned compute.

Adonai is an open-source operating layer for running models and sovereign agents on hardware you own. It is not another chat app, model registry, or thin Ollama wrapper. The bet is that local inference should feel like an operating system: clear hardware status, honest model planning, safe local defaults, durable agents, and engine complexity hidden behind a supervisor.

The current alpha is the foundation: a Rust supervisor, a local-only control API, an OpenTUI onboarding flow, hardware and inference-engine detection, model planning, and persisted agent smoke runs.

## Why Adonai Exists

Running local AI is still too hard. A useful model is not a single executable. It is weights, config, tokenizer files, quantization formats, memory constraints, accelerator quirks, runtime choices, process management, logs, and security boundaries.

Adonai exists to make that complexity disappear for users without hiding it from operators. It should tell you:

- what hardware you own,
- what engines are available,
- what model can run,
- why Adonai chose a route,
- what is missing,
- what is running locally,
- what is exposed to the network.

## What Works Today

- Local-only supervisor API on `127.0.0.1`.
- Hardware profile for OS, CPU, memory, storage, network exposure, and Apple Metal hints.
- Engine adapter detection for Ollama, llama.cpp, MLX, vLLM, and SGLang.
- Ollama readiness detection for binary, local API availability, and installed model names.
- Model planning through `POST /v1/models/plan`.
- OpenTUI init flow that scans, explains, plans, and runs a real local proof agent when the planned Ollama model is ready.
- SQLite-backed run history at `~/.adonai/state/runs.db`.

## Quickstart

Requirements:

- Rust toolchain.
- Bun.
- macOS or Linux.
- Optional: Ollama, llama.cpp, MLX, vLLM, or SGLang on your `PATH`.

```sh
git clone https://github.com/Inference-Exchange/adonai.git
cd adonai
bun install
. "$HOME/.cargo/env"
cargo run -p adonai-supervisor
```

The supervisor binds to `127.0.0.1:49231` by default.

In another terminal:

```sh
bun run init
```

For the lower-level dashboard:

```sh
bun run tui
```

For a non-interactive smoke check:

```sh
bun run init:check
```

Early testers should start with [docs/early-testing.md](docs/early-testing.md). It explains what to test, expected behavior, and current gaps.

## Release Builds

Tagged releases publish draft GitHub releases with supervisor binaries for:

- `x86_64-unknown-linux-gnu`
- `aarch64-apple-darwin`
- `x86_64-apple-darwin`

Release artifacts include SHA-256 checksum files. Draft releases should be verified before publishing.

## API

- `GET /health`
- `GET /v1/status`
- `GET /v1/hardware`
- `GET /v1/engines`
- `POST /v1/models/plan`
- `POST /v1/chat/completions`
- `POST /v1/agents/runs`
- `GET /v1/agents/runs`
- `GET /v1/agents/runs/{run_id}`

Agent runs are persisted to `~/.adonai/state/runs.db` by default. Override this for tests with `ADONAI_RUN_DB`.

Example:

```sh
curl -sS http://127.0.0.1:49231/v1/models/plan \
  -H 'content-type: application/json' \
  -d '{"model":"llama3.2:3b"}'
```

Chat providers:

- `mock` for deterministic runtime tests.
- `ollama` for a local Ollama server at `http://127.0.0.1:11434`.

The init flow uses `llama3.2:3b` by default. Override this with:

```sh
ADONAI_STARTER_MODEL=qwen2.5:7b bun run init
```

If Ollama is installed but the starter model is missing, Adonai reports the model as not runnable and shows the required `ollama pull` next action. If Ollama's local API is unavailable, Adonai shows `ollama serve` as the next action and falls back to a deterministic supervisor smoke run instead of pretending local inference worked.

## Architecture

- `adonai-core`: hardware, engine, endpoint policy, model planning, and supervisor contracts.
- `adonai-agent`: agent definitions, chat providers, one-shot runtime entrypoint, and persisted run state.
- `adonai-supervisor`: local daemon and HTTP API.
- `apps/tui`: OpenTUI init flow and operator dashboard.

The supervisor is the durable product boundary. The Mac app, menu bar, Linux service, and future appliance image should all control the same supervisor contract.

## Current Gaps

- No signed Mac installer or menu bar app.
- No launchd or systemd installer.
- No automatic inference-engine install.
- No automatic model download.
- No durable multi-step ReAct loop yet.
- No MCP server lifecycle yet.
- No Raspberry Pi image.
- No Inference Exchange cloud failover.
- No LAN exposure by default.

## Contributing

Adonai is early. The highest-value contributions right now are installation reports, engine detection fixes, hardware reports, docs corrections, and narrow runtime slices that make local inference work end to end.

Read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request.

## Project Direction

Public direction should live in issues, pull requests, and focused docs. Internal planning notes should stay out of the public repository.
