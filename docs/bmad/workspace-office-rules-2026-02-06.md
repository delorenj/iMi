# iMi Workspace Office Rules (2026-02-06)

## Intent

Every human/agent works from their own isolated office clone. No shared master clone.

## Canonical Layout

`<workspace_root>/<entity_id>/<repo>/`

Inside each repo office:
- `trunk-<default_branch>/` (entity-owned clone root)
- `feat-*`, `fix-*`, `aiops-*`, `devops-*`, `pr-*` worktrees as siblings to trunk

Example:

```
~/33GOD/workspaces/delorenj/iMi/
├── trunk-main/
├── feat-claim-lock-fix/
└── fix-path-validation/
```

## Config Inputs

From `~/.config/iMi/config.toml`:

```toml
[workspace_settings]
root_path = "/home/you/33GOD/workspaces"
entity_id = "delorenj"
```

Defaults:
- `root_path`: `~/33GOD/workspaces` (or `IMI_WORKSPACE_ROOT` env var)
- `entity_id`: `IMI_ENTITY_ID`, else `$USER`/`$USERNAME`

## Enforcement

- `iMi init` migrates mismatched repo layouts into office layout before registration.
- Repository registration rejects non-office trunk paths.
- Worktree creation/retrieval rejects repositories outside current entity office.

## Operational Notes

- Existing legacy registrations outside office layout can be migrated with:
  - `iMi migrate-office --dry-run` (preview)
  - `iMi migrate-office` (execute)
- Office isolation is path-level; claim/release remains ownership-level.
