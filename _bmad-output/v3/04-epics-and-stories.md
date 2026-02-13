---
date: 2026-02-11
author: BMAD Process (subagent)
status: approved
version: 3.0.0
stepsCompleted: [analysis, product-brief, prd, architecture, epics-and-stories]
inputDocuments:
  - _bmad-output/v3/01-product-brief.md
  - _bmad-output/v3/02-prd.md
  - _bmad-output/v3/03-architecture.md
---

# iMi v3 — Epics and Stories

## Overview

This document breaks the iMi v3 PRD and Architecture into implementable epics and stories. Stories are ordered for sequential implementation — each epic builds on the previous one.

**Audience**: AI coding agents building iMi v3.

## Requirements Inventory

### Functional Requirements

- FR-1: Project Registry (CRUD, idempotent, UUID-based)
- FR-2: Work-in-Flight Tracking (branches, git state, ownership)
- FR-3: Metadata Linkage (ticket↔branch↔PR↔deploy chain)
- FR-4: Bloodbank Integration (publish events, consume events)
- FR-5: Component Manifest (components.toml → queryable data)
- FR-6: Thin CLI (optional, wraps API)

### Non-Functional Requirements

- NFR-1: Performance (<10ms project lookup, <25ms work queries)
- NFR-2: Reliability (idempotent, soft-deletes, health checks)
- NFR-3: Data Integrity (FK constraints, unique constraints, ACID)
- NFR-4: Observability (structured logging, correlation IDs)
- NFR-5: Security (API key auth)
- NFR-6: Deployability (Docker, env-based config, Alembic)
- NFR-7: Extensibility (JSONB metadata, API versioning)

### FR Coverage Map

| Requirement | Epic | Stories |
|-------------|------|---------|
| FR-1 | Epic 1 (Foundation), Epic 2 (Projects) | 1.1-1.5, 2.1-2.5 |
| FR-2 | Epic 3 (Work Tracking) | 3.1-3.5 |
| FR-3 | Epic 4 (Linkage) | 4.1-4.5 |
| FR-4 | Epic 5 (Events) | 5.1-5.5 |
| FR-5 | Epic 6 (Components) | 6.1-6.3 |
| FR-6 | Epic 7 (CLI) | 7.1-7.2 |
| NFR-1..7 | Woven into Epics 1-5 | Cross-cutting |

## Epic List

1. **Epic 1: Service Foundation** — Project scaffold, database, config, health check
2. **Epic 2: Project Registry** — CRUD API for projects
3. **Epic 3: Work-in-Flight Tracking** — Work items, git state, status queries
4. **Epic 4: Metadata Linkage & Tracing** — Ticket/PR/deploy links, trace chain
5. **Epic 5: Bloodbank Integration** — Event publishing + consuming
6. **Epic 6: Component Manifest** — Component registry, seed from components.toml
7. **Epic 7: CLI & Polish** — Thin CLI, documentation, migration script

---

## Epic 1: Service Foundation

**Goal**: Stand up the iMi v3 project with FastAPI, PostgreSQL, configuration, database models, and a passing health check. No business logic yet — just the skeleton that everything plugs into.

### Story 1.1: Project Scaffold

As an **implementing agent**,
I want a properly structured Python project with pyproject.toml, src layout, and dev tooling,
So that all subsequent stories have a consistent foundation to build on.

**Acceptance Criteria:**

**Given** an empty `~/code/33GOD/iMi/` directory (ignoring legacy files)
**When** the scaffold is created
**Then** the following structure exists:
- `pyproject.toml` with dependencies: `fastapi`, `uvicorn`, `sqlalchemy[asyncio]`, `asyncpg`, `alembic`, `pydantic>=2`, `pydantic-settings`, `faststream[rabbit]`, `aio-pika`, `typer`, `httpx`, `orjson`
- `src/imi/__init__.py` with `__version__ = "3.0.0"`
- `src/imi/main.py` with minimal FastAPI app
- `src/imi/config.py` with `Settings` class (all fields from Architecture §10)
- `tests/conftest.py` with placeholder
- `mise.toml` with tasks: `dev`, `test`, `lint`, `migrate`
- `Dockerfile` and `docker-compose.yml` per Architecture §7
- `.env.example` with all config variables

