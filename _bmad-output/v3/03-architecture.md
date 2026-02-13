---
date: 2026-02-11
author: BMAD Process (subagent)
status: approved
version: 3.0.0
stepsCompleted: [analysis, product-brief, prd, architecture]
inputDocuments:
  - _bmad-output/v3/01-product-brief.md
  - _bmad-output/v3/02-prd.md
  - iMi/docs/architecture-imi-project-registry.md
  - 33god-service-development/SKILL.md
  - bloodbank/event_producers/
  - hookd/GOD.md
---

# Architecture Document: iMi v3 — Project Intelligence Layer

## 1. Architecture Overview

iMi v3 is a single-process Python service that exposes two interfaces:

1. **FastAPI HTTP server** — REST API for synchronous queries and mutations
2. **FastStream RabbitMQ consumers** — Event-driven handlers for Bloodbank integration

Both interfaces share the same database layer (PostgreSQL via SQLAlchemy async) and event publisher. The service runs as one process with uvicorn serving HTTP and FastStream consumers running as async tasks.

## 2. C4 Diagrams

### 2.1 System Context (C4 Level 1)

```mermaid
C4Context
    title iMi v3 — System Context

    Person(dev, "Developer", "Queries projects, traces tickets")
    System(imi, "iMi v3", "Project Intelligence Layer")
    System_Ext(plane, "Plane", "Project management / ticket system")
    System_Ext(github, "GitHub", "Git hosting, PRs, webhooks")
    System(bloodbank, "Bloodbank", "RabbitMQ event bus")
    System(flume, "Flume", "Agent orchestrator")
    System(yi, "Yi", "Agent adapter")
    System(hookd, "hookd", "Tool mutation pipeline")
    System(candybar, "Candybar", "Dashboard UI")
    System_Ext(postgres, "PostgreSQL", "Persistent storage")

    Rel(dev, imi, "REST API / CLI")
    Rel(flume, imi, "REST API + Events")
    Rel(yi, imi, "REST API")
    Rel(imi, bloodbank, "Publishes events")
    Rel(bloodbank, imi, "Delivers events")
    Rel(plane, bloodbank, "Webhook events via relay")
    Rel(github, bloodbank, "Push/PR events via hookd")
    Rel(hookd, bloodbank, "tool.mutation.* events")
    Rel(candybar, imi, "REST API for dashboards")
    Rel(imi, postgres, "Read/Write")
```

### 2.2 Container Diagram (C4 Level 2)

```mermaid
C4Container
    title iMi v3 — Container Diagram

    Container_Boundary(imi_service, "iMi v3 Service") {
        Container(api, "FastAPI Server", "Python/uvicorn", "REST API endpoints for projects, work items, links, components, traces")
        Container(consumers, "FastStream Consumers", "Python/FastStream", "Event handlers for Plane, Git, hookd, CI/CD events")
        Container(publisher, "Event Publisher", "Python/aio-pika", "Publishes iMi events to Bloodbank")
        Container(db_layer, "Database Layer", "SQLAlchemy async + asyncpg", "Repository pattern over PostgreSQL")
    }

    ContainerDb(postgres, "PostgreSQL", "Database", "Projects, work items, links, components")
    Container_Ext(rabbitmq, "RabbitMQ", "Message Broker", "Bloodbank event bus")

    Rel(api, db_layer, "Reads/Writes")
    Rel(api, publisher, "Publishes events after mutations")
    Rel(consumers, db_layer, "Reads/Writes")
    Rel(consumers, publisher, "Publishes events after processing")
    Rel(publisher, rabbitmq, "AMQP publish")
    Rel(rabbitmq, consumers, "AMQP consume")
    Rel(db_layer, postgres, "asyncpg")
```

### 2.3 Component Diagram (C4 Level 3)

