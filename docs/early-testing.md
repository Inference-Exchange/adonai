# Adonai Early Testing Guide

Adonai is the agent OS for owned compute. The alpha should prove one thing first: a developer can clone the repo, start a local supervisor, see what their machine can run, and execute a persisted proof agent without guessing what is happening.

## What You Can Test Now

- Start the Adonai supervisor on loopback.
- Run the OpenTUI init flow.
- Inspect hardware and network exposure.
- Probe installed inference engines: Ollama, llama.cpp, MLX, vLLM, and SGLang.
- Ask Adonai to plan a model run for `llama3.2:3b`.
- Execute a real local Ollama proof agent when `llama3.2:3b` is ready, or an explicit deterministic supervisor smoke run when it is not.
- Confirm runs persist in SQLite.

## Requirements

- macOS or Linux.
- Rust toolchain.
- Bun.
- Optional: Ollama, llama.cpp, MLX, vLLM, or SGLang installed on your `PATH`.

Adonai does not install inference engines or download models yet. The current build detects them honestly and explains what is missing.

## Install From Source

```sh
git clone https://github.com/Inference-Exchange/adonai.git
cd adonai
bun install
```

Start the supervisor:

```sh
. "$HOME/.cargo/env"
cargo run -p adonai-supervisor
```

In another terminal, run first-time onboarding:

```sh
bun run init
```

For a non-interactive smoke check:

```sh
bun run init:check
```

## Expected Behavior

The init flow should show:

- machine summary,
- loopback-only privacy state,
- engine recommendation for the requested model,
- whether the model is runnable now,
- installed Ollama model names when Ollama is available,
- missing runtime pieces,
- one proof agent run,
- recent persisted runs.

If Ollama is installed but `llama3.2:3b` is missing, the model plan should say it is not runnable and include this next action:

```sh
ollama pull llama3.2:3b
```

If Ollama's local API is unavailable, the model plan should show `ollama serve` as the next action. The proof run should use `mock/test-model` and label itself as a supervisor smoke run. That is expected; Adonai should not pretend local inference worked.

The run database lives at:

```sh
~/.adonai/state/runs.db
```

For disposable testing:

```sh
ADONAI_RUN_DB=/tmp/adonai-test-runs.db cargo run -p adonai-supervisor
```

## API Checks

```sh
curl -sS http://127.0.0.1:49231/health
curl -sS http://127.0.0.1:49231/v1/status
curl -sS http://127.0.0.1:49231/v1/engines
curl -sS http://127.0.0.1:49231/v1/models/plan \
  -H 'content-type: application/json' \
  -d '{"model":"llama3.2:3b"}'
```

## What Does Not Work Yet

- No signed Mac app.
- No menu bar control surface.
- No launchd or systemd installer.
- No Raspberry Pi image.
- No automatic inference-engine install.
- No automatic model download.
- No multi-step crash-resumable ReAct loop.
- No MCP server lifecycle.
- No Inference Exchange cloud failover.

## Feedback Wanted

- Did the supervisor start cleanly?
- Did the init flow explain your machine correctly?
- Which inference engines did Adonai detect or miss?
- Was the model plan useful?
- What part felt too manual or unclear?
- What would need to work before you would run Adonai daily?

Open an early test report using the GitHub issue template. Hardware reports are valuable even when everything works.