**And** `uv sync` succeeds
**And** `uv run uvicorn imi.main:app` starts without error

### Story 1.2: Database Setup

As an **implementing agent**,
I want SQLAlchemy async engine configuration and Alembic migration setup,
So that database connectivity and schema management are ready.

**Acceptance Criteria:**

**Given** the project scaffold from Story 1.1
**When** database setup is complete
**Then**:
- `src/imi/database.py` exists with `create_async_engine`, `async_sessionmaker`, and `get_db` dependency
- `migrations/` directory exists with Alembic config
- `migrations/env.py` is configured for async SQLAlchemy
- Running `uv run alembic upgrade head` against a test PostgreSQL creates an empty database
- A `get_db` FastAPI dependency yields an async session and properly closes it

### Story 1.3: Core Database Models

As an **implementing agent**,
I want SQLAlchemy ORM models for all entities defined in the Architecture,
So that Alembic can generate the initial migration.

**Acceptance Criteria:**

**Given** database setup from Story 1.2
**When** models are defined
**Then** the following models exist in `src/imi/models/`:
- `Project` — all fields from Architecture §4.1, including JSONB metadata, soft-delete
- `WorkItem` — all fields including git state, source, owner, completion tracking
- `TicketLink`, `PRLink`, `DeployLink` — linkage tables with FKs to WorkItem
- `Component`, `ComponentRelationship` — component manifest tables
**And** all constraints from Architecture §4.3 are applied (unique, check, exclude)
**And** `uv run alembic revision --autogenerate -m "initial schema"` produces a valid migration
**And** `uv run alembic upgrade head` creates all tables with correct constraints

### Story 1.4: Base Repository Pattern

As an **implementing agent**,
I want a base repository class with common CRUD operations,
So that all entity repositories share consistent patterns.

**Acceptance Criteria:**

**Given** models from Story 1.3
**When** the base repository is implemented
**Then** `src/imi/repositories/base.py` provides:
- `get_by_id(id: UUID) -> Model | None`
- `list(offset, limit, filters) -> list[Model]`
- `create(data: dict) -> Model`
- `update(id: UUID, data: dict) -> Model`
- `soft_delete(id: UUID) -> None` (sets `active=false`)
**And** repositories are generic over the model type
**And** all operations use async sessions from `get_db`

### Story 1.5: Health Check & Startup

As an **implementing agent**,
I want a `/health` endpoint and proper FastAPI lifespan management,
So that the service can be health-checked by Docker and monitoring.

**Acceptance Criteria:**

**Given** the scaffold, DB, and models from previous stories
**When** the service starts
**Then**:
- `GET /health` returns `{"status": "healthy", "database": "connected", "version": "3.0.0"}`
- If PostgreSQL is unreachable, `/health` returns `{"status": "unhealthy", "database": "disconnected"}` with 503
- FastAPI lifespan properly initializes and closes database engine
- Structured JSON logging is configured (using `logging` with JSON formatter)
- CORS middleware is configured from `settings.cors_origins`
- The root path redirects to `/docs` (Swagger UI)

---

## Epic 2: Project Registry

**Goal**: Full CRUD API for projects — the core identity layer. Projects get UUIDs, are searchable, and support idempotent registration.

### Story 2.1: Project Repository

As an **implementing agent**,
I want a `ProjectRepository` with project-specific query methods,
So that the service layer can perform all project operations.

**Acceptance Criteria:**

