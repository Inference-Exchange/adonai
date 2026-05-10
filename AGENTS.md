# Adonai

## Product Rule
Adonai is a standalone product for turning owned hardware into trusted AI compute. It is not a quick feature inside Inference Exchange.

## Product Standard
- Mission: abstract inference complexity so normal users can run sovereign AI agents on hardware they own.
- Every product surface should make the machine understandable: what hardware exists, what models can run, which engine Adonai chose, why it chose it, and what is missing.
- The user should not need to understand vLLM, MLX, llama.cpp, GGUF, safetensors, mmap, KV cache, quantization, or GPU offload before getting useful work done.
- The interface should feel like an operating system: calm status, clear controls, reversible setup, visible privacy/exposure state, and no fake magic.
- Details matter. A future Mac menu-bar surface should show active models, running agents, compute use, local/cloud status, and engine health at a glance.
- Effectiveness before efficiency: first make the flow work end to end, then optimize performance and polish.
- Open-source quality is part of the product. Commits, APIs, errors, docs, and UI copy should assume serious inference engineers will read them.

## No Hacks
- Do not introduce local hacks, monkey patches, fake runtime states, fake benchmarks, fake marketplace eligibility, or partial security shortcuts.
- If an engine, signing path, update path, or OS integration is not implemented, represent it explicitly as unavailable or unimplemented.
- Local endpoints must bind to loopback by default. LAN exposure must require explicit configuration.
- Keep inference engines behind adapters with capability detection, health checks, logs, and provenance.
- Do not create marketplace behavior until pairing, identity, tenant isolation, policy, and benchmark gates exist.

## Engineering
- Rust code must compile without warnings.
- Prefer typed contracts over unstructured JSON.
- Use clear module boundaries: hardware, engines, endpoint policy, supervisor state, and API.
- Do not use `unsafe` unless there is a documented design reason and a contained abstraction.
- Do not use `unwrap` or `expect` in production paths.

## Verification
- Run `cargo fmt --check`.
- Run `cargo test`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

## Git
- Commit messages are one line, max 10 words, format `type: short description`.
- Do not mention AI assistants, generated code, or co-authors.