```mermaid
C4Component
    title iMi v3 — Component Diagram

    Container_Boundary(imi, "iMi v3 Service") {

        Component(proj_router, "Project Router", "FastAPI Router", "/api/v1/projects — CRUD operations")
        Component(work_router, "Work Router", "FastAPI Router", "/api/v1/work — Work item operations")
        Component(link_router, "Link Router", "FastAPI Router", "/api/v1/links — Ticket/PR/deploy linkage")
        Component(trace_router, "Trace Router", "FastAPI Router", "/api/v1/trace — Chain traceability")
        Component(comp_router, "Component Router", "FastAPI Router", "/api/v1/components — Manifest")
        Component(stats_router, "Stats Router", "FastAPI Router", "/api/v1/stats + /health")

        Component(cmd_consumer, "Command Consumer", "FastStream", "imi.cmd.* command handlers")
        Component(plane_consumer, "Plane Consumer", "FastStream", "plane.ticket.* event handlers")
        Component(git_consumer, "Git Consumer", "FastStream", "git.push, git.pr.* handlers")
        Component(mutation_consumer, "Mutation Consumer", "FastStream", "tool.mutation.* handlers")

        Component(proj_service, "ProjectService", "Python", "Project business logic")
        Component(work_service, "WorkService", "Python", "Work item business logic")
        Component(link_service, "LinkService", "Python", "Linkage business logic")
        Component(trace_service, "TraceService", "Python", "Chain trace logic")
        Component(comp_service, "ComponentService", "Python", "Component manifest logic")

        Component(proj_repo, "ProjectRepository", "SQLAlchemy", "Project DB operations")
        Component(work_repo, "WorkRepository", "SQLAlchemy", "Work item DB operations")
        Component(link_repo, "LinkRepository", "SQLAlchemy", "Link DB operations")
        Component(comp_repo, "ComponentRepository", "SQLAlchemy", "Component DB operations")

        Component(event_pub, "EventPublisher", "aio-pika", "Bloodbank event publishing")
    }

    Rel(proj_router, proj_service, "")
    Rel(work_router, work_service, "")
    Rel(link_router, link_service, "")
    Rel(trace_router, trace_service, "")
    Rel(comp_router, comp_service, "")

    Rel(cmd_consumer, proj_service, "")
    Rel(cmd_consumer, work_service, "")
    Rel(plane_consumer, link_service, "")
    Rel(git_consumer, work_service, "")
    Rel(mutation_consumer, work_service, "")

    Rel(proj_service, proj_repo, "")
    Rel(proj_service, event_pub, "")
    Rel(work_service, work_repo, "")
    Rel(work_service, event_pub, "")
    Rel(link_service, link_repo, "")
    Rel(link_service, event_pub, "")
    Rel(comp_service, comp_repo, "")
    Rel(comp_service, event_pub, "")
    Rel(trace_service, link_repo, "")
    Rel(trace_service, work_repo, "")
```

### 2.4 Event Flow Diagram

```mermaid
sequenceDiagram
    participant Agent as AI Agent
    participant API as iMi API
    participant DB as PostgreSQL
    participant BB as Bloodbank
    participant Plane as Plane
    participant Flume as Flume

    Note over Agent,Flume: Work Lifecycle Flow

    Agent->>API: POST /api/v1/work {project_id, branch, source: {type: plane_ticket, ref: PROJ-123}}
    API->>DB: INSERT work_item
    API->>BB: publish imi.work.started
    API-->>Agent: 201 {work_item_id}

    BB->>Flume: imi.work.started (notification)

    Note over Agent,Flume: Git State Sync

    Agent->>API: PATCH /api/v1/work/{id}/git-state {ahead: 3, uncommitted: true}
    API->>DB: UPDATE work_items SET ...
    API->>BB: publish imi.work.git_state_updated
    API-->>Agent: 200

    Note over Agent,Flume: PR Created

    Agent->>API: POST /api/v1/links/pr {work_item_id, pr_number: 42, pr_url, pr_state: open}
    API->>DB: INSERT pr_link
    API->>BB: publish imi.pr.created
    API-->>Agent: 201

    Note over Agent,Flume: Plane Ticket Auto-Link

    Plane->>BB: plane.ticket.updated {ticket_id: PROJ-123, status: in_progress}
    BB->>API: (FastStream consumer)
    API->>DB: UPDATE ticket_link metadata
```

## 3. Project Structure

