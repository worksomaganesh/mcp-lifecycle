# MCP Lifecycle & Leak Lab

MCP Lifecycle & Leak Lab is a local-first Rust CLI for investigating lifecycle reliability in stdio-launched Model Context Protocol (MCP) servers. Its intended end state is to repeatedly start a configured server, exercise a selected MCP lifecycle, observe shutdown and resource behavior, and write a reproducible report for local review.

## Problems it addresses

MCP servers commonly run as child processes and communicate over standard input and output. A server can start slowly, exit unexpectedly, fail to follow the protocol, hang during shutdown, emit excessive output, or leave descendants behind. Repeated runs and resource observations can make reliability failures easier to reproduce and diagnose.

The intended users are MCP server developers, Rust systems engineers, and reliability or QA engineers who need repeatable local evidence about these behaviors.

## Development status

Phase 1 adds source code for a versioned TOML profile parser, strict schema checks, semantic validation, and the `validate` CLI command. The command is implemented to read and check a profile without launching, probing, or otherwise executing the configured executable. The profile's protocol mode is recorded as intent only; this release does not speak MCP.

Process supervision, an MCP client, lifecycle scenarios, resource collection, and run reports are not implemented. No hosted service, database, or telemetry backend is used. Verification results for this change are reported with the change and are not implied by this status description.

**Verification status:** `cargo fmt --check` passed. Windows Application Control blocked Cargo's dependency build scripts in the current environment (OS error 4551), so `cargo check`, Clippy, and both test commands did not complete. Runtime behavior and test results are not confirmed here.

## Initial architecture

The project is one Cargo package. `src/main.rs` is a thin command-line and stream adapter. `src/lib.rs` owns reusable CLI behavior, while `src/profile.rs` coordinates bounded TOML parsing and semantic validation. Private schema and validation modules separate deserialization from the construction of an opaque `ValidatedProfile`. The CLI can print a validation result, but no run command accepts raw or validated profiles yet.

Profiles use explicit absolute paths, argument arrays, an environment-variable name allowlist, a protocol mode, and finite resource bounds. The allowlist stores names only; future execution code may copy only those named values from the parent environment. No shell command string or environment value is stored in the schema. See [the architecture](docs/architecture.md), [the threat model](docs/threat-model.md), and [ADR 0002](docs/decisions/0002-profile-schema-validation.md) for details.

## Profile format

This complete example uses Unix-style absolute paths; replace them with paths that are absolute for the host operating system:

```toml
schema_version = 1
name = "example-mcp-server"
executable = "/usr/local/bin/example-mcp-server"
args = ["--stdio", "--log-level", "warn"]
working_directory = "/tmp"
environment_allowlist = ["HOME", "PATH"]
protocol_mode = "auto"

[limits]
execution_timeout_ms = 30000
graceful_shutdown_timeout_ms = 5000
stdout_capture_bytes = 16777216
stderr_capture_bytes = 1048576
max_protocol_frame_bytes = 8388608
max_lifecycle_cycles = 10
```

All fields are required and unknown fields are rejected. Schema version `1` is the only accepted version in this release. The current bounds are:

| Field | Accepted range |
| --- | ---: |
| `execution_timeout_ms` | 1,000–1,800,000 ms |
| `graceful_shutdown_timeout_ms` | 100–30,000 ms |
| `stdout_capture_bytes` | 1,024–267,386,880 bytes (255 MiB) |
| `stderr_capture_bytes` | 1,024–1,048,576 bytes (1 MiB) |
| Combined stdout and stderr capture | At most 268,435,456 bytes (256 MiB) |
| `max_protocol_frame_bytes` | 1,024–8,388,608 bytes (8 MiB) |
| `max_lifecycle_cycles` | 1–100 cycles |

The name is limited to 128 UTF-8 bytes. Paths must be host-absolute, valid Unicode, and no longer than 4,096 UTF-8 bytes; the working directory must exist and identify a directory. The executable path is not checked for existence. Arguments allow at most 128 entries, 8,192 UTF-8 bytes per entry, and 16,384 combined bytes including one separator per entry. The environment allowlist allows at most 64 names of up to 128 bytes each, matching `[A-Za-z_][A-Za-z0-9_]*`. Duplicate names are case-insensitive on Windows and case-sensitive elsewhere. NUL bytes are rejected from launch-relevant strings.

## Build and run

Install Rust 1.85 or newer, then run these commands from the repository root:

```powershell
cargo check
cargo test
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo run -- --help
cargo run -- --version
cargo run -- validate path\to\profile.toml
```

The `validate` command exits with status `0` when the profile is valid, `1` when a profile cannot be read or is invalid, and `2` for CLI usage errors. Error messages identify fields and constraints without printing profile values. Validation is not an execution safety boundary; see the [threat model](docs/threat-model.md).

## Planned, not implemented

- Modern and legacy MCP stdio protocol clients.
- Bounded asynchronous process supervision, output draining, cancellation, and cleanup.
- Repeated lifecycle scenarios and tested process-tree containment.
- CPU, memory, and process observations with explicit availability, scope, and provenance.
- Reproducible, versioned local run reports.

The primary development environment is Windows x64. Process supervision and process-tree behavior have not been implemented, and Linux, macOS, Windows MSVC, and cross-platform runtime guarantees remain unverified. See [the architecture](docs/architecture.md) for current platform boundaries.