**Given** base repository from Story 1.4
**When** `ProjectRepository` is implemented
**Then** `src/imi/repositories/project.py` provides:
- `get_by_remote_origin(remote_origin: str) -> Project | None` — exact match on active projects
- `search_by_name(query: str, limit: int) -> list[Project]` — ILIKE fuzzy search
- `register(data: ProjectCreate) -> tuple[Project, bool]` — returns (project, is_new). If remote_origin exists and is active, returns existing. If exists but inactive, reactivates.
- `get_with_counts(id: UUID) -> ProjectWithCounts | None` — includes active work item count
**And** all methods use async sessions
**And** unit tests exist with a test database

### Story 2.2: Project Pydantic Schemas

As an **implementing agent**,
I want Pydantic v2 schemas for project request/response models,
So that API validation and serialization are type-safe.

**Acceptance Criteria:**

**Given** the Project model
**When** schemas are defined in `src/imi/schemas/project.py`
**Then** these schemas exist:
- `ProjectCreate` — `name`, `remote_origin` (validated git URL pattern), `default_branch` (default "main"), `trunk_path`, `metadata` (optional dict)
- `ProjectUpdate` — all optional: `name`, `description`, `default_branch`, `trunk_path`, `metadata`
- `ProjectResponse` — full project fields + `work_item_count` (optional)
- `ProjectListResponse` — paginated list with `items`, `total`, `offset`, `limit`
**And** `remote_origin` validation accepts `git@github.com:user/repo.git` and `https://github.com/user/repo.git` formats

### Story 2.3: Project Service Layer

As an **implementing agent**,
I want a `ProjectService` class containing business logic for projects,
So that routers and consumers both call the same logic.

**Acceptance Criteria:**

**Given** repository and schemas from previous stories
**When** `src/imi/services/project.py` is implemented
**Then**:
- `register_project(data: ProjectCreate) -> tuple[ProjectResponse, bool]` — calls repo, returns (project, is_new_flag)
- `get_project(id: UUID) -> ProjectResponse` — raises 404 if not found
- `get_by_remote_origin(url: str) -> ProjectResponse` — raises 404 if not found
- `search_projects(query: str, limit: int) -> list[ProjectResponse]`
- `list_projects(offset, limit, active_only) -> ProjectListResponse`
- `update_project(id: UUID, data: ProjectUpdate) -> ProjectResponse`
- `deactivate_project(id: UUID) -> None`
**And** service accepts a `db: AsyncSession` dependency
**And** all methods are async

### Story 2.4: Project API Router

As an **implementing agent**,
I want FastAPI router endpoints for all project operations,
So that clients can interact with the project registry via HTTP.

**Acceptance Criteria:**

**Given** project service from Story 2.3
**When** `src/imi/api/projects.py` is implemented
**Then** these endpoints work:
- `POST /api/v1/projects` → register (201 if new, 200 if existing)
- `GET /api/v1/projects` → list with `?offset=`, `?limit=`, `?active=`, `?search=` query params
- `GET /api/v1/projects/{project_id}` → get by UUID (404 if not found)
- `GET /api/v1/projects/by-origin?remote_origin=` → get by git URL
- `PATCH /api/v1/projects/{project_id}` → update
- `DELETE /api/v1/projects/{project_id}` → soft-delete (204)
**And** router is mounted on the main app
**And** integration tests exist using `httpx.AsyncClient` with `TestClient`
**And** error responses follow the standard `{"detail": {"code": "...", "message": "..."}}` format

### Story 2.5: Project API Tests

As an **implementing agent**,
I want comprehensive integration tests for the project API,
So that correctness and edge cases are verified.

**Acceptance Criteria:**

**Given** the project API from Story 2.4
**When** tests in `tests/test_api/test_projects.py` are implemented
**Then** these scenarios are covered:
- Register a new project → 201, UUID returned
- Register same remote_origin again → 200, same UUID returned (idempotent)
- Register after soft-delete → reactivates, same UUID
- Get by UUID → 200
- Get non-existent UUID → 404
- Get by remote_origin → 200
- Search by name → returns matching projects
- List with pagination → correct offset/limit behavior
- Update metadata → 200, updated fields
- Delete → 204, project no longer in list (but still gettable with `?active=false`)
- Invalid remote_origin format → 400 validation error
**And** all tests pass against a real PostgreSQL instance (testcontainers or docker)

