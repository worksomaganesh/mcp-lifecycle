# AI engineering workforce

## Purpose and current status

This workforce supports the local-first Rust CLI in `MCP Lifecycle & Leak Lab`. The primary Codex session remains responsible for scope, delegation, final integration, verification, and the user-facing report. Specialists are project-scoped roles; they do not imply that the application has implemented their areas of expertise.

The application now contains Phase 1 TOML profile parsing and validation source in addition to help and version output; compile and test verification remain outstanding. It does not implement a process supervisor, MCP client, lifecycle scenarios, resource collector, or report writer. Specialist roles describe review and engineering responsibilities; they do not imply that those application features exist.

## Roles

| Codex role | Responsibility | Source and adaptation | Authority |
| --- | --- | --- | --- |
| `rust_systems_engineer` | Implement assigned Rust changes; reason about ownership, async I/O, process lifecycle, cancellation, Windows APIs, errors, and compiler/Clippy results. | Adapted from Agency `engineering-rust-refactoring-specialist.md`; narrowed from repository-wide refactoring to this single-package process-testing CLI. | Only role designated to write, and only after explicit assignment in a parent workspace-write session. |
| `mcp_protocol_researcher` | Research official MCP versions, lifecycle behavior, JSON-RPC framing, stdio rules, and compatibility behavior with exact citations. | Project-specific role; informed by the Agency technical-writer emphasis on accuracy and traceable documentation. | Read-only by workflow. |
| `process_reliability_engineer` | Review child startup/shutdown, cancellation, reaping, containment, failure classification, and observation validity. | Adapted from Agency `engineering-sre.md`; cloud SLOs, deployment operations, and telemetry stacks are excluded. | Read-only by workflow. |
| `systems_test_engineer` | Design deterministic Rust process fixtures and tests for hangs, output floods, descendants, cancellation races, cleanup, and platform behavior. | Project-specific role; informed by test and automation practices in Agency `engineering-developer-tooling-engineer.md` and `engineering-sre.md`. | Read-only; test design and review only. |
| `application_security_reviewer` | Threat model target execution, profile validation, paths, argv, environment, output limits, secrets, and artifacts. | Adapted from Agency `security-appsec-engineer.md`; scoped to local executable-launch risks. | Read-only by workflow. |
| `independent_code_reviewer` | Independently inspect diffs for correctness, races, resource leaks, cancellation, security, evidence, and missing tests. | Adapted from Agency `engineering-code-reviewer.md`; review findings require concrete code evidence and line references. | Read-only by workflow. |

Agency `engineering-technical-writer.md` was also studied. Its accuracy and reader-oriented documentation practices informed this workforce guide. No separate writer role was added because the requested roster contains six specialists and the primary session owns final integration and reporting.

## Upstream attribution

The Agency prompts were studied from [msitarzewski/agency-agents](https://github.com/msitarzewski/agency-agents), commit `8ae6164602c95ca78828b148b9052460ee426885`, under the upstream MIT license (Copyright (c) 2025 AgentLand Contributors). The prompts in `.codex/agents/` are project-specific adaptations and do not vendor the upstream repository.

The upstream Codex integration describes one TOML file per agent with `name`, `description`, and `developer_instructions`. Its converter source was inspected at `scripts/convert.sh`; its installer usage and Codex destination behavior were inspected in `scripts/install.sh`. The converter could not be executed in this Windows sandbox: Git Bash failed while creating an MSYS named object (`NtCreateDirectoryObject`, access denied). Escalated execution is disabled in this session. No installer was run and no global Codex agent directory was touched. The local role files were authored directly in the documented three-field format.

## Codex configuration and permission limits

`.codex/config.toml` enables subagent tools, caps spawned agents at three concurrent threads, and selects `read-only` as the project session default. Each specialist has a standalone TOML file under `.codex/agents/`. The five research/review roles also set `features.request_permissions_tool = false` and disable unrelated apps, plugins, and memories while retaining command access needed to inspect a repository.

The configuration keys and role behavior were checked against the [official Codex config schema](https://github.com/openai/codex/blob/main/codex-rs/core/config.schema.json) and [Codex role resolution](https://github.com/openai/codex/blob/main/codex-rs/core/src/agent/role.rs).

**Codex custom-agent files inherit the parent session sandbox. A role file cannot force a read-only sandbox or grant workspace-write to an individual agent.** Therefore the project cannot technically give one agent workspace-write while making its peers read-only inside the same write-enabled parent session.

The enforceable workflow is:

1. Start and keep research, test-planning, and review work in a read-only parent session. Those child agents inherit read-only access; their permission-request tool is also disabled.
2. For an explicitly authorized implementation, use `rust_systems_engineer` only from a parent session that has workspace-write. Do not spawn other agents from that write-enabled session, because their effective sandbox would also inherit write access.
3. Have the primary session inspect the full diff and perform final integration and requested verification.

The role names express organizational responsibility; they are not an OS security boundary. A parent session or CLI option can override the project default, and the role instructions cannot constrain a more permissive parent. Do not describe this setup as per-agent sandbox enforcement.

## Delegation workflow

- Give each specialist one bounded question or change with the relevant files and expected evidence.
- Delegate independent research or review questions in parallel only when they do not overlap; the project limit is three spawned threads.
- Ask research roles for official source links and exact versions. Ask reliability and security reviewers for code locations, preconditions, impact, and limitations. Ask the test engineer for setup, trigger, expected result, bounds, cleanup, and observable evidence per test.
- Use the Rust Systems Engineer for one explicitly scoped implementation at a time. Do not let multiple agents edit the same files concurrently.
- The primary session reconciles findings, inspects actual diffs, runs user-requested verification, and reports what was and was not validated.

## Invocation

In Codex, ask for a role by its configured name, for example: “Use `mcp_protocol_researcher` to verify the 2026-07-28 stdio shutdown requirements against the official specification.” Other configured names are `rust_systems_engineer`, `process_reliability_engineer`, `systems_test_engineer`, `application_security_reviewer`, and `independent_code_reviewer`.

Role availability depends on Codex loading this project configuration and the project being trusted by the current Codex installation. The files were created and syntax-checked in this repository; this phase did not invoke or test subagent spawning.

## Unresolved limitations

- Codex's parent-inherited sandbox prevents hard per-agent permission separation. A future supported per-agent isolation mechanism would be needed to provide a write-enabled Rust specialist and simultaneously enforce read-only peers in the same parent session.
- The Agency converter could not run in this sandbox, so no output from its executable pipeline was generated or compared. The three-field format was taken from its source and Codex integration documentation.
- Agent discovery and actual spawning were not exercised. MCP protocol claims still require fresh official-spec research during protocol implementation.
