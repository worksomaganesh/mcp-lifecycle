# Engineering instructions

## Product and current boundary

MCP Lifecycle & Leak Lab is a local-first Rust CLI for investigating the lifecycle reliability of stdio-launched MCP servers. V1 remains one Cargo package. Do not add a hosted API, database, telemetry service, or unrelated infrastructure.

The current application is a Phase 0 foundation. It implements help and version output only. Profile parsing, process supervision, MCP client behavior, lifecycle scenarios, resource observation, and report generation are not implemented. Do not describe planned behavior as working.

## Authoritative project references

- `README.md`: user-facing purpose, current status, and build commands.
- `docs/architecture.md`: component boundaries, protocol and platform strategy, observation semantics.
- `docs/threat-model.md`: execution risks and security limits.
- `docs/decisions/0001-v1-scope.md`: accepted scope decisions and unresolved questions.
- `docs/engineering/ai-workforce.md`: agent roles, delegation policy, and Codex sandbox limitations.

If these documents disagree, surface the conflict and follow the user's latest explicit direction. Do not silently change an accepted protocol version, process guarantee, report contract, or security boundary.

## Engineering and safety principles

- Keep V1 local-first and the crate structure proportional to implemented responsibilities.
- Pin MCP protocol claims to official specification revisions. Distinguish the modern stateless 2026-07-28 lifecycle from handshake-era versions.
- Never execute a profile before validating its version, fields, paths, and resource limits. Profile validation does not sandbox an executable.
- For any future launch path, use an explicit executable path, argument array, environment allowlist, explicit working directory, execution timeout, bounded output capture, and no shell interpolation by default.
- Treat subprocess ownership, pipe draining, cancellation, termination, and reaping as correctness requirements. Do not claim process-tree cleanup without an implemented and tested OS containment boundary.
- Preserve measurement scope, unit, method, sampling, and status. Unavailable, unsupported, and failed observations must not be represented as zero.
- Keep schemas and reports versioned and reproducible. Redact sensitive values and avoid raw transcripts by default.
- Do not bypass Windows Application Control or other execution restrictions to run blocked binaries.

## Changes, verification, and Git safety

Inspect project instructions, Git status, and the relevant diff before editing. Preserve all unrelated user changes. Keep implementation scoped to the explicit request; do not add dependencies, change MSRV, alter runtime behavior, or create speculative abstractions without authorization.

When an implementation task calls for verification, run the relevant existing checks, report their actual results, and inspect the complete diff. Do not claim an unrun check passed. Do not add or run tests unless the user asks for tests or verification.

Never discard user work with reset, clean, force checkout, or similar destructive operations. Do not commit, push, publish, or deploy unless the user explicitly authorizes that operation.

## Delegation and permissions

The primary Codex session owns scope, orchestration, final integration, verification, and reporting. Delegate only independent, bounded tasks; avoid duplicate inspection and conflicting edits. At most one specialist may write at a time, and no two writers may edit the same files concurrently.

`rust_systems_engineer` is the only specialist designated to implement project changes. It may edit only after explicit task assignment and only when the parent session has workspace-write access. The other five specialists are research, planning, test-design, and review roles. Their role files disable permission requests and unrelated app, plugin, and memory features.

Codex custom-agent files cannot override the parent session's sandbox. This project defaults sessions to read-only. To enforce read-only research or review, keep the parent session read-only. A Rust implementation task requires a parent session with workspace-write; while using that session, do not spawn other agents. Role prompts are organizational guidance, not a security boundary. The primary session must inspect and integrate every change.