---

## Epic 3: Work-in-Flight Tracking

**Goal**: Register active branches, track git state, query who's working on what. The "reality layer" of what's happening in code right now.

### Story 3.1: Work Item Repository & Schemas

As an **implementing agent**,
I want repository methods and Pydantic schemas for work items,
So that work-in-flight tracking has a complete data layer.

**Acceptance Criteria:**

**Given** the base repository and models
**When** `WorkRepository` and work item schemas are implemented
**Then**:
- `WorkItemCreate` schema: `project_id`, `branch_name`, `work_type`, `source` (object with `type` + `ref`), `owner_id`, `worktree_path` (optional), `metadata`
- `WorkItemGitStateUpdate` schema: `has_uncommitted_changes`, `uncommitted_files_count`, `ahead_of_trunk`, `behind_trunk`, `last_commit_hash`, `last_commit_message` (all optional)
- `WorkItemComplete` schema: `completion_type`, `merge_commit_hash` (optional), `merged_by` (optional)
- `WorkItemResponse` schema: all fields + computed `status`
- `WorkRepository` provides: `get_by_project_and_branch`, `list_by_owner`, `list_inflight(project_id)`, `update_git_state`, `complete`
**And** the `status` field is computed from git state fields per Architecture TD-3

### Story 3.2: Work Service Layer

As an **implementing agent**,
I want a `WorkService` with business logic for work item lifecycle,
So that the work tracking API and consumers share the same logic.

**Acceptance Criteria:**

**Given** work repository from Story 3.1
**When** `src/imi/services/work.py` is implemented
**Then**:
- `start_work(data: WorkItemCreate) -> WorkItemResponse` — validates project exists, creates work item
- `get_work_item(id: UUID) -> WorkItemResponse`
- `list_work(project_id, owner_id, work_type, status, offset, limit) -> WorkItemListResponse`
- `update_git_state(id: UUID, data: WorkItemGitStateUpdate) -> WorkItemResponse`
- `complete_work(id: UUID, data: WorkItemComplete) -> WorkItemResponse` — sets completed_at, completion_type
- `get_inflight(project_id: UUID) -> list[WorkItemResponse]` — active items with uncommitted/ahead/diverged status
**And** all methods validate FK relationships (project must exist)
**And** duplicate (project_id, branch_name) for active items returns 409

### Story 3.3: Work API Router

As an **implementing agent**,
I want FastAPI router endpoints for work item operations,
So that agents and tools can report and query work-in-flight.

**Acceptance Criteria:**

**Given** work service from Story 3.2
**When** `src/imi/api/work.py` is implemented
**Then** these endpoints work:
- `POST /api/v1/work` → start work (201)
- `GET /api/v1/work` → list with `?project_id=`, `?owner_id=`, `?work_type=`, `?status=`, pagination
- `GET /api/v1/work/{work_item_id}` → get by UUID
- `PATCH /api/v1/work/{work_item_id}/git-state` → update git state (200)
- `POST /api/v1/work/{work_item_id}/complete` → mark completed (200)
- `GET /api/v1/work/inflight` → list all in-flight work, `?project_id=` optional filter
**And** integration tests cover: create, duplicate branch detection, git state update, completion, inflight query

### Story 3.4: Stats Endpoint

As an **implementing agent**,
I want a `/api/v1/stats` endpoint with registry-wide statistics,
So that dashboards can show ecosystem health at a glance.

**Acceptance Criteria:**

