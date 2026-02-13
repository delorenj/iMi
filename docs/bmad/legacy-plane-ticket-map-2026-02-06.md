# iMi Legacy BMAD -> Plane Ticket Mapping (2026-02-06)

## Scope
- Workspace: `33god`
- Project: `IMI` (`495de1b1-a4a2-4456-a185-351885858b1e`)
- Legacy sources:
- `bmad-legacy/sprint-status.yaml`
- `bmad-legacy/workflow-status.yaml`
- `docs/bmad/sprint-plan-imi-2026-01-27.md`

## Migration Rules
- `legacy status: completed` -> Plane state `Done`
- `legacy status: not_started` -> Plane state `Todo`
- Legacy points are stored in each issue description as `Legacy Points: N`.
- Effort is represented with labels (`effort:S`, `effort:M`) because numeric `estimate_point` was not accepted by this Plane instance.
- All migrated issues carry labels:
- `legacy-migration`
- `sprint-1`
- `feature`
- `agent-workflow`
- `yi-integration`

## Story Mapping
| Legacy Story | Legacy Status | Legacy Points | Plane Ticket | Plane Issue UUID | API Resource |
|---|---|---:|---|---|---|
| STORY-001 | completed | 5 | IMI-2 | `814afb44-e484-4771-806c-aed6add2f81c` | `/api/v1/workspaces/33god/projects/495de1b1-a4a2-4456-a185-351885858b1e/issues/814afb44-e484-4771-806c-aed6add2f81c/` |
| STORY-002 | completed | 5 | IMI-3 | `85a54434-cc3c-431b-bfa0-a90a30ab5b84` | `/api/v1/workspaces/33god/projects/495de1b1-a4a2-4456-a185-351885858b1e/issues/85a54434-cc3c-431b-bfa0-a90a30ab5b84/` |
| STORY-003 | not_started | 3 | IMI-4 | `39120246-1762-4030-9bd1-149c425d2bee` | `/api/v1/workspaces/33god/projects/495de1b1-a4a2-4456-a185-351885858b1e/issues/39120246-1762-4030-9bd1-149c425d2bee/` |
| STORY-004 | not_started | 2 | IMI-5 | `8d5350cc-a2df-4c62-82f7-2afba7788669` | `/api/v1/workspaces/33god/projects/495de1b1-a4a2-4456-a185-351885858b1e/issues/8d5350cc-a2df-4c62-82f7-2afba7788669/` |
| STORY-005 | not_started | 5 | IMI-6 | `de780d2a-85a1-4564-895d-54df4d46fa64` | `/api/v1/workspaces/33god/projects/495de1b1-a4a2-4456-a185-351885858b1e/issues/de780d2a-85a1-4564-895d-54df4d46fa64/` |

## Labels Added to `IMI`
- `must-have`
- `should-have`
- `could-have`
- `sprint-1`
- `effort:S`
- `effort:M`
- `effort:L`
- `effort:XL`
- `feature`
- `agent-workflow`
- `legacy-migration`
- `yi-integration`

## Verification Snapshot
- Active migrated issue count in `IMI`: `5`
- Done: `2` (`IMI-2`, `IMI-3`)
- Todo: `3` (`IMI-4`, `IMI-5`, `IMI-6`)

