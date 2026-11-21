# ADR 0002: Versioned TOML profile schema and validation gate

- Status: Accepted for Phase 1
- Date: 2026-11-21

## Context

The Phase 0 CLI only printed help and version information. Later process execution must not accept partially parsed configuration or launch a target before version, path, field, and resource policy checks have succeeded. Phase 1 needs a user-visible profile validation command without introducing a runner, protocol client, or process API.

## Decisions

### One strict, versioned TOML format

**Decision:** Accept a local TOML document with required `schema_version = 1`, profile identity, explicit executable path, string-array arguments, explicit working directory, environment-variable name allowlist, protocol mode, and a `[limits]` table. Reject missing and unknown fields, wrong primitive types, malformed TOML, and unsupported schema versions. Do not apply implicit defaults.

**Alternatives considered:** JSON or YAML; permissive unknown-field handling; defaults for omitted policy limits; embedding an executable command string.

**Rationale:** TOML is readable in local hand-authored configuration and its table syntax fits the profile shape. Strict, versioned input makes configuration drift visible. An argv array preserves argument boundaries and avoids implying shell interpretation. Required limits are explicit instead of dependent on undocumented defaults.

### Separate raw input from validated data

**Decision:** Deserialize into an opaque `UnvalidatedProfile`, then consume it through semantic validation to construct `ValidatedProfile`. Keep validated fields and the constructor private; expose read-only accessors. Do not implement `Debug` on the validated profile because it contains paths and arguments.

**Alternatives considered:** Deserializing directly into the type later accepted by process code; using a public struct and relying on call-site discipline; one all-purpose parser/runner module.

**Rationale:** Type boundaries let later execution APIs require evidence that validation ran. Separate schema and semantic checks are testable responsibilities, while one Cargo package remains appropriate for the current code size.

### Schema and semantic bounds

**Decision:** Cap profile input at 1 MiB. For schema version 1, accept:

| Field or constraint | Bound |
| --- | --- |
| Profile name | 1–128 UTF-8 bytes; no surrounding whitespace or control characters |
| Paths | 1–4,096 UTF-8 bytes; valid Unicode; host-absolute; no NUL/control characters |
| Working directory | Must exist and resolve to a directory |
| Arguments | 0–128 entries; 8,192 bytes per entry; 16,384 combined bytes including one separator per entry; no NUL |
| Environment names | 0–64 entries; 1–128 ASCII bytes; `[A-Za-z_][A-Za-z0-9_]*`; duplicates compared case-insensitively on Windows and case-sensitively elsewhere |
| `execution_timeout_ms` | 1,000–1,800,000 ms |
| `graceful_shutdown_timeout_ms` | 100–30,000 ms |
| `stdout_capture_bytes` | 1,024–267,386,880 bytes (255 MiB) |
| `stderr_capture_bytes` | 1,024–1,048,576 bytes (1 MiB) |
| Combined stdout/stderr capture limit | At most 268,435,456 bytes (256 MiB) |
| `max_protocol_frame_bytes` | 1,024–8,388,608 bytes (8 MiB) |
| `max_lifecycle_cycles` | 1–100 cycles |

Protocol mode accepts exactly `modern`, `legacy`, or `auto`. This phase does not use the mode to communicate with a server. No limit is interpreted as unlimited.

**Alternatives considered:** Smaller arbitrary limits that would make 256 MiB unreachable; independent output caps whose sum could exceed the documented run budget; requiring the executable to exist during validation.

**Rationale:** The chosen stdout maximum plus stderr maximum equals the existing 256 MiB total output budget. All quantities have explicit units and are finite. The executable is not statted because validation is not a trust check and filesystem checks would be vulnerable to changes before a future launch; it must still be absolute. The working directory is checked now to catch a clear configuration error, with revalidation still required before future execution.

### Safe validation errors and environment allowlist

**Decision:** Store environment names only, never environment values. Return structured field errors and print safe field/constraint context without raw profile values, environment values, or the profile file path. Use bounded input and escape/truncate unknown keys in diagnostics.

**Alternatives considered:** Echoing TOML parser details or field values; embedding environment values; adding secret references in this phase.

**Rationale:** Values may contain credentials or local paths. Safe errors can still identify which field and rule failed. Future code must resolve only explicitly allowlisted names, avoid logging their values, and revisit secret handling before launch support.

### Serde and TOML dependencies

**Decision:** Add Serde with derive support and the TOML crate as direct dependencies; commit the resolved lockfile.

**Alternatives considered:** Hand-written TOML parsing; adding a CLI framework or a larger validation framework.

**Rationale:** Serde/TOML are established Rust libraries for structured configuration. A TOML value-tree preflight is used before typed deserialization so user input errors can include field context without echoing raw values. No additional framework is needed for one command.

### Validation does not execute

**Decision:** `mcp-lifecycle validate <profile.toml>` reads, parses, and validates only. It must not spawn, launch, probe, or execute the profile's target. This phase adds no process-management dependency or API.

**Alternatives considered:** Reusing a future runner's startup path to “test” that an executable is launchable; testing by executing the configured path.

**Rationale:** Validation has a clear side-effect-free boundary and cannot safely establish executable trust. A sentinel fixture verifies that the CLI accepts a configured executable path without changing or running it.

## Consequences

- The CLI reports schema version on success but does not echo profile identity, paths, arguments, or environment names.
- Future run code must accept `ValidatedProfile` and independently enforce runtime time/output limits; validating their declarations alone does not enforce them.
- Schema changes must update this ADR, README example and bounds, architecture, tests, and schema versioning policy.
- Validation does not sandbox a target or guarantee time-of-check/time-of-use safety.

## Unresolved questions

- Should future schema revisions support a migration command, or require users to edit profiles explicitly?
- Should path policy include optional allowed roots, recognizing that this would still not create an OS sandbox?
- Should the profile define a reviewed secret-reference mechanism, or should future execution obtain secrets only through an external caller interface?
- Should executable existence or file identity be captured immediately before launch, and what platform-specific APIs can reduce replacement races?
- Should stdout and stderr capture budgets be independent profile fields when the supervisor's retained memory policy is implemented, or should the current sum represent a strict combined byte budget?
