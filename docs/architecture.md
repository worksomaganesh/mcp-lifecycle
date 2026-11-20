# Architecture

## Current foundation

The repository is a single Rust Cargo package with two targets:

- **src/main.rs** is the binary adapter. It reads command-line arguments, locks stdout and stderr, delegates to the library, and returns the library's exit code.
- **src/lib.rs** contains the minimal help/version command behavior and unit tests for that behavior.

There are no runtime dependencies. This keeps Phase 0 small and leaves dependency selection until the protocol and process-supervision implementation can be evaluated against the pinned requirements.

The library and CLI type-check cleanly, and all three library unit tests pass. Full `cargo test` could not complete because Windows Application Control blocked `rustdoc` at the doctest step. Direct end-to-end execution of the CLI help/version paths has not been verified.

## Intended component responsibilities

The planned flow is:

1. **Profile input and validation:** parse a versioned local profile, reject unknown or invalid settings, resolve configured paths, and produce a validated value. The runner must accept only this validated value.
2. **Scenario runner:** execute a bounded, sequential set of startup, protocol, shutdown, and restart checks. The initial concurrency limit is one target process.
3. **MCP stdio driver:** own protocol message construction, newline framing, response correlation, protocol-version handling, and request deadlines. It must keep stdout protocol data separate from stderr diagnostics.
4. **Process supervisor:** launch the explicit executable with argument-array and environment settings, drain both output streams concurrently, enforce time budgets, initiate shutdown, escalate if needed, and wait for process completion.
5. **Platform observations:** collect direct-child data and, where supported, process-tree data. Every measurement includes its scope, unit, method, and availability status.
6. **Report writer:** write a versioned local report containing run inputs and observations without silently converting missing data to zero.

These components are responsibilities, not a commitment to create a separate crate or abstraction for each one. The initial project will remain one package.

## MCP protocol strategy

V1 is intended to cover both protocol eras:

- Modern protocol revision **2026-07-28**, which has no initialize handshake and carries protocol metadata on each request.
- The latest handshake-era revision, **2025-11-25**, which uses initialize, version negotiation, and notifications/initialized.

Modern support is the first protocol implementation. Legacy handshake support follows before the project claims dual-era V1 support. Once protocol code exists, profiles will select **modern**, **legacy**, or an explicit **auto** compatibility mode. Exact protocol versions and the selected mode must be included in reports. The auto mode will implement the stdio backward-compatibility probe and fallback rules; it will not infer the lifecycle from an arbitrary method failure.

The trade-off is additional protocol and test complexity in exchange for testing both current and older servers. A modern-only V1 would be simpler, but would exclude servers that still require the initialization handshake. Implementing modern first keeps the initial protocol surface small while retaining a clear path to legacy coverage. The lifecycle differences are defined by the official [versioning specification](https://modelcontextprotocol.io/specification/2026-07-28/basic/versioning) and [2025-11-25 lifecycle specification](https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle).

The 2026-07-28 stdio specification defines newline-delimited JSON-RPC messages, reserves stdout for MCP messages, allows server diagnostics on stderr, and recommends shutdown by closing stdin, waiting, then using an OS-appropriate termination mechanism if needed. The older stdio binding also requires newline-delimited JSON-RPC and separates stderr logging from protocol output. See the [2026-07-28 stdio specification](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/stdio) and [2025-11-25 transports specification](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports).

## Platform and process supervision

The inspected development environment is 64-bit Microsoft Windows, OS build **10.0.26300**. Windows is the primary development and initial verification platform. The locally installed Rust toolchain used the Windows GNU ABI; the MSVC ABI and runtime execution remain unverified. Linux and macOS are also unverified and will not be described as supported until CI and process-tree tests exist there.

The V1 supervision target is a bounded, awaited direct-child lifecycle on Windows: concurrently drain stdio, enforce deadlines, close stdin for graceful shutdown, escalate termination, and wait for exit. The intended process-tree containment backend is a Windows Job Object established before the target is allowed to execute user code, with breakaway disabled. If containment cannot be established, the run should fail closed rather than silently claim tree cleanup. Windows Job Objects can manage associated processes and aggregate accounting, but process assignment and nested-job constraints must be handled explicitly. See [Microsoft's Job Objects documentation](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects).

No process supervisor or containment backend exists in this foundation. The guarantees above are design targets only; they are not yet implemented or verified.

## Observation contract

The initial report model should distinguish:

- **Direct-child observations:** process identifier, exit status, monotonic elapsed time, stdout/stderr byte counts, direct-child CPU time, and working-set measurements where available.
- **Process-tree observations:** active process count, cumulative CPU time, and peak job memory from a successfully established Windows Job Object. Process-tree working-set snapshots, if added, are sampled and must be labeled with their sampling interval. Job memory accounting and working-set measurements are different metrics and must not be merged under a generic “memory” label.
- **Unsupported observations:** a backend or platform does not provide the metric.
- **Failed observations:** a backend exists but collection failed, including a reason. This is distinct from unsupported.

Each report field must specify scope, unit, collection method, and whether it is sampled, OS-accounted peak, or cumulative. A missing or failed measurement is never serialized as numeric zero. Windows process working-set figures describe resident pages and can include shared pages; they are not equivalent to committed memory. Windows Job Objects provide aggregate process accounting and peak job-memory information. See [Microsoft's process memory documentation](https://learn.microsoft.com/en-us/windows/win32/psapi/process-memory-usage-information) and [Job Object accounting documentation](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects).

## Report reproducibility

Reports will have an explicit schema version and record the tool version, profile schema version, protocol mode and version, scenario limits, platform, run timestamps, and observation provenance. Runtime measurements naturally vary, so reproducibility means that the inputs and measurement method are recorded well enough to repeat and interpret a run; it does not mean independently measured runs produce identical output.

Raw protocol transcripts and environment values will not be included by default. A later report format must define redaction before any optional diagnostic output is persisted.