**Given** project and work repositories
**When** `src/imi/api/stats.py` is implemented
**Then** `GET /api/v1/stats` returns:
```json
{
    "total_projects": 15,
    "active_projects": 12,
    "total_work_items": 47,
    "active_work_items": 23,
    "inflight_work_items": 8,
    "work_items_by_status": {"clean": 5, "uncommitted": 3, "ahead": 7, "diverged": 1, "behind": 2},
    "work_items_by_type": {"feature": 12, "fix": 5, "experiment": 3, "review": 2, "devops": 1},
    "total_components": 15
}
```
**And** counts are computed via efficient aggregate queries (not N+1)

### Story 3.5: Work API Tests

As an **implementing agent**,
I want comprehensive tests for work item operations,
So that the reality layer is verified correct.

**Acceptance Criteria:**

**Given** work API from Story 3.3
**When** tests in `tests/test_api/test_work.py` are implemented
**Then** these scenarios are covered:
- Start work on valid project → 201
- Start work on non-existent project → 404
- Start duplicate active branch → 409
- Update git state → status transitions correctly (clean→uncommitted, clean→ahead, etc.)
- Complete work → status becomes completion_type, completed_at is set
- Query inflight → returns only active, non-clean items
- Query by owner → returns only that owner's work
- Stats endpoint → returns correct aggregate counts

---

## Epic 4: Metadata Linkage & Tracing

**Goal**: Link Plane tickets, PRs, and deploys to work items. Enable full traceability from ticket to deployed code.

### Story 4.1: Link Repository & Schemas

As an **implementing agent**,
I want repository methods and schemas for ticket/PR/deploy links,
So that the linkage data layer is complete.

**Acceptance Criteria:**

**Given** work items from Epic 3
**When** link repository and schemas are implemented
**Then**:
- `TicketLinkCreate` schema: `work_item_id`, `plane_ticket_id`, `ticket_title` (optional), `ticket_url` (optional)
- `PRLinkCreate` schema: `work_item_id`, `pr_number`, `pr_url`, `pr_state`, `pr_title` (optional)
- `PRLinkUpdate` schema: `pr_state` (for lifecycle tracking)
- `DeployLinkCreate` schema: `work_item_id`, `deploy_id`, `environment`, `commit_hash`
- `LinkRepository` provides: `create_ticket_link`, `create_pr_link`, `create_deploy_link`, `update_pr_state`, `get_links_for_work_item`, `get_work_items_for_ticket`, `get_work_item_for_pr`
**And** schemas validate that referenced work_item exists

### Story 4.2: Link Service & API

As an **implementing agent**,
I want API endpoints for creating and querying links,
So that agents and CI can report ticket/PR/deploy linkage.

**Acceptance Criteria:**

**Given** link repository from Story 4.1
**When** `src/imi/services/link.py` and `src/imi/api/links.py` are implemented
**Then**:
- `POST /api/v1/links/ticket` → create ticket link (201), validates work_item exists
- `POST /api/v1/links/pr` → create PR link (201)
- `PATCH /api/v1/links/pr/{pr_link_id}` → update PR state (200)
- `POST /api/v1/links/deploy` → create deploy link (201)
- `GET /api/v1/links?work_item_id=` → list all links for a work item
- `GET /api/v1/links/ticket/{plane_ticket_id}` → get all work items for a ticket
**And** duplicate ticket links (same work_item + ticket) are idempotent

### Story 4.3: Trace Service & API

As an **implementing agent**,
I want a trace endpoint that resolves the full linkage chain,
So that any entity can be traced to its origin and destination.

**Acceptance Criteria:**

**Given** links from Story 4.2
**When** `src/imi/services/trace.py` and `src/imi/api/trace.py` are implemented
**Then** `GET /api/v1/trace/{entity_type}/{entity_id}` works for:
- `entity_type=ticket`, `entity_id=PROJ-123` → returns: ticket info + linked work items + their PRs + their deploys
- `entity_type=work_item`, `entity_id=<uuid>` → returns: work item + its ticket(s) + its PR(s) + its deploy(s)
- `entity_type=pr`, `entity_id=42` → returns: PR info + linked work item + its ticket(s) + its deploy(s)
- `entity_type=deploy`, `entity_id=<deploy_id>` → returns: deploy info + linked work item + its ticket(s) + its PR(s)

