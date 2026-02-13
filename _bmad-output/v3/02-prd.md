---
date: 2026-02-11
author: BMAD Process (subagent)
status: approved
version: 3.0.0
stepsCompleted: [analysis, product-brief, prd]
inputDocuments:
  - _bmad-output/v3/01-product-brief.md
  - iMi/docs/api-contracts.md
  - iMi/docs/architecture-imi-project-registry.md
  - 33god-service-development/SKILL.md
---

# PRD: iMi v3 — Project Intelligence Layer

## 1. Executive Summary

iMi v3 is a **FastAPI + FastStream microservice** that serves as the Project Intelligence Layer for the 33GOD ecosystem. It provides a UUID-based project registry, work-in-flight tracking, Plane↔Git metadata linkage, Bloodbank event integration, and a component manifest for meta-repos.

This PRD defines the functional and non-functional requirements for a clean-room implementation.

## 2. Functional Requirements

### FR-1: Project Registry

#### FR-1.1: Register Project
- Accept: `name`, `remote_origin` (git URL), `default_branch`, `trunk_path`, `metadata` (JSONB)
- Assign a UUID v4 as `project_id`
- Enforce unique constraint on `remote_origin` (one project per repo)
- Idempotent: if `remote_origin` already exists and is active, return existing record
- Re-activate soft-deleted projects on re-registration
- Emit `imi.project.registered` event to Bloodbank

#### FR-1.2: Query Projects
- Get by UUID
- Get by `remote_origin` (exact match)
- Search by name (fuzzy/ILIKE)
- List all with pagination, filter by `active`, metadata fields
- Include aggregate counts: active worktrees, in-flight branches

#### FR-1.3: Update Project
- Update `name`, `description`, `default_branch`, `trunk_path`, `metadata`
- Emit `imi.project.updated` event

#### FR-1.4: Deactivate Project
- Soft-delete (set `active=false`)
- Cascade: deactivate all associated work items
- Emit `imi.project.deactivated` event

### FR-2: Work-in-Flight Tracking

#### FR-2.1: Register Work Item
- Accept: `project_id`, `branch_name`, `work_type` (feature/fix/experiment/review/devops/aiops), `source` (object: `{type: "plane_ticket"|"bloodbank_command"|"agent_assignment"|"manual", ref: "PROJ-123"}`)
- Accept: `owner_id` (agent or human identifier), `worktree_path` (optional)
- Assign UUID as `work_item_id`
- Emit `imi.work.started` event

#### FR-2.2: Update Git State
- Accept: `work_item_id` or `(project_id, branch_name)` composite key
- Update: `has_uncommitted_changes`, `uncommitted_files_count`, `ahead_of_trunk`, `behind_trunk`, `last_commit_hash`, `last_commit_message`, `last_sync_at`
- Compute derived `status`: `clean`, `uncommitted`, `ahead`, `behind`, `diverged`
- Emit `imi.work.git_state_updated` event

#### FR-2.3: Complete Work
- Mark work item as completed (merged, abandoned, or superseded)
- Accept: `completion_type` (merged/abandoned/superseded), `merge_commit_hash`, `merged_by`
- Set `completed_at` timestamp
- Emit `imi.work.completed` event

#### FR-2.4: Query In-Flight Work
- List all active work items, optionally filtered by `project_id`, `owner_id`, `work_type`, `status`
- Include git state summary
- Support "what is agent X working on?" queries

### FR-3: Metadata Linkage

#### FR-3.1: Link Plane Ticket to Work
- Accept: `plane_ticket_id`, `work_item_id`
- Store bidirectional link
- Multiple work items can link to one ticket (e.g., implementation branch + fix branch)
- Emit `imi.link.ticket_linked` event

#### FR-3.2: Link PR to Work
- Accept: `pr_number`, `pr_url`, `pr_state` (open/merged/closed), `work_item_id`
- Track PR lifecycle events
- Emit `imi.pr.created`, `imi.pr.merged`, `imi.pr.closed` events

#### FR-3.3: Link Deployment to Work
- Accept: `deploy_id`, `environment`, `commit_hash`, `work_item_id`
- Emit `imi.deploy.completed` event

#### FR-3.4: Trace Chain Query
- Given any node (ticket, branch, PR, deploy), return the full linkage chain
- `GET /trace/{entity_type}/{entity_id}` → returns all linked entities
- Supports: `ticket` → branches → PRs → deploys (forward trace)
- Supports: `deploy` → PR → branch → ticket (reverse trace)

### FR-4: Bloodbank Integration

#### FR-4.1: Event Publishing
All state mutations emit events to Bloodbank using the standard EventEnvelope format:

