# Adonai Early Testing Guide

Adonai is the fastest OS to run your own local models. The alpha should prove one thing first: a developer can install one terminal command, get to a real local generation quickly, see what their machine can run, get an exact next action when setup is incomplete, and execute or prepare a persisted proof agent without guessing what is happening.

## What You Can Test Now

- Run `adonai up`.
- Run `adonai run` and inspect whether it gives a real local generation or the exact setup action.
- Inspect hardware and network exposure.
- Probe installed inference engines: Ollama, llama.cpp, MLX, vLLM, and SGLang.
- Ask Adonai to plan a model run for `llama3.2:3b`.
- Execute a real local Ollama proof agent when `llama3.2:3b` is ready, or an explicit deterministic supervisor smoke run when it is not.
- Confirm runs persist in SQLite.
- Generate a GitHub-ready report with `adonai report`.

## Requirements

- macOS or Linux.
- Rust toolchain.
- Optional: Ollama, llama.cpp, MLX, vLLM, or SGLang installed on your `PATH`.
- Optional: Bun if you want to test the OpenTUI dashboard from source.

Adonai does not install inference engines or download models yet. The current build detects them honestly and explains what is missing.

## Install From Source

```sh
git clone https://github.com/Inference-Exchange/adonai.git
cd adonai
bun install
```

Run the terminal-first CLI:

```sh
. "$HOME/.cargo/env"
cargo run -p adonai-cli -- run
cargo run -p adonai-cli -- run --yes
cargo run -p adonai-cli -- up
```

Check readiness directly:

```sh
cargo run -p adonai-cli -- doctor
```

Generate a report:

```sh
cargo run -p adonai-cli -- report
```

## Expected Behavior

`adonai up` should show:

- machine summary,
- loopback-only privacy state,
- engine recommendation for the requested model,
- whether the model is runnable now,
- installed Ollama model names when Ollama is available,
- missing runtime pieces,
- a proof agent run when runnable, or a clear prepare action when not runnable,
- tokens/sec when a real Ollama proof run returns timing metadata,
- recent persisted runs.

If Ollama is installed but `llama3.2:3b` is missing, the model plan should say it is not runnable and include this next action:

```sh
ollama pull llama3.2:3b
```

If Ollama's local API is unavailable, the model plan should show `ollama serve` as the next action. `adonai run proof` should use `mock/test-model` and label itself as a supervisor smoke run. That is expected; Adonai should not pretend local inference worked.

The run database lives at:

```sh
~/.adonai/state/runs.db
```

For disposable testing:

```sh
ADONAI_RUN_DB=/tmp/adonai-test-runs.db cargo run -p adonai-cli -- up
```

## OpenTUI Dashboard

The OpenTUI dashboard still exists for richer terminal UI testing:

```sh
bun install
cargo run -p adonai-supervisor
bun run init
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
- Did `adonai up` explain your machine correctly?
- Which inference engines did Adonai detect or miss?
- Was the model plan and next action useful?
- What part felt too manual or unclear?
- What would need to work before you would run Adonai daily?

Open an early test report using the GitHub issue template and paste `adonai report`. Hardware reports are valuable even when everything works.