```
~/code/33GOD/iMi/
├── _bmad-output/v3/           # BMAD spec artifacts (this)
├── src/
│   └── imi/
│       ├── __init__.py
│       ├── main.py            # FastAPI app + FastStream app creation
│       ├── config.py          # Pydantic BaseSettings
│       ├── database.py        # SQLAlchemy async engine, session factory
│       │
│       ├── models/            # SQLAlchemy ORM models
│       │   ├── __init__.py
│       │   ├── project.py     # Project model
│       │   ├── work_item.py   # WorkItem model (+ git state fields)
│       │   ├── ticket_link.py # TicketLink model
│       │   ├── pr_link.py     # PRLink model
│       │   ├── deploy_link.py # DeployLink model
│       │   └── component.py   # Component model
│       │
│       ├── schemas/           # Pydantic request/response schemas
│       │   ├── __init__.py
│       │   ├── project.py
│       │   ├── work_item.py
│       │   ├── link.py
│       │   ├── trace.py
│       │   ├── component.py
│       │   └── events.py      # Event payload schemas
│       │
│       ├── repositories/      # Database access (repository pattern)
│       │   ├── __init__.py
│       │   ├── base.py        # BaseRepository with common CRUD
│       │   ├── project.py
│       │   ├── work_item.py
│       │   ├── link.py
│       │   └── component.py
│       │
│       ├── services/          # Business logic
│       │   ├── __init__.py
│       │   ├── project.py
│       │   ├── work.py
│       │   ├── link.py
│       │   ├── trace.py
│       │   └── component.py
│       │
│       ├── api/               # FastAPI routers
│       │   ├── __init__.py
│       │   ├── projects.py
│       │   ├── work.py
│       │   ├── links.py
│       │   ├── trace.py
│       │   ├── components.py
│       │   ├── stats.py
│       │   └── health.py
│       │
│       ├── consumers/         # FastStream event consumers
│       │   ├── __init__.py
│       │   ├── commands.py    # imi.cmd.* handlers
│       │   ├── plane.py       # plane.ticket.* handlers
│       │   ├── git.py         # git.push, git.pr.* handlers
│       │   └── mutations.py   # tool.mutation.* handlers
│       │
│       ├── events/            # Event publishing
│       │   ├── __init__.py
│       │   └── publisher.py   # EventPublisher wrapping Bloodbank patterns
│       │
│       └── cli/               # Optional Typer CLI
│           ├── __init__.py
│           └── main.py
│
├── migrations/                # Alembic migrations
│   ├── env.py
│   ├── alembic.ini
│   └── versions/
│       └── 001_initial_schema.py
│
├── tests/
│   ├── conftest.py            # Fixtures: async DB, mock Bloodbank
│   ├── test_api/
│   │   ├── test_projects.py
│   │   ├── test_work.py
│   │   ├── test_links.py
│   │   ├── test_trace.py
│   │   └── test_components.py
│   ├── test_consumers/
│   │   ├── test_commands.py
│   │   ├── test_plane.py
│   │   └── test_git.py
│   └── test_services/
│       ├── test_project_service.py
│       ├── test_work_service.py
│       └── test_trace_service.py
│
├── pyproject.toml
├── Dockerfile
├── docker-compose.yml         # iMi + PostgreSQL + RabbitMQ (dev)
├── mise.toml                  # mise task runner config
├── GOD.md                     # Updated GOD doc (post-implementation)
└── README.md
```

## 4. Database Schema

### 4.1 Entity-Relationship Diagram

```mermaid
erDiagram
    projects ||--o{ work_items : "has"
    projects ||--o{ components : "contains"
    work_items ||--o{ ticket_links : "linked to"
    work_items ||--o{ pr_links : "linked to"
    work_items ||--o{ deploy_links : "linked to"
    components ||--o{ component_relationships : "relates to"

    projects {
        uuid id PK
        text name
        text remote_origin UK
        text default_branch
        text trunk_path
        text description
        jsonb metadata
        boolean active
        timestamptz created_at
        timestamptz updated_at
    }

    work_items {
        uuid id PK
        uuid project_id FK
        text branch_name
        text work_type
        text source_type
        text source_ref
        text owner_id
        text worktree_path
        boolean has_uncommitted_changes
        integer uncommitted_files_count
        integer ahead_of_trunk
        integer behind_trunk
        text last_commit_hash
        text last_commit_message
        timestamptz last_sync_at
        text status
        timestamptz completed_at
        text completion_type
        text merge_commit_hash
        jsonb metadata
        boolean active
        timestamptz created_at
        timestamptz updated_at
    }

    ticket_links {
        uuid id PK
        uuid work_item_id FK
        text plane_ticket_id
        text ticket_title
        text ticket_url
        jsonb metadata
        timestamptz created_at
    }

    pr_links {
        uuid id PK
        uuid work_item_id FK
        integer pr_number
        text pr_url
        text pr_state
        text pr_title
        jsonb metadata
        timestamptz created_at
        timestamptz updated_at
    }

    deploy_links {
        uuid id PK
        uuid work_item_id FK
        text deploy_id
        text environment
        text commit_hash
        timestamptz deployed_at
        jsonb metadata
        timestamptz created_at
    }

    components {
        uuid id PK
        uuid parent_project_id FK
        text name
        text path
        text kind
        boolean enabled
        uuid linked_project_id FK
        jsonb metadata
        timestamptz created_at
        timestamptz updated_at
    }

    component_relationships {
        uuid id PK
        uuid source_component_id FK
        uuid target_component_id FK
        text relationship_type
        jsonb metadata
        timestamptz created_at
    }
```