**Response structure**:
```json
{
    "entity": {"type": "ticket", "id": "PROJ-123"},
    "chain": {
        "tickets": [{"plane_ticket_id": "PROJ-123", "ticket_title": "..."}],
        "work_items": [{"id": "...", "branch_name": "feat/auth", "status": "ahead"}],
        "prs": [{"pr_number": 42, "pr_state": "open", "pr_url": "..."}],
        "deploys": []
    }
}
```

**And** unknown entity types return 400
**And** entity not found returns empty chain (not 404 — the entity might exist but have no links yet)

### Story 4.4: Linkage Tests

As an **implementing agent**,
I want comprehensive tests for linkage and tracing,
So that the traceability chain is verified correct.

**Acceptance Criteria:**

**Given** link and trace APIs from previous stories
**When** tests are implemented
**Then** these scenarios are covered:
- Create ticket link → 201
- Create duplicate ticket link → idempotent (200 or 201, same link)
- Create PR link → 201
- Update PR state from open→merged → 200
- Create deploy link → 201
- Trace from ticket → returns full chain (ticket → work → PR → deploy)
- Trace from deploy → returns reverse chain
- Trace for entity with no links → returns empty chain
- FK violations → appropriate error

---

## Epic 5: Bloodbank Integration

**Goal**: Publish iMi events to Bloodbank on every mutation. Consume external events (Plane, Git, hookd) to auto-update iMi state.

### Story 5.1: Event Publisher

As an **implementing agent**,
I want an `EventPublisher` class that wraps Bloodbank event publishing,
So that all iMi mutations can emit events consistently.

**Acceptance Criteria:**

**Given** the Bloodbank `event_producers` library
**When** `src/imi/events/publisher.py` is implemented
**Then**:
- `EventPublisher` class with `publish(event_type: str, payload: dict, correlation_id: str | None)` method
- Uses `create_envelope()` from `event_producers.events.envelope`
- Uses `create_source(host=hostname, trigger_type="system", app="imi")`
- Publishes to exchange `bloodbank.events.v1` with routing key = event_type
- Fire-and-forget: publish errors are logged but never raise to caller
- Lifespan integration: publisher starts on app startup, closes on shutdown
**And** publisher is injectable as a FastAPI dependency

### Story 5.2: Wire Event Publishing into Services

As an **implementing agent**,
I want every service mutation to emit the corresponding Bloodbank event,
So that the entire 33GOD ecosystem can react to iMi state changes.

**Acceptance Criteria:**

**Given** EventPublisher from Story 5.1
**When** services are updated
**Then** these events are emitted (with correct payloads):
- `ProjectService.register_project` → `imi.project.registered` (only if is_new=true)
- `ProjectService.update_project` → `imi.project.updated`
- `ProjectService.deactivate_project` → `imi.project.deactivated`
- `WorkService.start_work` → `imi.work.started`
- `WorkService.update_git_state` → `imi.work.git_state_updated`
- `WorkService.complete_work` → `imi.work.completed`
- `LinkService.create_ticket_link` → `imi.link.ticket_linked`
- `LinkService.create_pr_link` → `imi.pr.created`
- `LinkService.update_pr_state(merged)` → `imi.pr.merged`
- `LinkService.update_pr_state(closed)` → `imi.pr.closed`
- `LinkService.create_deploy_link` → `imi.deploy.completed`
**And** event payloads include the entity data (project/work_item/link details)
**And** tests verify events are published (mock EventPublisher, assert calls)

### Story 5.3: FastStream Consumer Setup

