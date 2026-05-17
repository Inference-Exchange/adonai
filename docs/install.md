# Installing Adonai

Adonai is currently a terminal-first alpha for macOS and Linux. The public entrypoint is the `adonai` CLI, built around the product promise: the fastest OS to run your own local models.

## Release Install

Download the archive for your machine from the latest GitHub release:

- `adonai-aarch64-apple-darwin.tar.gz` for Apple Silicon Macs.
- `adonai-x86_64-apple-darwin.tar.gz` for Intel Macs.
- `adonai-x86_64-unknown-linux-gnu.tar.gz` for Linux x64.

Then unpack and run:

```sh
tar -xzf adonai-aarch64-apple-darwin.tar.gz
cd adonai-aarch64-apple-darwin
./adonai run
```

Replace the archive and directory names with the artifact for your platform.

## Source Install

Requirements:

- Rust toolchain.
- Optional: Ollama for real local model execution.
- Optional: Bun if you want the OpenTUI dashboard.

```sh
git clone https://github.com/Inference-Exchange/adonai.git
cd adonai
cargo run -p adonai-cli -- up
```

## First Commands

```sh
adonai run
adonai run --yes
adonai up
adonai status
adonai doctor
adonai prepare
adonai run proof
adonai report
```

`adonai run` is the main first-run command. It should become the fastest path from a fresh machine to a real local generation. In the current alpha it scans the machine, checks local AI readiness, and either gives a concrete next action or confirms a local proof path. `adonai run --yes` lets Adonai apply supported setup actions such as starting Ollama or pulling the selected model. `adonai up` remains the lower-level status and doctor flow.

## Ollama Path

Adonai does not install engines or download models automatically yet. If Ollama is not ready, Adonai will say so and show the next command, such as:

```sh
ollama serve
ollama pull llama3.2:3b
```

After running the next action, run:

```sh
adonai up
```

## Early Test Reports

Generate a GitHub-ready report with:

```sh
adonai report
```

Paste that output into an early test report issue. It includes hardware, engine health, model readiness, next actions, and latest run status.

## Current Gaps

- No signed Mac app.
- No menu bar control surface.
- No automatic inference-engine install.
- No automatic model download.
- No marketplace.
- No cloud failover.
