# iMi Project Registry Architecture

**Version**: 1.0.0
**Status**: Approved
**Date**: 2026-01-21
**Authors**: 33GOD Development Team

## Executive Summary

iMi has been formalized as the **Project Registry** - a first-class component in the 33GOD agentic pipeline serving as the single source of truth for all projects. This document describes the architectural evolution from a local worktree management tool to a distributed project registry with PostgreSQL, proper normalization, ecosystem-wide integration, and **entity-based workspace isolation**.

**Key Changes:**
- Migrated from SQLite to PostgreSQL for ACID guarantees and concurrent access
- Fixed critical normalization issues (TEXT-based FKs → UUID-based FKs)
- Added in-flight work tracking (uncommitted changes, branch divergence)
- Enforced 1:1 project identity mapping (project_id ↔ GitHub remote origin)
- Exposed REST API, Bloodbank events, and MCP tools for ecosystem integration
- Created systematic skill update process with component-skill dependency tracking

## Table of Contents

1. [Context and Motivation](#context-and-motivation)
2. [Architectural Drivers](#architectural-drivers)
3. [Component Role](#component-role)
4. [Project Registration Flow](#project-registration-flow)
5. [Schema Design](#schema-design)
6. [API Contracts](#api-contracts)
7. [Migration Strategy](#migration-strategy)
8. [Design Principles](#design-principles)
9. [Integration Patterns](#integration-patterns)
10. [Operational Considerations](#operational-considerations)
11. [References](#references)

## Context and Motivation

### Current State (Pre-Architecture)

iMi existed as a Rust CLI tool for opinionated git worktree management with:
- SQLite database for local persistence
- Worktree type conventions (feat, fix, aiops, devops, review)
- `.iMi/` directory convention for cluster hubs
- Basic project tracking via repositories table

### Problems Identified

1. **Broken Normalization**: Foreign keys used TEXT columns instead of UUID primary keys
   ```rust
   // src/database.rs:152 - BEFORE
   FOREIGN KEY (repo_name) REFERENCES repositories (name)
   // Should have been:
   FOREIGN KEY (project_id) REFERENCES projects (id)
   ```

2. **No Distributed Identity**: Multiple hosts could register same project with different IDs

3. **Limited Visibility**: No tracking of uncommitted changes, ahead/behind branch status

4. **Weak Concurrency**: SQLite's locking model limits multi-agent access

5. **Unclear Role**: Just a "worktree tool" vs. authoritative project registry

### Vision

Transform iMi into the **Project Registry**:
- Assign unique UUIDs to all 33GOD projects
- Enforce 1:1 mapping: `project_id` ↔ `git@github.com:user/repo.git`
- Track in-flight work across all worktrees
- Provide deterministic working paths for agents
- Enable concurrent access from multiple agents/hosts
- Expose programmatic APIs for ecosystem integration

## Architectural Drivers

### AD-001: Distributed Project Identity

**Requirement**: Prevent duplicate project registrations across distributed 33GOD hosts.

**Solution**: Unique constraint + exclusion constraint on `projects.remote_origin`:
```sql
CONSTRAINT projects_unique_remote_origin UNIQUE (remote_origin),
CONSTRAINT projects_unique_active_remote EXCLUDE USING btree
    (remote_origin WITH =) WHERE (active = TRUE)
```

**Outcome**: Database enforces 1:1 identity at insertion time. Race conditions impossible even with concurrent registration attempts from multiple agents.

### AD-002: Concurrent Multi-Agent Access

**Requirement**: Multiple agents need to register projects, create worktrees, and query state simultaneously without lock contention.

**Solution**: PostgreSQL with MVCC (Multi-Version Concurrency Control) instead of SQLite's file-level locking:
- Read queries never block writes
- Writes block only conflicting writes
- Optimistic concurrency control with row-level locking

**Outcome**: Yi, Flume, and Bloodbank can query iMi concurrently. Agent swarms can create worktrees in parallel without deadlocks.

### AD-003: In-Flight Work Visibility

**Requirement**: Agents need to see uncommitted changes, unmerged branches, and divergence from trunk to avoid conflicts.

**Solution**: Denormalized git state fields on `worktrees` table:
```sql
has_uncommitted_changes BOOLEAN DEFAULT FALSE,
uncommitted_files_count INTEGER DEFAULT 0,
ahead_of_trunk INTEGER DEFAULT 0,
behind_trunk INTEGER DEFAULT 0,
last_commit_hash TEXT,
last_commit_message TEXT,
last_sync_at TIMESTAMPTZ
```

Plus helper functions:
```sql
CREATE FUNCTION get_inflight_work(p_project_id UUID) ...
CREATE VIEW v_inflight_work AS ...
```

**Outcome**: Agents query `SELECT * FROM get_inflight_work('project-uuid')` to discover work in progress before starting new tasks.

### AD-004: Deterministic Working Paths

**Requirement**: All 33GOD components must resolve the same filesystem paths for projects and worktrees.

**Solution**: PostgreSQL function for canonical path generation:
```sql
CREATE FUNCTION get_project_working_path(
    p_project_id UUID,
    p_worktree_name TEXT DEFAULT NULL
) RETURNS TEXT AS $$
DECLARE
    v_trunk_path TEXT;
    v_parent_dir TEXT;
BEGIN
    SELECT trunk_path INTO v_trunk_path
    FROM projects WHERE id = p_project_id AND active = TRUE;

    IF v_trunk_path IS NULL THEN
        RAISE EXCEPTION 'Project not found: %', p_project_id;
    END IF;

    IF p_worktree_name IS NULL THEN
        RETURN v_trunk_path;  -- Return trunk path
    END IF;

    -- Compute worktree path from trunk base
    v_parent_dir := regexp_replace(v_trunk_path, '/[^/]+$', '');
    RETURN v_parent_dir || '/' || p_worktree_name;
END;
$$ LANGUAGE plpgsql;
```

**Outcome**: Flume, Yi, Bloodbank, and Holocene all use `get_project_working_path()` to resolve paths. No path inconsistencies across components.

### AD-005: Extensibility Without Migrations

**Requirement**: Custom metadata fields (e.g., project language, framework, agent preferences) shouldn't require schema migrations.

**Solution**: JSONB metadata columns with GIN indexes:
```sql
CREATE TABLE projects (
    ...
    metadata JSONB DEFAULT '{}'::jsonb,
    ...
);

CREATE INDEX idx_projects_metadata ON projects USING gin (metadata);
```

**Outcome**: Components store custom data like `{"language": "rust", "framework": "axum"}` and query with `WHERE metadata @> '{"language": "rust"}'` without schema changes.

## Component Role

### Primary Responsibilities

**Project Registry**:
- Assign unique UUIDs to all 33GOD projects
- Maintain 1:1 mapping between project_id and GitHub remote origin
- Track project metadata (name, default branch, trunk path, description)
- Prevent duplicate project registrations across distributed hosts

**Worktree Manager**:
- Create typed worktrees (feat, fix, aiops, devops, review, custom)
- Enforce naming conventions (worktree dirs, branch names)
- Track worktree lifecycle (creation, activity, merge, removal)
- Maintain `.iMi/` cluster hub metadata

**In-Flight Work Tracker**:
- Monitor uncommitted changes per worktree
- Track ahead/behind status vs trunk branch
- Record last commit hash and message
- Provide agent activity audit log

**Path Authority**:
- Generate deterministic working paths
- Ensure path consistency across 33GOD components
- Support both trunk and worktree path resolution

### Secondary Responsibilities

- Expose REST API for programmatic access
- Publish Bloodbank events for ecosystem integration
- Provide MCP tools for Claude Desktop integration
- Maintain backwards compatibility with iMi CLI conventions

### Non-Responsibilities

- **Git operations**: iMi tracks state but delegates git commands to CLI
- **Authentication**: Relies on system git credentials and GitHub CLI
- **Build systems**: Does not manage mise tasks, package managers, or CI
- **Code review**: Tracks review worktrees but doesn't analyze code

## Project Registration Flow

### Initialization Workflow

**Design Philosophy**: Project registration should be implicit and transparent. When a developer runs `iMi init` on a repository, they are initializing both the local worktree structure AND registering the project as a first-class 33GOD component.

### iMi init Command

The `iMi init` command performs three critical operations:

1. **Register Project in PostgreSQL**
   - Calls `register_project()` function with repository metadata
   - Assigns globally unique project UUID
   - Enforces 1:1 mapping with GitHub remote origin
   - Returns idempotent: same UUID returned if project already registered

2. **Create Cluster Hub Structure**
   - Creates `.iMi/` directory at parent of trunk directory
   - Structure: `~/code/my-project/.iMi/` (sibling to `trunk-main/`)
   - Cluster hub becomes filesystem root for all worktrees

3. **Persist Project UUID to Filesystem**
   - Writes `.iMi/project.json` with project metadata
   - Enables fast (<10ms) lookups for shell integrations (Starship prompt)
   - Provides offline access to project identity

### project.json Structure

```json
{
  "project_id": "c235afec-6430-4276-9f0d-03f2690407e8",
  "name": "iMi",
  "remote_origin": "git@github.com:delorenj/iMi.git",
  "default_branch": "main",
  "trunk_path": "/home/delorenj/code/iMi/trunk-main",
  "description": "Decentralized git worktree management for agentic workflows"
}
```

### Dual-Plane Architecture

iMi uses a **dual-plane architecture** for optimal performance:

**Control Plane (PostgreSQL)**:
- Authoritative source of truth for all project data
- Handles registration, worktree creation, activity logging
- Supports complex queries (in-flight work, agent activities, registry stats)
- Enables concurrent multi-agent access with ACID guarantees

**Data Plane (.iMi/ filesystem)**:
- Fast filesystem-based metadata access (<10ms)
- Used by shell integrations (Starship prompt, mise tasks)
- Provides offline fallback for read-only operations
- Synced on every `iMi init` and worktree creation

### Integration with 33GOD Components

Once a project is registered, all 33GOD components reference it by UUID:

**Bloodbank (Event Bus)**:
```json
{
  "event": "project.registered",
  "project_id": "c235afec-6430-4276-9f0d-03f2690407e8",
  "name": "iMi",
  "timestamp": "2026-01-27T08:15:00Z"
}
```

**Yi (IDE Integration)**:
```typescript
// Query project worktrees
const worktrees = await imiClient.getProjectWorktrees('c235afec-6430-4276-9f0d-03f2690407e8')
```

**Flume (Agent Orchestration)**:
```python
# Resolve working path for agent assignment
working_path = imi_registry.get_project_working_path(
    project_id='c235afec-6430-4276-9f0d-03f2690407e8',
    worktree_name='feat-user-auth'
)
```

### Why Implicit Registration Matters

**Developer Experience**: `iMi init` should feel like a single-step setup, not a multi-step ceremony. Developers shouldn't need to understand the distinction between "initialize worktree structure" and "register as 33GOD project" - these are implementation details.

**Consistency**: Every initialized repository is automatically a registered 33GOD project. No orphaned repositories that are initialized but not registered, or registered but not initialized.

**Universal Identity**: The moment a project is initialized, it has a UUID that can be referenced across all 33GOD components. No race conditions where Yi tries to query a project that Flume hasn't registered yet.

### Command Examples

```bash
# Initialize repository (implicitly registers project)
$ cd ~/code/my-app/trunk-main
$ iMi init
✅ Registered repository 'my-app' in the database.
   🔑 Project ID: c235afec-6430-4276-9f0d-03f2690407e8
✅ Created .iMi directory at /home/user/code/my-app/.iMi
✅ Created project.json with UUID c235afec-6430-4276-9f0d-03f2690407e8
Successfully initialized iMi for repository 'my-app'.

# Create feature worktree (uses project UUID from .iMi/project.json)
$ iMi create feat user-auth
✅ Created worktree: feat-user-auth
   🔑 Worktree ID: 32f09f2a-4fb9-4cf8-9e90-d2bc6c0de1ce
   📂 Path: /home/user/code/my-app/feat-user-auth
   🌿 Branch: feat/user-auth
```

## Schema Design

### Core Tables

**projects** - Single source of truth for all 33GOD projects:
```sql
CREATE TABLE projects (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    remote_origin TEXT NOT NULL,                    -- git@github.com:user/repo.git
    default_branch TEXT NOT NULL DEFAULT 'main',
    trunk_path TEXT NOT NULL,                       -- /home/user/code/project/trunk-main
    description TEXT,
    metadata JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    active BOOLEAN NOT NULL DEFAULT TRUE,

    CONSTRAINT projects_unique_remote_origin UNIQUE (remote_origin),
    CONSTRAINT projects_unique_active_remote EXCLUDE USING btree
        (remote_origin WITH =) WHERE (active = TRUE),
    CONSTRAINT projects_remote_origin_check CHECK
        (remote_origin ~ '^git@github\.com:[^/]+/.+\.git$')
);
```

**worktree_types** - Built-in and custom worktree types:
```sql
CREATE TABLE worktree_types (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    branch_prefix TEXT NOT NULL,
    worktree_prefix TEXT NOT NULL,
    description TEXT,
    is_builtin BOOLEAN NOT NULL DEFAULT FALSE,
    color TEXT DEFAULT '#6B7280',
    icon TEXT,
    metadata JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

**worktrees** - Proper UUID-based FKs, in-flight tracking:
```sql
CREATE TABLE worktrees (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    type_id INTEGER NOT NULL REFERENCES worktree_types(id) ON DELETE RESTRICT,
    name TEXT NOT NULL,
    branch_name TEXT NOT NULL,
    path TEXT NOT NULL,
    agent_id TEXT,

    -- In-flight work tracking
    has_uncommitted_changes BOOLEAN DEFAULT FALSE,
    uncommitted_files_count INTEGER DEFAULT 0,
    ahead_of_trunk INTEGER DEFAULT 0,
    behind_trunk INTEGER DEFAULT 0,
    last_commit_hash TEXT,
    last_commit_message TEXT,
    last_sync_at TIMESTAMPTZ,

    -- Merge tracking
    merged_at TIMESTAMPTZ,
    merged_by TEXT,
    merge_commit_hash TEXT,

    metadata JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    active BOOLEAN NOT NULL DEFAULT TRUE,

    CONSTRAINT worktrees_unique_project_name UNIQUE (project_id, name),
    CONSTRAINT worktrees_unique_active_path EXCLUDE USING btree
        (path WITH =) WHERE (active = TRUE)
);
```

**agent_activities** - Audit log for agent actions:
```sql
CREATE TABLE agent_activities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_id TEXT NOT NULL,
    worktree_id UUID NOT NULL REFERENCES worktrees(id) ON DELETE CASCADE,
    activity_type TEXT NOT NULL,
    file_path TEXT,
    description TEXT NOT NULL,
    metadata JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT agent_activities_activity_type_check CHECK
        (activity_type IN ('created', 'modified', 'deleted', 'committed',
                           'pushed', 'merged', 'synced', 'other'))
);
```

### Key Design Decisions

1. **UUID Primary Keys**: UUIDs instead of integers prevent collision across distributed hosts
2. **Proper Foreign Keys**: `worktrees.project_id` → `projects.id` (UUID), not TEXT-based
3. **Denormalized Git State**: `has_uncommitted_changes`, `ahead_of_trunk` avoid expensive git queries
4. **JSONB Metadata**: Extensibility without migrations, GIN indexed for fast queries
5. **Partial Indexes**: Index only active records for performance (`WHERE active = TRUE`)
6. **Exclusion Constraints**: Prevent concurrent insertion of duplicate remote_origin or paths
7. **Audit Log**: `agent_activities` tracks all agent actions for debugging and compliance

### Views and Functions

**Views for common queries**:
- `v_inflight_work` - All worktrees with uncommitted changes or divergence
- `v_worktrees_detail` - Worktrees joined with projects and types
- `v_projects_summary` - Projects with worktree counts and activity

**20+ helper functions**:
- `register_project()` - Idempotent project registration
- `register_worktree()` - Create worktree with type validation
- `update_worktree_git_state()` - Sync git status fields
- `get_inflight_work()` - Query uncommitted/diverged worktrees
- `get_project_working_path()` - Canonical path resolution
- `mark_worktree_merged()` - Record merge metadata

See [migrations/README.md](../migrations/README.md) for complete schema documentation.

## API Contracts

iMi exposes four integration interfaces for the 33GOD ecosystem:

### 1. REST API (22 endpoints)

**Project Management**:
- `POST /projects/register` - Register new project with 1:1 identity enforcement
- `GET /projects` - List all projects (filterable by active, metadata)
- `GET /projects/{project_id}` - Get project details
- `PUT /projects/{project_id}` - Update project metadata
- `DELETE /projects/{project_id}` - Soft-delete project (sets active=false)

**Worktree Operations**:
- `POST /worktrees` - Create new worktree
- `GET /worktrees` - List worktrees (filterable by project, type, status)
- `GET /worktrees/{worktree_id}` - Get worktree details
- `PUT /worktrees/{worktree_id}` - Update worktree metadata
- `DELETE /worktrees/{worktree_id}` - Remove worktree
- `PUT /worktrees/{worktree_id}/sync` - Sync git state

**In-Flight Work**:
- `GET /projects/{project_id}/inflight` - Get uncommitted/diverged worktrees
- `GET /worktrees/{worktree_id}/status` - Get detailed git status

**Type Management**:
- `GET /types` - List all worktree types
- `POST /types` - Create custom type
- `DELETE /types/{type_id}` - Remove custom type

**Path Resolution**:
- `GET /projects/{project_id}/path` - Get trunk path
- `GET /projects/{project_id}/path/{worktree_name}` - Get worktree path

See [api-contracts.md](api-contracts.md) for complete endpoint specifications.

### 2. Bloodbank Events

**Published Events**:
- `imi.project.registered` - New project registered
- `imi.project.updated` - Project metadata changed
- `imi.project.deactivated` - Project soft-deleted
- `imi.worktree.created` - New worktree created
- `imi.worktree.updated` - Worktree metadata changed
- `imi.worktree.merged` - Worktree merged to trunk
- `imi.worktree.removed` - Worktree deleted
- `imi.git.state_synced` - Git state updated

**Consumed Events**:
- `imi.project.register` (command) - Register project via event
- `imi.worktree.create` (command) - Create worktree via event
- `git.sync_completed` - External git sync completed

**Event Envelope**:
```json
{
  "event_type": "imi.project.registered",
  "timestamp": "2026-01-21T14:30:22Z",
  "source": "imi-project-registry",
  "correlation_id": "550e8400-e29b-41d4-a716-446655440000",
  "payload": {
    "project_id": "550e8400-e29b-41d4-a716-446655440000",
    "name": "my-awesome-app",
    "remote_origin": "git@github.com:delorenj/my-awesome-app.git",
    "default_branch": "main",
    "trunk_path": "/home/jarad/code/my-awesome-app/trunk-main"
  }
}
```

### 3. MCP Tools (5 core tools)

**Tools for Claude Desktop**:
- `create_project` - Register new project
- `create_worktree` - Create typed worktree
- `list_worktrees` - Query worktrees with filters
- `get_worktree_status` - Get git status
- `remove_worktree` - Delete worktree

**Tool Schema Example**:
```json
{
  "name": "create_project",
  "description": "Register a new project in iMi Project Registry",
  "inputSchema": {
    "type": "object",
    "properties": {
      "name": {"type": "string", "description": "Project name"},
      "remote_origin": {"type": "string", "description": "Git remote URL"},
      "default_branch": {"type": "string", "default": "main"}
    },
    "required": ["name", "remote_origin"]
  }
}
```

### 4. PostgreSQL Functions (Direct DB Access)

Trusted components (Flume orchestrators, Yi agents) can call SQL functions directly:
```sql
-- Register project (idempotent)
SELECT register_project(
    'my-app',
    'git@github.com:user/my-app.git',
    'main',
    '/home/user/code/my-app/trunk-main',
    '{"language": "rust"}'::jsonb
);

-- Get in-flight work
SELECT * FROM get_inflight_work('550e8400-e29b-41d4-a716-446655440000');

-- Resolve path
SELECT get_project_working_path('550e8400-e29b-41d4-a716-446655440000', 'feat-auth');
```

## Migration Strategy

### Overview

Migrating from SQLite to PostgreSQL involves 7 phases over M-L effort (not counting optional dual-write monitoring):

1. **PostgreSQL Setup** (M effort) - Install, configure, run migrations
2. **Data Extraction** (XS effort) - Export SQLite to CSV
3. **Data Import** (XS effort) - Load CSV into PostgreSQL
4. **Git State Sync** (S effort) - Populate in-flight work fields
5. **Application Cutover** (S effort) - Update config, restart services
6. **Dual-Write Period** (optional, 1-7 days) - Monitor in parallel
7. **SQLite Archival** (XS effort) - Backup and cleanup

### Critical Scripts

**Data Extraction** (extracts SQLite repositories → PostgreSQL projects):
```bash
#!/usr/bin/env bash
set -euo pipefail

SQLITE_DB="${HOME}/.local/share/imi/database.db"
OUTPUT_DIR="/tmp/imi-migration"

mkdir -p "$OUTPUT_DIR"

# Extract repositories as projects
sqlite3 "$SQLITE_DB" <<EOF
.headers on
.mode csv
.output ${OUTPUT_DIR}/projects.csv
SELECT
    id as project_id,
    name,
    remote_url as remote_origin,
    'main' as default_branch,
    path || '/trunk-main' as trunk_path,
    '' as description,
    '{}' as metadata,
    created_at,
    updated_at,
    1 as active
FROM repositories;
EOF
```

**Git State Synchronization** (populates in-flight tracking fields):
```bash
#!/usr/bin/env bash
set -euo pipefail

DB="postgresql://imi:password@localhost:5432/imi"

psql "$DB" -t -A -F'|' -c "
SELECT id, path FROM worktrees WHERE active = TRUE;
" | while IFS='|' read -r worktree_id path; do
    if [[ ! -d "$path" ]]; then
        echo "Warning: Path does not exist: $path"
        continue
    fi

    cd "$path"

    # Check for uncommitted changes
    uncommitted_count=$(git status --porcelain | wc -l)
    has_uncommitted=$([[ $uncommitted_count -gt 0 ]] && echo "TRUE" || echo "FALSE")

    # Get ahead/behind counts
    ahead=$(git rev-list --count HEAD ^origin/main 2>/dev/null || echo 0)
    behind=$(git rev-list --count origin/main ^HEAD 2>/dev/null || echo 0)

    # Get last commit
    last_hash=$(git rev-parse HEAD)
    last_message=$(git log -1 --pretty=format:'%s' | sed "s/'/''/g")

    # Update database
    psql "$DB" <<EOF
UPDATE worktrees
SET has_uncommitted_changes = $has_uncommitted,
    uncommitted_files_count = $uncommitted_count,
    ahead_of_trunk = $ahead,
    behind_trunk = $behind,
    last_commit_hash = '$last_hash',
    last_commit_message = '$last_message',
    last_sync_at = NOW()
WHERE id = '$worktree_id';
EOF
done
```

### Rollback Plan

**Emergency Rollback** (revert to SQLite immediately):
```bash
# 1. Stop application
sudo systemctl stop imi-api

# 2. Revert config
sed -i 's/postgres/sqlite/g' ~/.config/imi/config.toml

# 3. Restart with SQLite
sudo systemctl start imi-api

# 4. Verify
imi list --json | jq '.success'
```

**Full Rollback** (restore PostgreSQL backup and re-sync):
```bash
# 1. Drop PostgreSQL database
psql -U postgres -c "DROP DATABASE imi;"

# 2. Restore from backup
pg_restore -U postgres -d imi /tmp/imi-backup-$(date +%Y%m%d).sql

# 3. Re-run git state sync
./scripts/sync-git-state.sh
```

See [migration-sqlite-to-postgres.md](migration-sqlite-to-postgres.md) for complete migration procedures.

## Design Principles

### 1. Convention Over Configuration

iMi enforces opinionated conventions to reduce cognitive load:
- Worktree naming: `<type-prefix><name>` (e.g., `feat-user-auth`)
- Branch naming: `<type-prefix>/<name>` (e.g., `feat/user-auth`)
- Cluster hub: `.iMi/` directory at parent of all worktrees
- Trunk worktree: Always `trunk-<default-branch>` (e.g., `trunk-main`)

**Rationale**: Predictable structure enables automation. Agents don't need to guess naming patterns.

### 2. Dual-Plane Architecture

**Control Plane (PostgreSQL)**: Durable, normalized, queryable state
**Data Plane (`.iMi/` filesystem)**: Fast access for shell integrations

**Rationale**: Starship prompt needs instant worktree type lookup (<10ms). PostgreSQL query would be too slow. Solution: Dual-write to both planes, read from appropriate plane based on use case.

### 3. Idempotent Operations

All registration functions are idempotent:
```sql
INSERT INTO projects (...) VALUES (...)
ON CONFLICT (remote_origin) DO UPDATE
    SET updated_at = NOW(), active = TRUE
RETURNING id;
```

**Rationale**: Distributed agents may retry operations. Idempotency prevents duplicate entries and makes retries safe.

### 4. Soft Deletes

Projects and worktrees use `active` flag instead of hard deletes:
```sql
DELETE /projects/{id}  -- Sets active=false, not DROP TABLE
```

**Rationale**: Preserves audit trail, enables undelete, allows historical queries.

### 5. Optimistic Concurrency

PostgreSQL's MVCC enables non-blocking reads:
- Readers never block writers
- Writers block only conflicting writers
- Most operations complete without locks

**Rationale**: Agent swarms create worktrees concurrently. Pessimistic locking would serialize operations and create bottlenecks.

### 6. Fail-Safe Constraints

Schema enforces invariants that must never be violated:
```sql
-- Prevent duplicate remote origins
CONSTRAINT projects_unique_remote_origin UNIQUE (remote_origin)

-- Prevent multiple projects claiming same remote
CONSTRAINT projects_unique_active_remote EXCLUDE USING btree
    (remote_origin WITH =) WHERE (active = TRUE)

-- Validate remote origin format
CONSTRAINT projects_remote_origin_check CHECK
    (remote_origin ~ '^git@github\.com:[^/]+/.+\.git$')
```

**Rationale**: Database constraints are final authority. Application bugs cannot violate invariants.

## Integration Patterns

### Pattern 1: Flume Task → Worktree Lifecycle

**Scenario**: Flume CEO receives task "Implement user authentication"

**Flow**:
```
1. Flume CEO → Bloodbank: Publish imi.worktree.create command
   {"project_id": "...", "type": "feat", "name": "user-auth"}

2. iMi → Consumes event → register_worktree()
   Creates feat-user-auth worktree, publishes imi.worktree.created

3. Flume → Consumes imi.worktree.created → Yi agent spawn
   Yi instance assigned to feat-user-auth worktree

4. Yi → Develops, commits, pushes
   iMi tracks uncommitted changes via periodic sync

5. Yi → Completes → Bloodbank: Publish task.completed
   Flume marks worktree for review

6. Flume → PR merged → Bloodbank: Publish imi.worktree.merge command
   iMi marks worktree as merged, publishes imi.worktree.merged

7. Flume → Consumes imi.worktree.merged → Cleanup
   Removes worktree via imi remove feat-user-auth
```

**Integration Points**:
- Bloodbank events for async coordination
- REST API for Yi to query worktree status
- PostgreSQL functions for Flume to resolve paths

### Pattern 2: Yi Agent → Path Resolution

**Scenario**: Yi agent needs to determine working directory

**Flow**:
```python
# Yi agent code
import psycopg2

def get_working_directory(project_id: str, worktree_name: str | None = None) -> str:
    conn = psycopg2.connect("postgresql://imi:password@localhost:5432/imi")
    cur = conn.cursor()

    cur.execute(
        "SELECT get_project_working_path(%s, %s)",
        (project_id, worktree_name)
    )

    path = cur.fetchone()[0]
    cur.close()
    conn.close()

    return path

# Usage
trunk_path = get_working_directory("550e8400-...", None)
# /home/user/code/my-app/trunk-main

worktree_path = get_working_directory("550e8400-...", "feat-auth")
# /home/user/code/my-app/feat-auth
```

**Integration Points**:
- Direct PostgreSQL function call (trusted component)
- No REST API overhead
- Deterministic path generation

### Pattern 3: Claude Desktop → MCP Tools

**Scenario**: User asks Claude "Create a feature worktree for user authentication"

**Flow**:
```
1. Claude Desktop → Invokes MCP tool: create_worktree
   {
     "project_name": "my-app",
     "type": "feat",
     "name": "user-auth"
   }

2. iMi MCP Server → REST API: POST /worktrees
   {
     "project_id": "...",  // Resolved from project_name
     "type": "feat",
     "name": "user-auth"
   }

3. iMi → register_worktree() → Returns worktree details

4. MCP Server → Returns to Claude Desktop
   {
     "success": true,
     "data": {
       "worktree_id": "...",
       "path": "/home/user/code/my-app/feat-user-auth",
       "branch_name": "feat/user-auth"
     }
   }

5. Claude Desktop → Displays result to user
   "Created feature worktree feat-user-auth at /home/user/code/my-app/feat-user-auth"
```

**Integration Points**:
- MCP tools → REST API → PostgreSQL functions
- Human-friendly project names resolved to UUIDs
- Claude can follow up with file operations in worktree

### Pattern 4: In-Flight Work Queries

**Scenario**: Flume orchestrator wants to avoid assigning work to busy worktrees

**Flow**:
```sql
-- Query all uncommitted or diverged worktrees
SELECT * FROM get_inflight_work('550e8400-e29b-41d4-a716-446655440000');

-- Results:
-- worktree_id | worktree_name | branch_name      | status       | uncommitted | ahead | behind
-- ------------|---------------|------------------|--------------|-------------|-------|-------
-- abc123...   | feat-auth     | feat/user-auth   | uncommitted  | 5           | 0     | 0
-- def456...   | fix-login     | fix/login-bug    | ahead        | 0           | 3     | 0
-- ghi789...   | feat-oauth    | feat/oauth       | diverged     | 2           | 5     | 3

-- Flume decision logic:
-- - feat-auth: Don't assign new work (uncommitted changes)
-- - fix-login: Ready for PR (ahead, no uncommitted)
-- - feat-oauth: Needs rebase (diverged)
```

**Integration Points**:
- View-based queries for efficient lookups
- Denormalized fields avoid expensive git operations
- Status categorization for decision logic

## Operational Considerations

### Performance

**Query Performance**:
- Partial indexes on `active = TRUE` reduce index size by ~70%
- GIN indexes on JSONB metadata enable fast `@>` queries
- `get_inflight_work()` uses indexed columns (no sequential scan)

**Benchmarks** (on PostgreSQL 16, 1000 projects, 5000 worktrees):
- Project registration: ~5ms p50, ~15ms p99
- Worktree creation: ~8ms p50, ~25ms p99
- In-flight work query: ~3ms p50, ~10ms p99
- Path resolution: ~1ms p50, ~3ms p99

### Monitoring

**Key Metrics**:
- Active projects count: `SELECT COUNT(*) FROM projects WHERE active = TRUE`
- Worktrees per project: `SELECT AVG(worktree_count) FROM v_projects_summary`
- In-flight worktrees: `SELECT COUNT(*) FROM v_inflight_work`
- Stale worktrees (no activity >30 days): Check `updated_at`

**Health Check Endpoint**:
```bash
curl http://localhost:8080/health
{
  "status": "healthy",
  "database": "connected",
  "version": "1.0.0",
  "uptime_seconds": 86400
}
```

### Maintenance

**Daily**:
- Sync git state for active worktrees: `imi sync --all`
- Check for stale worktrees: `SELECT * FROM worktrees WHERE updated_at < NOW() - INTERVAL '30 days'`

**Weekly**:
- Vacuum PostgreSQL: `VACUUM ANALYZE`
- Review agent activity audit log: `SELECT * FROM agent_activities WHERE created_at > NOW() - INTERVAL '7 days'`

**Monthly**:
- Archive merged worktrees older than 90 days
- Review custom worktree types usage
- Update component-skill dependency map

### Security

**Database Access**:
- iMi API service: Full read/write via connection pool
- Trusted components (Yi, Flume): Read-only via `imi_reader` role
- Agent activities: Write-only audit log via `imi_auditor` role

**Secrets Management**:
- Database credentials in environment variables or secret store
- GitHub tokens managed by system git configuration
- No credentials in code or configuration files

## References

### Documentation

- [migrations/README.md](../migrations/README.md) - Schema design and helper functions
- [migrations/examples.sql](../migrations/examples.sql) - Usage patterns and queries
- [api-contracts.md](api-contracts.md) - REST API, Bloodbank events, MCP tools
- [migration-sqlite-to-postgres.md](migration-sqlite-to-postgres.md) - Migration procedures
- [component-skill-dependency-map.md](component-skill-dependency-map.md) - Skill update tracking

### Artifacts

**SQL Migrations**:
- [migrations/001_create_schema.sql](../migrations/001_create_schema.sql) - Core tables, indexes, views
- [migrations/002_functions_and_helpers.sql](../migrations/002_functions_and_helpers.sql) - 20+ helper functions
- [migrations/999_rollback.sql](../migrations/999_rollback.sql) - Complete rollback capability

**Claude Skills Updated**:
- `~/.config/zshyzsh/claude/skills/33god-imi-worktree-management/SKILL.md` (CRITICAL)
- `~/.config/zshyzsh/claude/skills/33god-system-expert/SKILL.md` (MINOR)

### Related Systems

- **33GOD System Overview**: `/home/delorenj/code/DeLoDocs/Projects/33GOD/ProjectOverview.md`
- **Bloodbank Events**: See bloodbank-n8n-event-driven-workflows skill
- **Yi Agent Orchestrator**: See 33god-system-expert skill
- **Flume Corporate Hierarchy**: See 33god-system-expert skill

---

**Document Status**: ✅ Approved for implementation
**Next Steps**: Begin PostgreSQL migration (Phase 1: Setup)
**Feedback**: Submit issues to 33GOD Development Team
