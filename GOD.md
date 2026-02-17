# iMi - GOD Document

> **Status**: ⚠️ **RETIRED** (2026-02-17)
>
> iMi has been retired from the 33GOD ecosystem. Agent workspace isolation is handled natively by OpenClaw's per-agent `workspace` config. The Rust worktree CLI functionality was not adopted by agents in practice.
>
> If a need arises for dedicated worktree management tooling in the future, iMi can be resurrected from this repo.
>
> **Decision by**: Jarad (CEO), recommended by Cack (CTO)
> **Reason**: Scope had drifted far from reality. Bloodbank integration, token identity, and jelmore coupling were aspirational, not implemented. OpenClaw handles the actual need.

---

## What iMi Was

A Rust CLI for git worktree management (`imi add/list/go/remove/status`). Tracked worktrees in local SQLite. Had ambitions to be an agent workspace isolation platform with Bloodbank event emission and jelmore session integration — none of which materialized.

## What Replaced It

- **Agent workspace isolation**: OpenClaw `agents.list[].workspace` config
- **Worktree management for agents**: `33god-creating-and-working-with-projects` skill (uses native git commands)
- **Session management**: Was supposed to be jelmore's job, but sessions are managed by OpenClaw natively