| Event | Routing Key | Trigger |
|-------|------------|---------|
| `imi.project.registered` | `imi.project.registered` | Project created |
| `imi.project.updated` | `imi.project.updated` | Project metadata changed |
| `imi.project.deactivated` | `imi.project.deactivated` | Project soft-deleted |
| `imi.work.started` | `imi.work.started` | Work item created |
| `imi.work.git_state_updated` | `imi.work.git_state_updated` | Git state synced |
| `imi.work.completed` | `imi.work.completed` | Work item completed |
| `imi.link.ticket_linked` | `imi.link.ticket_linked` | Ticket linked to work |
| `imi.pr.created` | `imi.pr.created` | PR opened |
| `imi.pr.merged` | `imi.pr.merged` | PR merged |
| `imi.pr.closed` | `imi.pr.closed` | PR closed |
| `imi.deploy.completed` | `imi.deploy.completed` | Deploy recorded |
| `imi.component.registered` | `imi.component.registered` | Component added |

#### FR-4.2: Event Consumption
Subscribe to external events via FastStream consumers:

| Source Event | Queue | Handler |
|-------------|-------|---------|
| `plane.ticket.created` | `imi.plane_events` | Auto-create linkage placeholder |
| `plane.ticket.updated` | `imi.plane_events` | Update linked ticket metadata |
| `git.push` | `imi.git_events` | Update git state for matching branch |
| `hookd.tool.mutation.*` | `imi.mutation_events` | Update last activity timestamp |
| `ci.deploy.completed` | `imi.deploy_events` | Create deploy linkage |

#### FR-4.3: Command Events
Accept command events for operations that don't need HTTP:

| Command | Routing Key | Queue |
|---------|------------|-------|
| Register project | `imi.cmd.register_project` | `imi.commands` |
| Start work | `imi.cmd.start_work` | `imi.commands` |
| Complete work | `imi.cmd.complete_work` | `imi.commands` |
| Link ticket | `imi.cmd.link_ticket` | `imi.commands` |

### FR-5: Component Manifest

#### FR-5.1: Register Component
- Accept: `name`, `path`, `kind` (submodule/local/external), `project_id` (FK to project registry), `metadata`
- For meta-repos (33GOD), tracks which sub-components exist
- Seed from existing `components.toml`

#### FR-5.2: Query Components
- List all components for a meta-repo project
- Filter by `kind`, `enabled`, metadata
- Include health/status indicators

#### FR-5.3: Component Relationships
- Define relationships: `depends_on`, `produces_events_for`, `consumes_events_from`
- Query dependency graph

### FR-6: Thin CLI (Optional)

#### FR-6.1: CLI Wrapper
- `imi projects list` → `GET /api/v1/projects`
- `imi projects register <remote_origin>` → `POST /api/v1/projects`
- `imi work list [--project <id>]` → `GET /api/v1/work`
- `imi work start <project_id> <branch>` → `POST /api/v1/work`
- `imi trace ticket <ticket_id>` → `GET /api/v1/trace/ticket/{id}`
- `imi components list` → `GET /api/v1/components`
- All commands are thin wrappers around HTTP calls to the API

## 3. Non-Functional Requirements

### NFR-1: Performance
- Project lookup by UUID or remote_origin: <10ms p95
- In-flight work query (per project): <25ms p95
- Trace chain query: <50ms p95
- Event publishing: fire-and-forget, <5ms added latency
- Supports 50+ concurrent agent connections

### NFR-2: Reliability
- Idempotent operations (safe retries)
- Soft-deletes (no data loss)
- Dead-letter queue for failed event handlers
- Health check endpoint at `/health`
- Graceful shutdown (drain in-flight requests)

### NFR-3: Data Integrity
- PostgreSQL ACID guarantees
- Foreign key constraints enforced at DB level
- Unique constraints prevent duplicate projects/work items
- JSONB with GIN indexes for metadata queries

### NFR-4: Observability
- Structured JSON logging with correlation IDs
- All Bloodbank events carry `correlation_id` for tracing
- Metrics endpoint at `/metrics` (Prometheus format, optional)
- Event count and error rate tracking

### NFR-5: Security
- API key authentication (header-based, simple bearer token)
- Read-only and read-write scopes
- No secrets in event payloads
- PostgreSQL role-based access (read-only role for dashboards)

### NFR-6: Deployability
- Docker-first (Dockerfile + docker-compose entry)
- Environment-based configuration (12-factor)
- Alembic migrations run on startup or via CLI
- Compatible with existing 33GOD docker-compose infrastructure