As an **implementing agent**,
I want FastStream broker and consumer infrastructure wired into the FastAPI app,
So that iMi can consume Bloodbank events alongside serving HTTP.

**Acceptance Criteria:**

**Given** the FastStream patterns from `33god-service-development` skill
**When** FastStream is integrated
**Then**:
- `RabbitBroker` is created in `main.py` with `settings.rabbitmq_url`
- Broker starts in FastAPI lifespan (`await broker.start()` on startup, `await broker.close()` on shutdown)
- Consumer modules in `src/imi/consumers/` are imported and registered
- Queues are created as durable with correct routing keys per Architecture §6.1
- A basic `imi.commands` consumer exists that logs received messages

### Story 5.4: Command Event Consumers

As an **implementing agent**,
I want FastStream consumers for `imi.cmd.*` command events,
So that other services can trigger iMi operations via Bloodbank events.

**Acceptance Criteria:**

**Given** FastStream setup from Story 5.3
**When** `src/imi/consumers/commands.py` is implemented
**Then** these handlers exist:
- `imi.cmd.register_project` → calls `ProjectService.register_project`, publishes result event
- `imi.cmd.start_work` → calls `WorkService.start_work`
- `imi.cmd.complete_work` → calls `WorkService.complete_work`
- `imi.cmd.link_ticket` → calls `LinkService.create_ticket_link`
**And** each handler unwraps EventEnvelope per FastStream pattern
**And** errors publish `imi.cmd.failed` event with error details
**And** handlers are idempotent (safe to process the same event twice)

### Story 5.5: External Event Consumers

As an **implementing agent**,
I want FastStream consumers for Plane, Git, and hookd events,
So that iMi auto-updates when external systems report state changes.

**Acceptance Criteria:**

**Given** consumers infrastructure from Story 5.3
**When** `src/imi/consumers/plane.py`, `git.py`, and `mutations.py` are implemented
**Then**:

**Plane consumer** (`imi.plane_events` queue, `plane.ticket.*`):
- `plane.ticket.created` → create a placeholder ticket link if a work item with matching source_ref exists
- `plane.ticket.updated` → update ticket title/url on existing links

**Git consumer** (`imi.git_events` queue, `git.push`, `git.pr.*`):
- `git.push` → find work item by branch_name, update last_commit_hash and last_commit_message
- `git.pr.opened` → create PR link for matching work item
- `git.pr.merged` → update PR link state, trigger work item completion
- `git.pr.closed` → update PR link state

**Mutation consumer** (`imi.mutation_events` queue, `tool.mutation.*`):
- Any mutation event → find work item by repo+branch, update `last_sync_at` timestamp

**And** all consumers gracefully handle "no matching work item" (log and skip, not error)
**And** consumers have unit tests with mocked services

---

## Epic 6: Component Manifest

**Goal**: Replace the static `components.toml` with a queryable component registry. Seed from existing data.

### Story 6.1: Component Repository, Service & API

As an **implementing agent**,
I want CRUD operations for components,
So that meta-repo component manifests are queryable via API.

**Acceptance Criteria:**

**Given** the Component model from Epic 1
**When** component repository, service, and API are implemented
**Then**:
- `POST /api/v1/components` → register component with `parent_project_id`, `name`, `path`, `kind`, `enabled`, `linked_project_id` (optional), `metadata`
- `GET /api/v1/components` → list with `?parent_project_id=`, `?kind=`, `?enabled=` filters
- `GET /api/v1/components/{id}` → get by UUID
- `PATCH /api/v1/components/{id}` → update metadata, enabled, linked_project_id
- Component registration emits `imi.component.registered` event
**And** `linked_project_id` FK is validated against projects table (if provided)

### Story 6.2: Component Relationships

As an **implementing agent**,
I want to define relationships between components,
So that dependency graphs can be queried.

**Acceptance Criteria:**

