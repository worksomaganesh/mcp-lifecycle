# MCP Lifecycle & Leak Lab

MCP Lifecycle & Leak Lab is a local-first Rust CLI foundation for investigating lifecycle reliability in stdio-launched Model Context Protocol (MCP) servers. The intended tool will repeatedly start a configured server, exercise its selected MCP lifecycle, observe shutdown and resource behavior, and produce a versioned report that can be reviewed locally.

## Problems it is intended to address

MCP servers are often launched as child processes and communicate over stdin and stdout. A server may start slowly, exit unexpectedly, fail to follow the protocol, hang during shutdown, emit excessive output, or leave processes behind. Repeated runs can also reveal resource trends that a single successful connection does not show.

The planned CLI is intended for MCP server developers, Rust systems engineers, and reliability or QA engineers who need repeatable local evidence about these behaviors.

## Development status

This repository is at the Phase 0 foundation. The code implements basic help and version output only. It does not parse profiles, launch a process, speak MCP, collect resource measurements, or create run reports. No MCP server or arbitrary configured executable is launched by the current code. All three library unit tests pass. The full `cargo test` command remains incomplete because Windows Application Control blocked `rustdoc` during the doctest step; the CLI executable's help/version behavior has not been directly verified end to end.

## Initial architecture

The project is one Cargo package with a small library and a thin CLI binary. The library owns reusable command behavior; the binary only adapts process arguments, standard streams, and exit status. Later work is planned to add profile parsing and validation, a protocol driver, a process supervisor, lifecycle scenarios, platform observations, and versioned reports as independently testable modules.

Every future run must parse and validate a versioned profile before creating a child process. The intended execution policy uses an explicit executable path, an argument array, an environment allowlist, an explicit working directory, and bounded time and output budgets. Profile validation is not a sandbox; the target still runs with the user's operating-system privileges.

The protocol compatibility decision is recorded in docs/decisions/0001-v1-scope.md. The project targets both MCP protocol eras in V1, with modern 2026 support implemented first and the older initialization handshake added before claiming V1 compatibility. Stdio framing and shutdown requirements are described in the official [2026-07-28 stdio specification](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/stdio).

## Build and run

Install Rust 1.85 or newer, then run these commands from the repository root:

~~~powershell
cargo check
cargo test
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo run -- --help
cargo run -- --version
~~~

The current CLI accepts only help and version options. It does not accept a profile or launch a configured target.

## Planned, not implemented

- Versioned local profiles and a validation command.
- Modern and legacy MCP stdio lifecycle support.
- Bounded asynchronous process supervision and cleanup.
- Repeated lifecycle scenarios and process-tree observations.
- Resource measurements with explicit availability, scope, and provenance.
- Reproducible, versioned local reports.

These are plans, not working features. Platform support and limitations are documented in the architecture and threat-model documents.
