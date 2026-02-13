---
date: 2026-02-11
author: BMAD Process (subagent)
status: approved
version: 3.0.0
stepsCompleted: [analysis, product-brief]
inputDocuments:
  - iMi/GOD.md
  - docs/GOD.md
  - iMi/docs/api-contracts.md
  - iMi/docs/architecture-imi-project-registry.md
  - components.toml
---

# Product Brief: iMi v3 — Project Intelligence Layer

## 1. Vision

iMi v3 is the **Project Intelligence Layer** for the 33GOD agentic software pipeline. It is the single source of truth for what projects exist, what work is in flight, and how Plane tickets map to git reality (branches, commits, PRs, deploys).

iMi v3 replaces the deprecated Rust CLI (iMi v1/v2) with a **FastAPI + FastStream service** that participates natively in the Bloodbank event bus.

**One-liner:** iMi answers "What exists? What's happening? Where did it come from?"

## 2. Problem Statement

The 33GOD ecosystem has no centralized, queryable registry that connects:

1. **Project identity** — Which repos exist, their UUIDs, metadata
2. **Work-in-flight** — What branches/worktrees are active right now, who owns them
3. **Ticket-to-code traceability** — The chain from a Plane ticket → branch → commits → PR → deploy
4. **Component manifest** — For meta-repos like 33GOD, which sub-components exist and their relationships

The old iMi was a Rust CLI that managed git worktrees locally. It conflated **workspace mechanics** (creating worktrees, claiming them) with **intelligence** (knowing what exists, tracking linkage). v3 separates cleanly: git does git, Plane does planning, iMi does **reality observation and recording**.

## 3. Layer Model (No Overlap)

| Layer | Owner | Responsibility |
|-------|-------|----------------|
| **Intent** | Plane | Backlog, priorities, acceptance criteria, sprint planning |
| **Reality** | **iMi v3** | What repos exist, what branches are active, who's working on what, ticket↔branch↔PR linkage |
| **Events** | Bloodbank | Event bus connecting intent and reality; status sync via pub/sub |

## 4. Target Users

| User | Use Case |
|------|----------|
| **AI Agents** (Yi, Flume, OpenClaw) | Query project registry, report work status, resolve paths |
| **Orchestrators** (Flume CEO) | Discover in-flight work to avoid conflicts, track agent assignments |
| **Developers** (Jarad) | Query what's happening across all projects, trace ticket→PR chains |
| **CI/CD** (hookd, GitHub Actions) | Report commits, PR status, deployment completions |
| **Dashboards** (Candybar, Holocene) | Visualize project activity, work-in-flight heatmaps |

## 5. Core Capabilities

### 5.1 Project Registry
- UUID-based identity for every project
- Canonical mapping: `project_id ↔ remote_origin`
- Metadata store (language, framework, team, custom JSONB)
- Idempotent registration (ON CONFLICT returns existing)
- Soft-delete with audit trail

### 5.2 Work-in-Flight Tracking
- Register active branches/worktrees per project
- Record source context: which Plane ticket, Bloodbank command, or agent assignment spawned the work
- Track ownership: which agent or human owns each branch
- Record git state: uncommitted changes, ahead/behind, last commit
- **Observability only** — no locks, no enforcement, no mutex

### 5.3 Metadata Linkage
- Link chain: `plane_ticket_id → branch → commits → PR → deploy`
- Bi-directional lookup: "What branch is ticket PROJ-123 on?" / "What ticket is branch feat/auth for?"
- PR lifecycle tracking: opened → reviewed → merged → deployed
- Commit-level linkage for audit trails

### 5.4 Bloodbank Integration
- **Emits**: `imi.project.registered`, `imi.work.started`, `imi.work.completed`, `imi.pr.created`, `imi.pr.merged`, `imi.deploy.completed`
- **Consumes**: Plane webhook events (via hookd/webhook relay), `git.push` events, `hookd.tool.mutation.*` events, CI/CD completion events
- Standard EventEnvelope format, FastStream consumer pattern

### 5.5 Component Manifest
- Owns the `components.toml` equivalent as a queryable data model
- Tracks component → repo mapping, status, relationships, dependencies
- Serves component discovery for meta-repos (33GOD is a repo-of-repos)

## 6. What iMi v3 is NOT

| ❌ Not This | ✅ That's This |
|------------|---------------|
| Git worktree creation tool | Git CLI does that |
| Agent lock/mutex system | Orchestrators (Flume) handle isolation |
| Task manager / sprint board | Plane does that |
| Deployment tool | CI/CD pipelines do that |
| Code review tool | GitHub/agents do that |
| Build system | mise/cargo/uv do that |

## 7. Tech Stack

| Component | Technology | Rationale |
|-----------|-----------|-----------|
| REST API | FastAPI | Standard 33GOD HTTP service pattern |
| Event bus | FastStream + RabbitMQ | Standard 33GOD consumer pattern (ADR-0002) |
| Database | PostgreSQL + asyncpg + SQLAlchemy | Concurrent multi-agent access, ACID, JSONB |
| Migrations | Alembic | Standard Python DB migration tool |
| Config | Pydantic BaseSettings | Standard 33GOD config pattern |
| CLI (optional) | Typer | Thin CLI that calls the API |
| Testing | pytest + pytest-asyncio | Standard 33GOD test stack |
| Package mgmt | uv | Standard 33GOD Python toolchain |

## 8. Success Criteria

1. Any 33GOD component can resolve a project UUID from a git remote URL in <10ms
2. Agents can query "what work is in-flight for project X?" via REST or event
3. Full traceability chain from Plane ticket → deployed code exists in iMi
4. Component manifest is queryable via API (replaces static `components.toml`)
5. All state changes emit Bloodbank events for downstream consumers
6. Zero downtime registration: idempotent, concurrent-safe, no locks

## 9. Key Risks

| Risk | Mitigation |
|------|-----------|
| Stale git state (branch deleted but iMi still shows active) | Periodic reconciliation job + event-driven updates |
| Plane webhook reliability | Idempotent event handlers + manual sync endpoint |
| Schema evolution | Alembic migrations + JSONB for extensible metadata |
| Adoption by existing agents | Gradual migration: register projects during init, agents opt-in to reporting |

## 10. Prior Art / Migration Notes

- iMi v1/v2 Rust CLI is deprecated. Its SQLite data (projects, worktrees, claims) can be migrated via the existing `migration-sqlite-to-postgres.md` strategy.
- The v2 architecture doc (`architecture-imi-project-registry.md`) and API contracts (`api-contracts.md`) contain useful schema and endpoint designs that inform v3, but v3 is a clean rewrite with the FastAPI+FastStream stack.
- The `components.toml` at repo root is the seed data for the Component Manifest feature.