### NFR-7: Extensibility
- JSONB metadata columns on all core tables (no migration for custom fields)
- Plugin-friendly event consumption (new consumers don't require iMi changes)
- API versioned at `/api/v1/` with future `/api/v2/` path

## 4. Data Model Summary

### Core Entities

```
projects (UUID PK)
  ├── work_items (UUID PK, FK → projects)
  │     ├── git_state (embedded fields on work_items)
  │     ├── ticket_links (FK → work_items)
  │     ├── pr_links (FK → work_items)
  │     └── deploy_links (FK → work_items)
  └── components (UUID PK, FK → projects)
        └── component_relationships (self-referential)
```

### Key Fields per Entity

**projects**: `id`, `name`, `remote_origin`, `default_branch`, `trunk_path`, `description`, `metadata`, `active`, `created_at`, `updated_at`

**work_items**: `id`, `project_id`, `branch_name`, `work_type`, `source_type`, `source_ref`, `owner_id`, `worktree_path`, `has_uncommitted_changes`, `uncommitted_files_count`, `ahead_of_trunk`, `behind_trunk`, `last_commit_hash`, `last_commit_message`, `last_sync_at`, `status` (computed), `completed_at`, `completion_type`, `merge_commit_hash`, `metadata`, `active`, `created_at`, `updated_at`

**ticket_links**: `id`, `work_item_id`, `plane_ticket_id`, `ticket_title`, `ticket_url`, `metadata`, `created_at`

**pr_links**: `id`, `work_item_id`, `pr_number`, `pr_url`, `pr_state`, `pr_title`, `metadata`, `created_at`, `updated_at`

**deploy_links**: `id`, `work_item_id`, `deploy_id`, `environment`, `commit_hash`, `deployed_at`, `metadata`, `created_at`

**components**: `id`, `parent_project_id`, `name`, `path`, `kind`, `enabled`, `linked_project_id` (optional FK → projects), `metadata`, `created_at`, `updated_at`

## 5. API Surface Summary

### REST Endpoints (FastAPI)

| Method | Path | Purpose |
|--------|------|---------|
| `POST` | `/api/v1/projects` | Register project |
| `GET` | `/api/v1/projects` | List/search projects |
| `GET` | `/api/v1/projects/{id}` | Get project |
| `PATCH` | `/api/v1/projects/{id}` | Update project |
| `DELETE` | `/api/v1/projects/{id}` | Deactivate project |
| `POST` | `/api/v1/work` | Start work item |
| `GET` | `/api/v1/work` | List work items |
| `GET` | `/api/v1/work/{id}` | Get work item |
| `PATCH` | `/api/v1/work/{id}/git-state` | Update git state |
| `POST` | `/api/v1/work/{id}/complete` | Complete work |
| `POST` | `/api/v1/links/ticket` | Link ticket |
| `POST` | `/api/v1/links/pr` | Link PR |
| `POST` | `/api/v1/links/deploy` | Link deploy |
| `GET` | `/api/v1/trace/{entity_type}/{entity_id}` | Trace chain |
| `GET` | `/api/v1/components` | List components |
| `POST` | `/api/v1/components` | Register component |
| `GET` | `/api/v1/components/{id}` | Get component |
| `PATCH` | `/api/v1/components/{id}` | Update component |
| `GET` | `/api/v1/stats` | Registry statistics |
| `GET` | `/health` | Health check |

### Bloodbank Queues (FastStream)

| Queue | Routing Keys | Purpose |
|-------|-------------|---------|
| `imi.commands` | `imi.cmd.*` | Command events |
| `imi.plane_events` | `plane.ticket.*` | Plane webhook relay |
| `imi.git_events` | `git.push`, `git.pr.*` | Git events |
| `imi.mutation_events` | `tool.mutation.*` | hookd mutation events |
| `imi.deploy_events` | `ci.deploy.*` | CI/CD events |

## 6. Out of Scope

- **Git operations**: iMi records state, it does not run `git` commands
- **Worktree creation**: Agents/CLI create worktrees; iMi registers them
- **Agent scheduling**: Flume/orchestrators decide who works on what
- **Task management**: Plane owns backlog, priorities, sprints
- **Code analysis**: hookd + mutation-ledger handle code intelligence
- **MCP tools**: Deferred to v3.1 (API-first, MCP wraps API later)
- **Auth/RBAC**: v3.0 uses simple API key; proper RBAC deferred to v3.1

## 7. Dependencies

| Dependency | Type | Notes |
|-----------|------|-------|
| PostgreSQL 16+ | Infrastructure | Shared instance or dedicated |
| RabbitMQ (Bloodbank) | Infrastructure | Existing 33GOD instance |
| Bloodbank Python library | Library | `event_producers` package for EventEnvelope |
| Holyfields | Schema | Event payload schemas (Pydantic models) |
| Plane webhooks | Integration | Requires hookd or webhook relay to publish Plane events to Bloodbank |

## 8. Acceptance Criteria (System Level)

1. **AC-1**: Registering a project returns a stable UUID that persists across restarts
2. **AC-2**: Querying a project by `remote_origin` returns the project in <10ms
3. **AC-3**: Starting work on a project emits `imi.work.started` to Bloodbank within 1s
4. **AC-4**: Linking a Plane ticket to a work item enables forward trace (ticket → branch → PR)
5. **AC-5**: The `/stats` endpoint returns accurate counts of projects, work items, and components
6. **AC-6**: Concurrent registrations of the same `remote_origin` return the same UUID (idempotent)
7. **AC-7**: All existing `components.toml` entries are importable as component records
8. **AC-8**: Service starts, connects to Bloodbank, and passes health check within 30s
