# iMi v3 — BMAD Specification

**Generated**: 2026-02-11
**Status**: Implementation-ready
**Method**: BMAD v6 (Analysis → Planning → Solutioning)

## Artifacts

| # | Document | Purpose |
|---|----------|---------|
| 01 | [Product Brief](01-product-brief.md) | Vision, problem, layer model, capabilities, non-goals |
| 02 | [PRD](02-prd.md) | Functional requirements, non-functional requirements, data model, API surface, acceptance criteria |
| 03 | [Architecture](03-architecture.md) | C4 diagrams, project structure, database schema, technology decisions, integration architecture, deployment |
| 04 | [Epics & Stories](04-epics-and-stories.md) | 7 epics, 27 stories with acceptance criteria in Given/When/Then format |

## Quick Summary

**iMi v3** is a FastAPI + FastStream microservice — the **Project Intelligence Layer** for the 33GOD ecosystem.

- **Project Registry**: UUID-based identity for every repo
- **Work-in-Flight Tracking**: What branches are active, who owns them, git state
- **Metadata Linkage**: Plane ticket → branch → PR → deploy traceability
- **Bloodbank Integration**: Publishes/consumes events on the 33GOD event bus
- **Component Manifest**: Queryable replacement for `components.toml`

## Implementation Order

```
Epic 1: Service Foundation → Epic 2: Project Registry → Epic 3: Work Tracking
→ Epic 4: Linkage & Tracing → Epic 5: Bloodbank Events → Epic 6: Components → Epic 7: CLI & Polish
```

7 epics, 27 stories, ~30-50 agent-hours total.

## For Implementing Agents

Start with `04-epics-and-stories.md`. Each story has:
- Clear acceptance criteria in Given/When/Then
- Dependencies on previous stories (sequential within epic)
- Reference to architecture decisions in `03-architecture.md`

Tech stack: FastAPI, FastStream, PostgreSQL (asyncpg + SQLAlchemy), Alembic, Pydantic v2, aio-pika, Typer, uv.