### 4.2 Key Indexes

```sql
-- Projects
CREATE UNIQUE INDEX idx_projects_remote_origin ON projects(remote_origin) WHERE active = true;
CREATE INDEX idx_projects_name_trgm ON projects USING gin(name gin_trgm_ops);
CREATE INDEX idx_projects_metadata ON projects USING gin(metadata);

-- Work Items
CREATE UNIQUE INDEX idx_work_items_project_branch ON work_items(project_id, branch_name) WHERE active = true;
CREATE INDEX idx_work_items_owner ON work_items(owner_id) WHERE active = true;
CREATE INDEX idx_work_items_source ON work_items(source_type, source_ref) WHERE active = true;
CREATE INDEX idx_work_items_status ON work_items(status) WHERE active = true;

-- Links
CREATE INDEX idx_ticket_links_ticket ON ticket_links(plane_ticket_id);
CREATE INDEX idx_pr_links_pr ON pr_links(pr_number);
CREATE INDEX idx_deploy_links_commit ON deploy_links(commit_hash);

-- Components
CREATE INDEX idx_components_parent ON components(parent_project_id);
CREATE INDEX idx_components_kind ON components(kind);
```

### 4.3 Key Constraints

```sql
-- One active project per remote_origin
ALTER TABLE projects ADD CONSTRAINT uq_projects_active_remote
    EXCLUDE USING btree (remote_origin WITH =) WHERE (active = true);

-- One active work item per project+branch
ALTER TABLE work_items ADD CONSTRAINT uq_work_active_branch
    EXCLUDE USING btree (project_id WITH =, branch_name WITH =) WHERE (active = true);

-- Valid work types
ALTER TABLE work_items ADD CONSTRAINT chk_work_type
    CHECK (work_type IN ('feature', 'fix', 'experiment', 'review', 'devops', 'aiops'));

-- Valid source types
ALTER TABLE work_items ADD CONSTRAINT chk_source_type
    CHECK (source_type IN ('plane_ticket', 'bloodbank_command', 'agent_assignment', 'manual'));

-- Valid completion types
ALTER TABLE work_items ADD CONSTRAINT chk_completion_type
    CHECK (completion_type IS NULL OR completion_type IN ('merged', 'abandoned', 'superseded'));

-- Valid PR states
ALTER TABLE pr_links ADD CONSTRAINT chk_pr_state
    CHECK (pr_state IN ('open', 'merged', 'closed'));

-- Valid component kinds
ALTER TABLE components ADD CONSTRAINT chk_component_kind
    CHECK (kind IN ('submodule', 'local', 'external'));
```

## 5. Technology Decisions

### TD-1: Single Process (FastAPI + FastStream)

**Decision**: Run FastAPI and FastStream in one process, not separate services.

**Rationale**: iMi is a small-to-medium service. Splitting HTTP and event consumers into separate processes adds deployment complexity without meaningful scaling benefit. FastStream's broker can run alongside uvicorn's event loop.

**Implementation**:
```python
# main.py
from fastapi import FastAPI
from faststream.rabbit import RabbitBroker
from contextlib import asynccontextmanager

broker = RabbitBroker(settings.rabbitmq_url)

@asynccontextmanager
async def lifespan(app: FastAPI):
    await broker.start()
    yield
    await broker.close()

app = FastAPI(title="iMi v3", lifespan=lifespan)
```

### TD-2: Repository Pattern for DB Access

**Decision**: Use repository classes wrapping SQLAlchemy, not raw SQL or ORM query everywhere.

**Rationale**: Testability (mock repo in unit tests), single place for query logic, clean separation from business logic in services.

### TD-3: Computed `status` Field

**Decision**: The `status` field on work_items is computed at query time, not stored.

**Rationale**: Status depends on `has_uncommitted_changes`, `ahead_of_trunk`, `behind_trunk`, and `completed_at`. Storing it creates a sync problem. Computing it from the source fields is trivial and always correct.

