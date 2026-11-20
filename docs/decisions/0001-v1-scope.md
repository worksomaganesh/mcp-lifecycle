# ADR 0001: V1 scope and architecture foundation

- Status: Accepted for the Phase 0 foundation
- Date: 2026-11-20

## Context

MCP Lifecycle & Leak Lab is a local-first Rust CLI for measuring stdio MCP server lifecycle reliability. The repository was empty when bootstrapping began. The active development environment is 64-bit Microsoft Windows, OS build **10.0.26300**.

The protocol changed materially in revision **2026-07-28**. That revision uses a stateless modern lifecycle with per-request protocol metadata and no initialize handshake. Revisions through **2025-11-25** use the initialization handshake. Stdio is newline-delimited JSON-RPC over child stdin/stdout, with stderr available for logging.

## Decisions

### One Cargo package with a library and CLI binary

**Decision:** Use one Cargo package named **mcp-lifecycle**. Keep the CLI adapter in **src/main.rs** and reusable behavior in **src/lib.rs**. Add domain modules only when a later phase implements a real responsibility.

**Alternatives considered:** Multiple workspace crates; a binary-only crate; adding a CLI framework immediately.

**Rationale:** There is no independently versioned library or other consumer. A library target gives the CLI behavior a clean test boundary without a premature workspace. Help and version parsing need no runtime dependency.

### Dual-era MCP support is the V1 compatibility target

**Decision:** V1 will cover modern protocol revision **2026-07-28** and the latest handshake-era revision **2025-11-25**. Modern support is implemented first; legacy handshake support follows before claiming V1 dual-era compatibility. Older handshake-era revisions are not initially in the support target unless later evidence requires them.

Profiles will select **modern**, **legacy**, or **auto** explicitly. The mode and exact version used are recorded in each report. Modern requests carry their pinned version in MCP metadata; legacy clients pin **2025-11-25** in initialize and accept only a version the client explicitly supports. Auto mode follows the official stdio probe/fallback behavior and records which path was selected.

**Alternatives considered:** Modern-only V1; legacy-only V1; implementing both eras at once; automatic selection without profile control.

**Rationale:** Both eras matter for the intended local server population, while their lifecycle semantics differ enough that silently treating them as one would make results hard to interpret. Staging reduces initial complexity; withholding the V1 compatibility claim until both paths are implemented avoids overstating support. Explicit mode selection makes runs reproducible; explicit auto remains useful for testing compatibility behavior.

**References:** [2026-07-28 versioning](https://modelcontextprotocol.io/specification/2026-07-28/basic/versioning), [2026-07-28 stdio](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/stdio), and [2025-11-25 lifecycle](https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle).

### Windows is the primary initial development platform

**Decision:** Windows x64 is the primary development and initial verification platform because it is the inspected environment. The available verification toolchain used the Windows GNU ABI; the MSVC ABI and runtime execution remain unverified. Linux and macOS remain unverified and unsupported until their process supervision and process-tree behavior are implemented and tested.

**Initial supervision target:** bounded direct-child stdio, explicit timeouts, graceful stdin-close shutdown, forced termination after a grace period, and awaited process completion. The intended Windows process-tree boundary is a Job Object assigned before the target executes user code, with breakaway disabled. Failure to establish the required job must fail closed. These are future acceptance requirements; Phase 0 implements none of them.

**Alternatives considered:** Claiming cross-platform support at scaffold time; limiting V1 to direct-child termination; using process enumeration as a universal tree cleanup mechanism.

**Rationale:** OS process-tree semantics differ. A direct-child kill is not a tree guarantee. Windows Job Objects provide a native group boundary and accounting, but assignment and nesting constraints require platform-specific code and tests.

**Reference:** [Microsoft Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects).

### V1 observation semantics

**Decision:** Reports distinguish direct-child and process-tree scopes and classify every observation as available, unsupported, or failed. Failed observations include a reason. No unavailable observation is serialized as zero.

The planned common observations are monotonic elapsed time by lifecycle phase, process identifier, exit status, stdout/stderr byte counts, and truncation state. Windows direct-child CPU time and working-set observations are labeled direct-child. Job Object process count and cumulative CPU are labeled process-tree. Job Object peak job memory is labeled as an OS-reported peak, not RSS. Any sampled aggregate process working-set total is labeled with sample interval and snapshot scope. Linux/macOS resource metrics remain unsupported until separately implemented and tested.

**Alternatives considered:** A single generic memory or CPU number; reporting zero for unsupported metrics; deriving all descendants from occasional process-list snapshots.

**Rationale:** Process-tree membership is dynamic and OS metrics differ in meaning. Explicit scope and collection method let readers compare only measurements that mean the same thing. Windows provides direct process memory counters and Job Object accounting; these values still represent distinct measures. See [Microsoft process memory usage](https://learn.microsoft.com/en-us/windows/win32/psapi/process-memory-usage-information) and [Job Object accounting](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects).

### Execution policy and initial budgets

**Decision:** The future run path requires an absolute executable path, direct argv array, environment allowlist, explicit absolute existing working directory, and successful profile validation before spawn. The program does not invoke a shell or inherit the complete parent environment. The working directory is context, not a filesystem sandbox.

Initial limits are one concurrent child, 100 maximum sequential cycles, a 30-minute overall run, 15-second startup timeout (hard ceiling 60 seconds), 30-second request timeout (hard ceiling 5 minutes), 5-second graceful shutdown (hard ceiling 30 seconds), 8 MiB per protocol frame, 1 MiB retained stderr tail, and 256 MiB total child output per run.

Raw argv values, environment values, and protocol/stdout transcripts are excluded from reports by default. Profiles may mark argument indices and environment names for redaction in any future opt-in diagnostic output. The tool will not claim perfect automatic secret detection.

Profile validation is a configuration gate, not a sandbox. The target executes with the invoking user's privileges.

**Alternatives considered:** Shell command strings; PATH lookup; inherited environment; no hard cap on output or runtime; persisting complete transcripts by default.

**Rationale:** Direct argv and an allowlisted environment reduce ambiguity and accidental inheritance. Hard ceilings bound test-induced resource use. Excluding raw values and transcripts reduces accidental disclosure.

## Consequences

- Phase 0 must not launch a target.
- Later run code must consume a validated profile type, not raw profile input.
- The first protocol phase is modern **2026-07-28**; legacy and auto compatibility follow.
- Windows Job Object containment and metrics must have dedicated integration tests before they are described as working.
- Reports must preserve measurement availability, scope, units, method, and protocol version.
- No database, service, telemetry, or hosted dependency is part of V1.

## Unresolved questions

- Which older handshake-era versions, if any, should be accepted when a server counters **2025-11-25** with an older revision?
- Should the MCP driver use the official Rust SDK if it supports the selected modern and legacy versions, or a small version-scoped client to preserve low-level lifecycle observations?
- Should a later user-level policy restrict working directories to configured allowed roots?
- Should V1 offer a safe secret-reference mechanism rather than literal environment values in profiles?
- What sampling interval provides useful leak trends without excessive measurement overhead?
- Should optional diagnostic files ever contain redacted stderr, and what exact retention and permission policy should apply?
- Do supported Windows execution environments in CI permit Job Object assignment when the test runner is already inside a Job Object?
