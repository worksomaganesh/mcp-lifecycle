# Threat model

## Scope

The planned application will launch a process selected by a local profile. That process may be buggy, compromised, or intentionally hostile. It runs with the invoking user's privileges. The tool is intended for local reliability testing, not for executing untrusted code safely.

## Security limitations

Profile validation checks configuration shape and policy. It does **not** sandbox arbitrary executables, restrict filesystem or network access, prevent a target from reading user data, or guarantee that a process cannot escape an observation mechanism. Use an operating-system sandbox or disposable virtual machine when testing a target that must not have access to the host.

Path checks also do not establish binary trust. An explicit executable path avoids PATH lookup, but the file may still be malicious or replaced between validation and launch. Validation should be repeated immediately before launch and the resolved path recorded, but this is not a complete defense against local file replacement.

## Planned execution policy

- Require an explicit absolute executable path. Do not search PATH.
- Represent arguments as an array passed directly to the executable. Do not build or interpolate shell command strings.
- Clear the inherited environment and pass only profile-allowlisted names and values. Never dump environment values to diagnostics or reports.
- Require an explicit absolute existing working directory. Do not inherit the caller's current directory or expand shell, home-directory, or environment-variable syntax. The working directory is not filesystem confinement.
- Parse and validate the versioned profile on every run before creating any child process. Reject unsupported schema versions, unknown fields, invalid paths, invalid bounds, and unsupported protocol versions before launch.
- Limit execution to one concurrent child, at most 100 sequential cycles, a 30-minute overall run, 15-second startup timeout (hard ceiling 60 seconds), 30-second request timeout (hard ceiling 5 minutes), and 5-second graceful-shutdown wait (hard ceiling 30 seconds).
- Limit each protocol frame to 8 MiB, retain at most a 1 MiB stderr tail in memory, and stop a run if total child output exceeds 256 MiB. Continue draining both pipes while within the byte budget so output cannot block the child.
- Do not persist raw stdout protocol data or raw stderr by default. Reports omit raw argument and environment values. Any later opt-in diagnostic output must apply configured argument-index and environment-name redaction before persistence. Automatic secret detection is not reliable.

These limits are initial policy values. They should be exercised against controlled fixtures before a run command is implemented; changing them requires updating the decision record and report schema as appropriate.

## Process-tree boundary

The V1 target is a Windows Job Object established before target code can spawn descendants. The tool should disable breakaway and fail closed if job assignment is unavailable. It must not describe termination of the direct child as tree cleanup. Jobs cannot prove containment of processes deliberately created outside their association or through APIs that do not inherit job membership. Linux and macOS behavior is unimplemented and unverified.

## Data handling

Profiles and reports are local files. Profiles may contain executable paths, arguments, and environment values. Keep them under user-controlled filesystem permissions and do not commit secrets. The application should avoid echoing profile values in errors, help text, or reports. Reports may still reveal paths, timing, process identifiers, and server behavior; users should review them before sharing.

## Current behavior

The code implements help and version output only. Its three library unit tests pass. Full `cargo test` could not complete because Windows Application Control blocked `rustdoc` at the doctest step. The CLI executable's help/version behavior has not been verified end to end. It does not parse profiles, access configured paths, launch processes, capture MCP messages, or produce reports. The controls in this document describe the intended future run path, not implemented protections.