```python
@hybrid_property
def status(self) -> str:
    if self.completed_at:
        return self.completion_type  # merged/abandoned/superseded
    if self.has_uncommitted_changes:
        return "uncommitted"
    if self.ahead_of_trunk > 0 and self.behind_trunk > 0:
        return "diverged"
    if self.ahead_of_trunk > 0:
        return "ahead"
    if self.behind_trunk > 0:
        return "behind"
    return "clean"
```

### TD-4: Event Publishing Pattern

**Decision**: All event publishing goes through a shared `EventPublisher` class that wraps Bloodbank's `create_envelope()` and `Publisher`.

**Rationale**: Consistent envelope creation, single place to add correlation ID tracking, easy to mock in tests.

```python
class EventPublisher:
    async def publish(self, event_type: str, payload: dict, correlation_id: str | None = None):
        envelope = create_envelope(
            event_type=event_type,
            payload=payload,
            source=create_source(host=settings.hostname, trigger_type="system", app="imi"),
            correlation_ids=[correlation_id] if correlation_id else [],
        )
        await self._publisher.publish_event(routing_key=event_type, envelope=envelope)
```

### TD-5: Alembic for Migrations

**Decision**: Use Alembic with async support for database migrations.

**Rationale**: Industry standard for SQLAlchemy. Supports auto-generation of migration diffs. Runs on service startup in dev, manually in production.

### TD-6: No MCP in v3.0

**Decision**: Defer MCP tool interface to v3.1.

**Rationale**: The REST API is the primary interface. MCP tools would just wrap API calls (like the v2 design showed). Ship the API first, add MCP as a thin layer later.

## 6. Integration Architecture

### 6.1 Bloodbank Integration

**Exchange**: `bloodbank.events.v1` (topic, durable)

**Published Events** (all use standard EventEnvelope):

| Routing Key | Published When |
|------------|---------------|
| `imi.project.registered` | Project created |
| `imi.project.updated` | Project metadata changed |
| `imi.project.deactivated` | Project soft-deleted |
| `imi.work.started` | Work item created |
| `imi.work.git_state_updated` | Git state synced |
| `imi.work.completed` | Work completed (merged/abandoned) |
| `imi.link.ticket_linked` | Ticket linked to work |
| `imi.pr.created` | PR link created |
| `imi.pr.merged` | PR marked merged |
| `imi.pr.closed` | PR marked closed |
| `imi.deploy.completed` | Deploy link created |
| `imi.component.registered` | Component created |

**Consumer Queues**:

| Queue Name | Routing Keys | Handler |
|-----------|-------------|---------|
| `imi.commands` | `imi.cmd.register_project`, `imi.cmd.start_work`, `imi.cmd.complete_work`, `imi.cmd.link_ticket` | `consumers/commands.py` |
| `imi.plane_events` | `plane.ticket.created`, `plane.ticket.updated` | `consumers/plane.py` |
| `imi.git_events` | `git.push`, `git.pr.opened`, `git.pr.merged`, `git.pr.closed` | `consumers/git.py` |
| `imi.mutation_events` | `tool.mutation.*` | `consumers/mutations.py` |

### 6.2 PostgreSQL Connection

```python
# config.py
class Settings(BaseSettings):
    database_url: str = "postgresql+asyncpg://imi:imi@localhost:5432/imi"
    database_pool_size: int = 10
    database_max_overflow: int = 5

# database.py
engine = create_async_engine(
    settings.database_url,
    pool_size=settings.database_pool_size,
    max_overflow=settings.database_max_overflow,
    echo=settings.debug,
)
async_session = async_sessionmaker(engine, class_=AsyncSession, expire_on_commit=False)
```

### 6.3 Service Registry Entry

```yaml
# Addition to services/registry.yaml
imi-v3:
  name: "imi-v3"
  description: "Project Intelligence Layer - Registry, work tracking, metadata linkage"
  type: "hybrid"
  queue_names:
    - "imi.commands"
    - "imi.plane_events"
    - "imi.git_events"
    - "imi.mutation_events"
  routing_keys:
    - "imi.cmd.*"
    - "plane.ticket.*"
    - "git.push"
    - "git.pr.*"
    - "tool.mutation.*"
  produces:
    - "imi.project.*"
    - "imi.work.*"
    - "imi.link.*"
    - "imi.pr.*"
    - "imi.deploy.*"
    - "imi.component.*"
  status: "active"
  owner: "33GOD"
  tags:
    - "registry"
    - "intelligence"
    - "traceability"
  endpoints:
    api: "http://localhost:8400/api/v1"
    health: "http://localhost:8400/health"
    docs: "http://localhost:8400/docs"
```

