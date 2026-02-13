# iMi User Perspective Audit (Human + Agent)

Date: 2026-02-06
Scope: Current executable CLI behavior vs documented/legacy BMAD expectations.

## Executive Summary

iMi is currently strongest as an entity-office worktree manager with optional agent lock ownership (`claim`/`release`/`verify-lock`).

The documented identity/workspace model (`imi workspace ...`, `imi entity ...`, token-required flow) is not implemented in the current CLI surface, so user expectations can diverge quickly. The immediate alignment path is:

1. Treat `iMi` as worktree manager first.
2. Treat claim/release as the current agent coordination contract.
3. Use metadata set/get for task-source linkage and add integration coverage next.

As of this audit pass, `init` and worktree creation paths enforce office layout:
- `<workspace_root>/<entity_id>/<repo>/trunk-*` + sibling worktrees
- no out-of-office worktree creation

## What Users Can Do Today

### Human Workflow (Implemented)

- Discover projects/worktrees: `list`, `status`, `go`, `trunk`.
- Create worktrees: `add <type> <name>`, `fix`, `aiops`, `devops`, `review`.
- Close loop: `merge`, `close`, `remove`, `prune`, `sync`, `monitor`.
- Bootstrap: `init`, `project create`, `registry sync/stats`, `doctor`.

Primary source of truth for available commands:
- `src/cli.rs:25`
- `src/main.rs:230`

### Agent Workflow (Implemented)

- Claim lock ownership: `claim <worktree> --yi-id <id>`.
- Verify lock ownership: `verify-lock <worktree> --yi-id <id>`.
- Release ownership with clean git state enforcement: `release <worktree> --yi-id <id>`.
- Link task source metadata: `metadata set/get`.
- Lock files and activity logs are written through DB + `.iMi/presence` integration.

Implementation references:
- `src/cli.rs:249`
- `src/cli.rs:267`
- `src/cli.rs:282`
- `src/main.rs:1241`
- `src/main.rs:1337`
- `src/main.rs:1479`
- `src/local.rs:203`
- `src/database.rs:599`
- `src/database.rs:616`

## High-Impact Gaps

### 1) Docs/Architecture advertise commands that do not exist in CLI

Examples documented as available:
- `iMi workspace claim`
- `iMi workspace list`
- `iMi workspace audit`
- `imi entity register`

References:
- `README.md:145`
- `README.md:149`
- `README.md:155`
- `docs/SUMMARY.md:127`
- `docs/identity-service-architecture.md:262`

But there is no `Workspace` or `Entity` command in `src/cli.rs`.

### 2) Office layout rules are implemented, but docs are still mixed

Code now enforces per-entity office layout in `init` + worktree manager path checks:
- `src/init.rs`
- `src/worktree.rs`

Some docs still discuss planned workspace/entity commands as if already available.

### 3) Metadata command path exists, but coverage remains incomplete

Legacy BMAD scope expects:
- `imi metadata set`
- `imi metadata get`

References:
- `docs/bmad/sprint-plan-imi-2026-01-27.md:104`
- `docs/bmad/sprint-plan-imi-2026-01-27.md:136`

Current state:
- DB methods and CLI/handler wiring now exist.
- Task linkage can now be stored directly on worktrees.
- No dedicated claim/release/metadata integration test suite yet.

References:
- `src/database.rs:730`
- `src/database.rs:773`
- `src/main.rs`
- `src/cli.rs`

### 4) Contract confidence gap: no dedicated claim/release/metadata integration suite

No dedicated test files for end-to-end claim/release/verify-lock/metadata CLI lifecycle were found under `tests/`.

Impact:
- Higher risk when automation relies on lock ownership semantics.
- Metadata behavior can regress silently when implemented.

### 5) User-facing examples include stale invocation patterns

Examples include `iMi --repo my-project feat new-feature` which does not match current argument layout for `feat`.

Reference:
- `README.md:193`

## Recommended Canonical Usage Contract (Until IMI-6 is Done)

### Human

1. Initialize/register repo context with `iMi init`.
2. Create work via `iMi add <type> <name>` (preferred) or existing type commands.
3. Use `iMi status` + `iMi list` + `iMi go` for navigation and visibility.
4. Finish with `iMi merge` (or `iMi close` for cancellation).

### Agent

1. Select target worktree.
2. `iMi claim <worktree> --yi-id <agent-id>`.
3. Optional guardrail: `iMi verify-lock <worktree> --yi-id <agent-id>` before critical writes.
4. Complete work with clean git state.
5. `iMi release <worktree> --yi-id <agent-id>`.
6. Store ticket/task linkage in worktree metadata:
   `iMi metadata set --worktree <name> --key plane.ticket_id --value <ID>`.

## Alignment Plan (Mapped to Plane)

- `IMI-2`: Claim command baseline (Done)
- `IMI-3`: Release command baseline (Done)
- `IMI-4`: Implement metadata set CLI + wiring (Done)
- `IMI-5`: Implement metadata get CLI + wiring (Done)
- `IMI-6`: Add integration tests for claim/release/metadata lifecycle (Todo)

Mapping artifact:
- `docs/bmad/legacy-plane-ticket-map-2026-02-06.md`
- `docs/bmad/workspace-office-rules-2026-02-06.md`

## Suggested Definition of “Aligned”

iMi is considered aligned for human+agent usage when all are true:

1. CLI help, README, and architecture docs expose the same command surface.
2. Metadata set/get is available in CLI and tested.
3. Claim/release/verify-lock and metadata flows have integration coverage.
4. One canonical workflow doc exists for human and agent paths, linked from `README.md`.
