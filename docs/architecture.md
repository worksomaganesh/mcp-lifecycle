# Architecture

## Current foundation

The project is one Rust Cargo package with a library and CLI binary:

- **`src/main.rs`** adapts process arguments, stdout, stderr, and exit status to the library.
- **`src/lib.rs`** implements help, version, and `validate` command behavior. The CLI does not spawn a configured target.
- **`src/profile.rs`** exposes the profile parsing and validation API and owns the bounded input size and structured error type.
- **`src/profile/schema.rs`** deserializes strict TOML into an opaque `UnvalidatedProfile`, rejects missing and unknown fields, checks primitive types, and reports syntax locations without echoing profile values.
- **`src/profile/validation.rs`** applies semantic constraints and privately constructs `ValidatedProfile`. The working directory must exist and be a directory. The executable path must be absolute but is not statted or executed.
- **`Cargo.toml`** uses Serde derive for typed deserialization and TOML for parsing. These are the only direct runtime dependencies introduced for the profile boundary.

This separation gives tests a library boundary and ensures future run code can be required to accept a validated value. It does not create one module or crate per planned feature.

The code implements the validation path but has not completed compilation, test, or Clippy verification in the current Windows environment: Windows Application Control blocked Cargo dependency build scripts (OS error 4551). `cargo fmt --check` passed. No runtime verification claim is made; see the [README verification status](../README.md#development-status).

## Profile contract and validation data flow

The local TOML schema has required top-level fields `schema_version`, `name`, `executable`, `args`, `working_directory`, `environment_allowlist`, `protocol_mode`, and a `[limits]` table. All unknown fields are rejected. Version `1` is the only accepted schema version; schema changes that are incompatible require a version change rather than silently reinterpreting old profiles. There are no implicit field defaults.

The profile input is capped at 1 MiB. The semantic bounds for version 1 are:

| Constraint | Range or rule |
| --- | --- |
| `name` | 1–128 UTF-8 bytes; no surrounding whitespace or control characters |
| Paths | Absolute for the host OS, valid Unicode, 1–4,096 UTF-8 bytes; no NUL/control characters |
| Working directory | Existing directory |
| Executable | Explicit absolute path; existence and trust are not checked |
| `args` | 0–128 strings; at most 8,192 bytes each and 16,384 combined bytes including one separator per entry; no NUL |
| `environment_allowlist` | 0–64 names; each 1–128 ASCII bytes and matches `[A-Za-z_][A-Za-z0-9_]*`; no duplicates (Windows comparison is ASCII case-insensitive) |
| `protocol_mode` | Exactly `modern`, `legacy`, or `auto` |
| `execution_timeout_ms` | 1,000–1,800,000 milliseconds |
| `graceful_shutdown_timeout_ms` | 100–30,000 milliseconds |
| `stdout_capture_bytes` | 1,024–267,386,880 bytes (255 MiB) |
| `stderr_capture_bytes` | 1,024–1,048,576 bytes (1 MiB) |
| Combined capture limits | At most 268,435,456 bytes (256 MiB) |
| `max_protocol_frame_bytes` | 1,024–8,388,608 bytes (8 MiB) |
| `max_lifecycle_cycles` | 1–100 cycles |

The data flow is:

1. `read_profile` reads at most 1 MiB plus one byte, rejects oversized or non-UTF-8 input, and parses TOML.
2. Strict shape, required fields, field types, and schema version are checked before deserialization into `UnvalidatedProfile`.
3. `UnvalidatedProfile::validate` applies path, string, allowlist, protocol-mode, and numeric bounds.
4. Only successful validation produces an opaque `ValidatedProfile`; its constructor and fields are private, it has no `Debug` implementation, and its public accessors expose the accepted settings to future code.
5. `mcp-lifecycle validate <profile.toml>` prints a concise success line or a structured safe error. It does not invoke any process API.

The environment list holds names, not values. A future launcher must resolve only the listed names from the caller's environment and must not dump resolved values to diagnostics or reports. Numeric fields have explicit units and finite bounds. Limits are validation policy only until a future supervisor applies them.

## Intended later component responsibilities

The planned flow remains within this package:

1. **Profile input and validation:** implemented for schema version 1. A future runner must accept only `ValidatedProfile`.
2. **Scenario runner:** future bounded sequential startup, protocol, shutdown, and restart checks; initial concurrency target is one child.
3. **MCP stdio driver:** future JSON-RPC framing, response correlation, protocol-version behavior, and deadlines. Stdout protocol messages must remain separate from stderr diagnostics.
4. **Process supervisor:** future explicit executable launch, concurrent stream draining, timeout and cancellation handling, graceful shutdown, termination escalation, and awaited reaping.
5. **Platform observations:** future direct-child and process-tree observations with explicit scope, unit, collection method, and status.
6. **Report writer:** future versioned local reports that record inputs and observation provenance without converting unavailable data to zero.

Responsibilities do not imply that the associated components exist yet.

## MCP protocol strategy

The compatibility target remains dual-era, implemented incrementally:

- Modern protocol revision **2026-07-28** has no `initialize`/`notifications/initialized` handshake. Each request carries protocol version and related metadata in `_meta`.
- The handshake era through revision **2025-11-25** begins with `initialize`, which negotiates version and capabilities, followed by the `notifications/initialized` notification.
- The profile's `modern`, `legacy`, and `auto` settings are parsed and validated only. There is no protocol client in this phase.

The 2026-07-28 versioning specification calls the first mode Modern and versions through 2025-11-25 Legacy. For stdio dual-era detection, it specifies probing with `server/discover`: a modern result or recognized modern protocol error identifies the modern era, while another error or timeout falls back to the initialization handshake. This is not equivalent to treating every failure as evidence of a legacy server. The future `auto` behavior must pin both the preferred modern revision and the exact fallback revision and record the selected path in reports.

Modern support is planned first; legacy handshake support must be implemented and tested before claiming dual-era support. Implementing both immediately would broaden protocol and fixture requirements. Modern-first enables an incremental client, while retaining legacy as an explicit target avoids excluding handshake-era servers. A modern-only V1 would be simpler but would leave older servers out of scope. Exact protocol versions must be pinned in code and reports rather than following an unversioned “latest” value.

Official references:

- [2026-07-28 versioning and compatibility](https://modelcontextprotocol.io/specification/2026-07-28/basic/versioning)
- [2026-07-28 stdio transport](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/stdio)
- [2025-11-25 lifecycle](https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle)
- [2025-11-25 transports](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports)

Both stdio revisions use newline-delimited JSON-RPC over standard streams and reserve stdout for protocol messages; stderr may carry logs. The 2026-07-28 stdio shutdown guidance is to close the child's input stream, wait, then forcibly terminate with an OS-appropriate mechanism if needed. A future supervisor must implement and test those rules rather than assuming protocol validation provides lifecycle handling.

## Platform and process supervision

The inspected development environment is 64-bit Microsoft Windows, OS build **10.0.26300**. Windows x64 is the primary development and initial verification platform. The installed Rust toolchain uses the Windows GNU ABI. Windows runtime checks may be constrained by Windows Application Control; no runtime support claim follows from compilation alone. Windows MSVC, Linux, and macOS remain unverified.

No process supervisor, process-tree containment, or resource observation exists in Phase 1. The design target for Windows is a bounded, awaited direct-child lifecycle, with a Windows Job Object established before target code runs and breakaway disabled when tree containment is claimed. Assignment failures must fail closed. Actual guarantees need native implementation and integration tests. Direct-child termination alone cannot be reported as tree cleanup. Job membership limits and nesting behavior require explicit handling; see [Microsoft Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects).

## Observation semantics

Future reports must distinguish:

- **Direct-child observations:** process identifier, exit status, elapsed time, stream byte counts, and supported direct-process CPU or memory metrics.
- **Process-tree observations:** process count, aggregate CPU, and OS-reported Job Object peak memory when collected from an established Windows Job Object.
- **Sampled observations:** for example, process-tree working-set snapshots, with sampling period, method, and scope stated. They may miss short-lived descendants and are not equivalent to a kernel-accounted peak.
- **Unsupported observations:** no implementation/backend exists for the metric on this platform.
- **Failed observations:** a collector exists but could not obtain a measurement; include a safe reason.

Unavailable, unsupported, or failed observations must never be serialized as zero. Each numeric observation must preserve scope, unit, collection method, sampling/aggregation semantics, and status. Working set, committed memory, and Job Object peak memory are different measures. See [Microsoft process memory usage](https://learn.microsoft.com/en-us/windows/win32/psapi/process-memory-usage-information) and [Job Object accounting](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects).

## Report reproducibility

Future reports will have an explicit report schema version and record tool version, profile schema version, protocol mode and exact protocol version, scenario limits, platform, run timestamps, and observation provenance. Runtime measurements naturally vary; reproducibility means recording enough inputs and measurement methods to repeat and interpret a run, not producing identical values from separate executions.

Raw protocol transcripts and environment values are excluded by default. Any later optional diagnostic output needs a redaction and retention policy before it is implemented.