## 7. Deployment Architecture

### 7.1 Docker Compose (Development)

```yaml
services:
  imi:
    build: .
    ports:
      - "8400:8400"
    environment:
      - IMI_DATABASE_URL=postgresql+asyncpg://imi:imi@postgres:5432/imi
      - IMI_RABBITMQ_URL=amqp://guest:guest@rabbitmq:5672/
      - IMI_LOG_LEVEL=DEBUG
    depends_on:
      postgres:
        condition: service_healthy
      rabbitmq:
        condition: service_healthy

  postgres:
    image: postgres:16
    environment:
      POSTGRES_DB: imi
      POSTGRES_USER: imi
      POSTGRES_PASSWORD: imi
    ports:
      - "5433:5432"
    volumes:
      - imi_pgdata:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U imi"]
      interval: 5s
      timeout: 5s
      retries: 5

  rabbitmq:
    image: rabbitmq:3-management
    ports:
      - "5672:5672"
      - "15672:15672"
    healthcheck:
      test: rabbitmq-diagnostics check_port_connectivity
      interval: 5s
      timeout: 5s
      retries: 5

volumes:
  imi_pgdata:
```

### 7.2 Dockerfile

```dockerfile
FROM python:3.12-slim

WORKDIR /app

# Install uv
COPY --from=ghcr.io/astral-sh/uv:latest /uv /usr/local/bin/uv

# Install dependencies
COPY pyproject.toml uv.lock ./
RUN uv sync --frozen --no-dev

# Copy source
COPY src/ src/
COPY migrations/ migrations/

# Run migrations then start service
CMD ["sh", "-c", "uv run alembic upgrade head && uv run uvicorn imi.main:app --host 0.0.0.0 --port 8400"]
```

## 8. Error Handling Strategy

### HTTP Errors
Standard JSON error responses:
```json
{
    "detail": {
        "code": "NOT_FOUND",
        "message": "Project not found",
        "params": {"project_id": "..."}
    }
}
```

### Event Processing Errors
- Validation errors: log + dead-letter queue (do not retry)
- Transient errors (DB timeout, connection): auto-retry via RabbitMQ redelivery (up to 3 times)
- Publish failures: log error, do not fail the HTTP request (fire-and-forget)

### Database Errors
- Unique constraint violations → return 409 Conflict
- FK violations → return 400 Bad Request with helpful message
- Connection pool exhaustion → return 503 Service Unavailable

## 9. Testing Strategy

| Layer | Tool | Coverage Target |
|-------|------|----------------|
| Unit (services) | pytest + mock repos | Business logic, status computation, trace building |
| Integration (API) | pytest + httpx + TestClient | All endpoints, error cases, idempotency |
| Integration (consumers) | pytest + in-memory RabbitMQ | Event handling, envelope unwrapping |
| Integration (DB) | pytest + testcontainers-postgres | Repository queries, constraints, migrations |
| E2E | docker-compose + pytest | Full flow: HTTP → DB → Bloodbank → consumer |

**Test fixtures**:
- `conftest.py`: async PostgreSQL test database, mock EventPublisher, mock RabbitMQ broker
- Factory functions for creating test projects, work items, links

## 10. Configuration Reference

```python
class Settings(BaseSettings):
    # Service
    service_name: str = "imi"
    host: str = "0.0.0.0"
    port: int = 8400
    debug: bool = False
    log_level: str = "INFO"

    # Database
    database_url: str = "postgresql+asyncpg://imi:imi@localhost:5432/imi"
    database_pool_size: int = 10
    database_max_overflow: int = 5

    # RabbitMQ
    rabbitmq_url: str = "amqp://guest:guest@localhost:5672/"
    exchange_name: str = "bloodbank.events.v1"

    # API
    api_key: str = ""  # Empty = no auth (dev mode)
    cors_origins: list[str] = ["*"]

    model_config = SettingsConfigDict(
        env_prefix="IMI_",
        env_file=".env",
    )
```

## 11. Migration from v2

The existing iMi v2 Rust CLI data (SQLite) is migrated as follows:

1. **Projects**: `repositories` table → `projects` table (UUID preserved)
2. **Worktrees**: `worktrees` table → `work_items` table (remap fields)
3. **Claims**: `claims` table → `work_items.owner_id` field
4. **Activities**: `agent_activities` → dropped (hookd now handles this)

Migration is a one-time script, not part of the service. See `scripts/migrate_v2.py`.
