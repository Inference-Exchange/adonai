# Contributing to Adonai

Adonai is early. The project needs people who care about local inference, operating-system-grade UX, hardware truth, and safe agent runtimes.

## Best First Contributions

- Run the early testing guide on your machine and file a report.
- Fix engine detection for an installed runtime Adonai misses.
- Improve hardware detection without overstating capability.
- Add tests around model planning or endpoint policy.
- Improve setup docs when a step is unclear.

Start with [docs/early-testing.md](docs/early-testing.md).

## Product Principles

- Local first, loopback by default.
- Do not fake engine availability, benchmarks, model support, or marketplace behavior.
- Explain why Adonai chose an engine or refused to run something.
- Keep inference engines behind adapters.
- Make hardware understandable before making it look impressive.

## Development

```sh
bun install
cargo test
bun run typecheck
```

Before opening a pull request:

```sh
cargo fmt --check
cargo test
cargo clippy --workspace --all-targets -- -D warnings
bun run typecheck
```

For live testing:

```sh
ADONAI_RUN_DB=/tmp/adonai-test-runs.db cargo run -p adonai-supervisor
bun run init:check
```

## Pull Requests

- Keep PRs narrow.
- Include tests for behavior changes.
- Update public docs when the change affects architecture, shipped behavior, or user setup.
- Be explicit about what is still missing.
- Do not include generated co-author lines.

## Commit Messages

Use one-line commit messages:

```text
type: short description
```

Keep the subject under ten words.
