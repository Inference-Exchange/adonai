# G1-G3 Implementation Evidence

Date: 2026-05-09

## Scope

This first Adonai build starts the product without claiming finished installer, signing, or engine execution support.

## G1: Mac App Packaging And Service Install

Status: Not passed.

Evidence:
- Rust toolchain installed through rustup because Homebrew attempted a source LLVM build under a nonstandard prefix.
- No Tauri or SwiftUI app has been created yet.
- No signed helper, launchd plist, notarized artifact, or updater exists yet.

Next step:
- Build a Tauri shell that controls `adonai-supervisor`, then decide whether Tauri can own the helper/update path or SwiftUI/AppKit must own it.

## G2: Supervisor Contract

Status: First pass implemented.

Evidence:
- `adonai-core` defines typed hardware, engine, endpoint policy, and supervisor snapshot contracts.
- `adonai-supervisor` exposes:
  - `GET /health`
  - `GET /v1/status`
  - `GET /v1/hardware`
  - `GET /v1/engines`
- Default bind is `127.0.0.1:49231`.
- Non-loopback binding requires `--allow-lan`.

Known limits:
- No persistent config file.
- No stable API versioning header yet.
- No pairing contract yet.

## G3: Engine Adapter

Status: Detection pass plus first chat-provider call path implemented.

Evidence:
- Ollama and llama.cpp adapters expose stable adapter IDs.
- Missing binaries are reported as `BinaryMissing`.
- Adonai does not bundle or fake an engine binary.
- Engine provenance explicitly says detection is via PATH.
- `adonai-agent` exposes a `ChatProvider` trait with `mock` and `ollama` implementations.
- `adonai-supervisor` exposes `POST /v1/chat/completions`.
- The `mock` provider works through the live supervisor API and is suitable for deterministic runtime tests.

Known limits:
- No process manager starts an engine yet.
- No model provenance or checksum support yet.
- Ollama provider calls `http://127.0.0.1:11434/api/chat`, but this machine did not have Ollama running during verification.

## Verification

Commands run:

```sh
cargo fmt --check
cargo test
cargo clippy --workspace --all-targets -- -D warnings
```

Live API checks passed against `http://127.0.0.1:49231`:

```sh
curl -sS http://127.0.0.1:49231/health
curl -sS http://127.0.0.1:49231/v1/status
curl -sS http://127.0.0.1:49231/v1/engines
curl -sS -X POST http://127.0.0.1:49231/v1/chat/completions \
  -H 'content-type: application/json' \
  -d '{"provider":"mock","model":"test-model","messages":[{"role":"user","content":"hello adonai"}]}'
```

## Fragile Or Uncertain

- macOS available memory from `sysinfo` returned zero on this machine, so Adonai reports it as `null` rather than false precision.
- Accelerator detection is currently a conservative compile-target hint for Apple Silicon Metal, not a full runtime GPU inventory.
- Engine detection is PATH-based and does not yet verify checksums, signatures, or model compatibility.
- Product naming is settled for the alpha: Adonai is the public project name, and the repo, crate, state, and command surfaces should use it consistently.