**Given** component CRUD from Story 6.1
**When** relationship endpoints are added
**Then**:
- `POST /api/v1/components/{id}/relationships` → create relationship `{target_component_id, relationship_type}` where type is `depends_on`, `produces_events_for`, or `consumes_events_from`
- `GET /api/v1/components/{id}/relationships` → list relationships (both directions)
- `GET /api/v1/components/{id}/graph` → return full dependency graph (BFS from component)

### Story 6.3: Seed from components.toml

As an **implementing agent**,
I want a seed script that imports `components.toml` into the component manifest,
So that the existing 33GOD component data is available in iMi on day one.

**Acceptance Criteria:**

**Given** the `~/code/33GOD/components.toml` file
**When** `scripts/seed_components.py` is run
**Then**:
- Each entry in `[components]` is registered as a Component
- `parent_project_id` is set to the 33GOD meta-repo project (registered first if not exists)
- `name`, `path`, `kind`, `enabled` are mapped from TOML fields
- Script is idempotent (safe to re-run)
- Script outputs count of created/updated/skipped components
**And** after seeding, `GET /api/v1/components?parent_project_id=<33god_uuid>` returns all 15 components

---

## Epic 7: CLI & Polish

**Goal**: Optional thin CLI, migration script from v2, updated GOD doc, final documentation.

### Story 7.1: Thin CLI

As an **implementing agent**,
I want a Typer CLI that wraps common API operations,
So that developers and agents can interact with iMi from the terminal.

**Acceptance Criteria:**

**Given** the full API from Epics 2-6
**When** `src/imi/cli/main.py` is implemented with Typer
**Then** these commands work:
- `imi projects list` → table of projects (name, UUID, remote_origin, status)
- `imi projects register <remote_origin> [--name NAME]` → register and print UUID
- `imi projects get <project_id>` → detailed project info
- `imi work list [--project ID] [--owner OWNER]` → table of work items
- `imi work start <project_id> <branch> [--type TYPE] [--source-type TYPE --source-ref REF]`
- `imi work inflight [--project ID]` → in-flight work summary
- `imi trace ticket <ticket_id>` → print trace chain
- `imi components list [--project ID]` → table of components
- `imi stats` → print registry stats
**And** CLI reads `IMI_API_URL` env var (default `http://localhost:8400`)
**And** CLI uses `httpx` for all API calls
**And** `pyproject.toml` registers `imi` as a console script entry point

### Story 7.2: Documentation & Migration

As an **implementing agent**,
I want updated documentation and a v2 migration script,
So that the transition from v2 to v3 is smooth.

**Acceptance Criteria:**

**Given** the complete iMi v3 service
**When** documentation tasks are completed
**Then**:
- `GOD.md` is rewritten to reflect v3 architecture (FastAPI+FastStream, not Rust CLI)
- `README.md` has quickstart: setup, run, register a project, query work
- `scripts/migrate_v2.py` reads old SQLite DB and imports projects into PostgreSQL via API
- Migration script handles: projects (repositories table), worktrees→work_items, skips claims/activities
- `services/registry.yaml` has the iMi v3 entry per Architecture §6.3
- OpenAPI spec is auto-generated and accessible at `/docs`

---

## Implementation Order

```
Epic 1: Service Foundation (Stories 1.1→1.5)
    ↓
Epic 2: Project Registry (Stories 2.1→2.5)
    ↓
Epic 3: Work-in-Flight Tracking (Stories 3.1→3.5)
    ↓
Epic 4: Metadata Linkage & Tracing (Stories 4.1→4.4)
    ↓
Epic 5: Bloodbank Integration (Stories 5.1→5.5)
    ↓
Epic 6: Component Manifest (Stories 6.1→6.3)
    ↓
Epic 7: CLI & Polish (Stories 7.1→7.2)
```

**Total**: 7 Epics, 27 Stories

**Estimated effort per story**: Each story is scoped for a single AI agent session (1-3 hours of focused work). The full implementation is approximately 30-50 agent-hours.
