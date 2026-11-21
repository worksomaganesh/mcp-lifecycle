# Threat model

## Scope and assets

MCP Lifecycle & Leak Lab is intended to launch a process selected by a local profile in a later phase. That process may be buggy, compromised, or intentionally hostile, and it would run with the invoking user's operating-system privileges unless the user supplies external isolation. The tool is for local reliability testing, not for safely executing untrusted code.

Relevant assets include the user's files and credentials, inherited environment variables, profile contents, captured process output, process-tree state, and generated reports. Profiles may expose executable paths, arguments, and allowlisted variable names. A future launcher may read the corresponding environment values.

## Current behavior and limits

The Phase 1 source implements local TOML profile parsing and validation. The `validate` code path contains no child-process launch and does not resolve environment allowlist values. It caps input at 1 MiB, rejects unknown fields, applies finite semantic bounds, and formats structured errors without printing profile values. The executable must be an absolute path but is not checked for existence or trust; the working directory must exist and be a directory. These code paths have not completed compilation or test verification in this environment because Windows Application Control blocked dependency build scripts (OS error 4551); treat runtime behavior as unverified until the checks pass.

Profile validation does **not** sandbox arbitrary executables. It does not restrict filesystem or network access, prevent a target from reading user data, establish binary trust, or guarantee that a process cannot escape a future observation mechanism. An explicit path only removes PATH-based executable lookup. A file could be malicious or replaced after validation. A working directory is not filesystem confinement. Use an operating-system sandbox or disposable virtual machine for targets that must not access the host.

Validation is also subject to filesystem time-of-check/time-of-use changes: the working directory or executable may be replaced after validation and before any future launch. Future run code must revalidate immediately before launch, use OS APIs that minimize path substitution where practical, and clearly describe the remaining limitations.

## Future execution policy

Any run implementation must:

- Require a host-absolute explicit executable path. Do not search `PATH` or invoke a shell.
- Pass arguments as an array directly to the executable. Do not concatenate or interpolate a shell command string.
- Store only an environment-variable name allowlist in the profile. Clear the inherited environment and copy only explicitly allowed names at launch time. Never print values to diagnostics or reports.
- Require an explicit absolute working directory. Do not silently inherit the CLI's current directory or expand shell, home, or environment-variable syntax. This directory is not a sandbox.
- Reject unsupported schema and protocol versions, unknown fields, invalid paths, and out-of-range limits before child creation. Run code must accept `ValidatedProfile`, not raw profile input.
- Enforce explicit finite execution and graceful-shutdown timeouts, bounded stdout/stderr capture, a bounded protocol frame size, and maximum lifecycle cycles. Future process code must continue draining both pipes without blocking even after retaining output is capped.
- Avoid retaining raw stdout protocol messages or raw stderr by default. Any future diagnostic persistence must define redaction and retention first. Automatic secret detection is not reliable.
- Redact path, argument, environment, and profile values from error messages unless an explicitly reviewed diagnostic policy says otherwise. Current validation errors expose field names and safe constraints, not supplied values.

The current profile schema uses these hard bounds: profile file 1 MiB; execution timeout 1,000–1,800,000 ms; graceful shutdown 100–30,000 ms; stdout capture 1,024–267,386,880 bytes; stderr capture 1,024–1,048,576 bytes; combined capture at most 268,435,456 bytes; protocol frame 1,024–8,388,608 bytes; lifecycle cycles 1–100. The capture limits are configuration policy only today; no process output is captured in this phase.

## Process supervision boundary

Windows x64 is the primary development and initial verification platform, but there is currently no process supervisor. The V1 design target for process-tree containment is a Windows Job Object established before target code is allowed to spawn descendants, with breakaway disabled, and fail-closed behavior if assignment cannot be established. Direct-child termination is not proof of process-tree cleanup. Job Objects also cannot prove containment for a process intentionally created outside the job association or through APIs that do not inherit it.

Windows MSVC, Linux, and macOS runtime process-tree behavior are unverified. No cross-platform cleanup guarantee may be claimed until each platform has an implemented containment strategy and integration tests. Process enumeration alone is not a reliable universal tree-cleanup mechanism.

## Data handling

Profiles and future reports are local files. Keep them under user-controlled filesystem permissions; do not commit secrets. The environment allowlist contains names only, but later copied values may be sensitive. Reports may reveal paths, process identifiers, timings, exit states, or server behavior even when secrets are removed. Review reports before sharing.

## Verification boundary

This document describes the Phase 1 behavior and requirements for future launch code. It does not assert that future process safety controls, output handling, MCP protocol behavior, resource observations, or report redaction are implemented. Test and toolchain results are reported separately for each implementation change.
