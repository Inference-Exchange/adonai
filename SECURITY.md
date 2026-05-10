# Security Policy

Adonai is an alpha. Treat it as local developer software, not a hardened production daemon.

## Supported Versions

Security fixes target the `main` branch until tagged releases exist.

## Reporting a Vulnerability

Do not open a public issue for a vulnerability.

Email: security@compounder.dev

Include:

- affected commit,
- operating system and hardware,
- reproduction steps,
- expected impact,
- whether the supervisor was loopback-only or exposed with `--allow-lan`.

## Current Security Posture

- The supervisor binds to `127.0.0.1` by default.
- Non-loopback binding requires `--allow-lan`.
- Adonai does not expose a public marketplace path.
- Adonai does not install or launch external engine processes yet.
- Secrets management is not implemented yet.

Do not run Adonai on an untrusted network with LAN exposure unless you are testing that surface intentionally.
