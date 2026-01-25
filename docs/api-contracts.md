# iMi API Contracts for 33GOD Integration

**Version**: 2.0.0
**Component**: iMi Project Registry
**Purpose**: Formal API contracts for 33GOD ecosystem integration
**Created**: 2026-01-21

## Overview

iMi exposes its Project Registry functionality through three interfaces:

1. **REST API**: HTTP endpoints for CRUD operations on projects and worktrees
2. **Bloodbank Events**: Event-driven integration via RabbitMQ
3. **MCP Tools**: Direct tool invocation for Claude Desktop agents
4. **PostgreSQL Functions**: Direct database access for trusted components

## 1. REST API Contracts

### Base Configuration

```
Base URL: http://localhost:8080/api/v1
Authentication: Bearer token (JWT) or API key
Content-Type: application/json
```

### 1.1 Project Endpoints

#### POST /projects/register

Register a new project with automatic deduplication.

**Request:**
```json
{
  "name": "my-awesome-app",
  "remote_origin": "git@github.com:delorenj/my-awesome-app.git",
  "default_branch": "main",
  "trunk_path": "/home/jarad/code/my-awesome-app/trunk-main",
  "metadata": {
    "language": "rust",
    "framework": "axum",
    "stack": "backend"
  }
}
```

**Response (201 Created):**
```json
{
  "success": true,
  "data": {
    "project_id": "550e8400-e29b-41d4-a716-446655440000",
    "name": "my-awesome-app",
    "remote_origin": "git@github.com:delorenj/my-awesome-app.git",
    "default_branch": "main",
    "trunk_path": "/home/jarad/code/my-awesome-app/trunk-main",
    "active": true,
    "created_at": "2026-01-21T14:30:22Z",
    "updated_at": "2026-01-21T14:30:22Z"
  }
}
```

**Idempotency:** ON CONFLICT returns existing project with 200 OK instead of 201

**Error Cases:**
- `400 Bad Request`: Invalid remote_origin format
- `409 Conflict`: Active project with same remote_origin exists (shouldn't happen due to ON CONFLICT logic)

#### GET /projects/{project_id}

Retrieve project by UUID.

**Response (200 OK):**
```json
{
  "success": true,
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "name": "my-awesome-app",
    "remote_origin": "git@github.com:delorenj/my-awesome-app.git",
    "default_branch": "main",
    "trunk_path": "/home/jarad/code/my-awesome-app/trunk-main",
    "description": null,
    "metadata": {"language": "rust"},
    "active": true,
    "created_at": "2026-01-21T14:30:22Z",
    "updated_at": "2026-01-21T14:30:22Z",
    "worktree_counts": {
      "active": 3,
      "uncommitted": 1,
      "unmerged": 2
    }
  }
}
```

#### GET /projects?remote_origin={url}

Lookup project by GitHub remote origin.

**Query Parameters:**
- `remote_origin` (required): Full GitHub URL

**Response:** Same as GET /projects/{project_id}

#### GET /projects/search?q={query}

Fuzzy search projects by name or remote origin.

**Query Parameters:**
- `q` (required): Search query string
- `limit` (optional, default: 20): Max results

**Response (200 OK):**
```json
{
  "success": true,
  "data": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "name": "my-awesome-app",
      "remote_origin": "git@github.com:delorenj/my-awesome-app.git",
      "trunk_path": "/home/jarad/code/my-awesome-app/trunk-main",
      "relevance": 0.95
    }
  ]
}
```

#### PATCH /projects/{project_id}

Update project metadata or configuration.

**Request:**
```json
{
  "description": "Updated description",
  "metadata": {
    "language": "rust",
    "team": "backend"
  }
}
```

**Response:** Same as GET /projects/{project_id}

#### DELETE /projects/{project_id}

Soft delete project (sets active=false).

**Response (204 No Content)**

### 1.2 Worktree Endpoints

#### POST /projects/{project_id}/worktrees

Register a new worktree.

**Request:**
```json
{
  "type": "feat",
  "name": "feat-user-auth",
  "branch_name": "feat/user-auth",
  "path": "/home/jarad/code/my-awesome-app/feat-user-auth",
  "agent_id": "agent-coder-1",
  "metadata": {
    "story_id": "PROJ-123",
    "priority": "high"
  }
}
```

**Response (201 Created):**
```json
{
  "success": true,
  "data": {
    "id": "660e8400-e29b-41d4-a716-446655440001",
    "project_id": "550e8400-e29b-41d4-a716-446655440000",
    "type_id": 1,
    "type_name": "feat",
    "name": "feat-user-auth",
    "branch_name": "feat/user-auth",
    "path": "/home/jarad/code/my-awesome-app/feat-user-auth",
    "agent_id": "agent-coder-1",
    "has_uncommitted_changes": false,
    "uncommitted_files_count": 0,
    "ahead_of_trunk": 0,
    "behind_trunk": 0,
    "active": true,
    "created_at": "2026-01-21T14:35:00Z",
    "updated_at": "2026-01-21T14:35:00Z"
  }
}
```

**Error Cases:**
- `404 Not Found`: Project doesn't exist
- `400 Bad Request`: Invalid worktree type
- `409 Conflict`: Worktree with same name already exists for project

#### GET /projects/{project_id}/worktrees

List all worktrees for a project.

**Query Parameters:**
- `active` (optional, boolean): Filter by active status
- `type` (optional): Filter by worktree type name
- `agent_id` (optional): Filter by assigned agent

**Response (200 OK):**
```json
{
  "success": true,
  "data": [
    {
      "id": "660e8400-e29b-41d4-a716-446655440001",
      "name": "feat-user-auth",
      "type_name": "feat",
      "branch_name": "feat/user-auth",
      "agent_id": "agent-coder-1",
      "has_uncommitted_changes": true,
      "uncommitted_files_count": 3,
      "ahead_of_trunk": 2,
      "behind_trunk": 0,
      "active": true
    }
  ]
}
```

#### GET /worktrees/{worktree_id}

Retrieve worktree by UUID.

**Response:** Same structure as POST response

#### PATCH /worktrees/{worktree_id}/git-state

Update git state from external sync (called by git hooks or periodic sync).

**Request:**
```json
{
  "has_uncommitted": true,
  "uncommitted_count": 5,
  "ahead": 3,
  "behind": 0,
  "last_commit_hash": "a1b2c3d4e5f6",
  "last_commit_message": "Add authentication endpoints"
}
```

**Response (200 OK):**
```json
{
  "success": true,
  "data": {
    "worktree_id": "660e8400-e29b-41d4-a716-446655440001",
    "updated_fields": ["has_uncommitted_changes", "uncommitted_files_count", "ahead_of_trunk"]
  }
}
```

#### POST /worktrees/{worktree_id}/merge

Mark worktree as merged and deactivate.

**Request:**
```json
{
  "merged_by": "agent-merger-1",
  "merge_commit_hash": "f6e5d4c3b2a1"
}
```

**Response (200 OK):**
```json
{
  "success": true,
  "data": {
    "worktree_id": "660e8400-e29b-41d4-a716-446655440001",
    "merged_at": "2026-01-21T15:00:00Z",
    "active": false
  }
}
```

#### DELETE /worktrees/{worktree_id}

Soft delete worktree (sets active=false).

**Response (204 No Content)**

### 1.3 Query Endpoints

#### GET /inflight-work

Get all in-flight work across all projects.

**Query Parameters:**
- `project_id` (optional): Filter by project

**Response (200 OK):**
```json
{
  "success": true,
  "data": [
    {
      "worktree_id": "660e8400-e29b-41d4-a716-446655440001",
      "worktree_name": "feat-user-auth",
      "project_name": "my-awesome-app",
      "branch_name": "feat/user-auth",
      "status": "uncommitted",
      "uncommitted_count": 5,
      "ahead": 3,
      "behind": 0,
      "agent_id": "agent-coder-1",
      "last_activity": "2026-01-21T14:45:00Z"
    }
  ]
}
```

**Status Values:**
- `uncommitted`: Has uncommitted changes
- `ahead`: Ahead of trunk but no uncommitted changes
- `behind`: Behind trunk
- `diverged`: Both ahead and behind
- `clean`: Synced with trunk

#### GET /projects/{project_id}/working-path

Get deterministic working path for project or worktree.

**Query Parameters:**
- `worktree_name` (optional): Return worktree path instead of trunk

**Response (200 OK):**
```json
{
  "success": true,
  "data": {
    "path": "/home/jarad/code/my-awesome-app/trunk-main"
  }
}
```

#### GET /stats

Get overall registry statistics.

**Response (200 OK):**
```json
{
  "success": true,
  "data": {
    "total_projects": 15,
    "active_projects": 12,
    "total_worktrees": 47,
    "active_worktrees": 23,
    "in_flight_worktrees": 8,
    "total_activities": 1523,
    "activities_last_24h": 87
  }
}
```

### 1.4 Agent Activity Endpoints

#### POST /activities

Log agent activity.

**Request:**
```json
{
  "agent_id": "agent-coder-1",
  "worktree_id": "660e8400-e29b-41d4-a716-446655440001",
  "activity_type": "modified",
  "description": "Implemented JWT token validation",
  "file_path": "src/auth/jwt.rs",
  "metadata": {
    "lines_added": 45,
    "lines_removed": 12
  }
}
```

**Activity Types:**
- `created`, `modified`, `deleted`: File operations
- `committed`, `pushed`, `merged`: Git operations
- `synced`, `other`: Miscellaneous

**Response (201 Created):**
```json
{
  "success": true,
  "data": {
    "activity_id": "770e8400-e29b-41d4-a716-446655440002",
    "created_at": "2026-01-21T14:50:00Z"
  }
}
```

#### GET /agents/{agent_id}/recent-work

Get agent's recent activity.

**Query Parameters:**
- `limit` (optional, default: 50): Max activities to return

**Response (200 OK):**
```json
{
  "success": true,
  "data": [
    {
      "activity_id": "770e8400-e29b-41d4-a716-446655440002",
      "activity_type": "modified",
      "description": "Implemented JWT token validation",
      "file_path": "src/auth/jwt.rs",
      "created_at": "2026-01-21T14:50:00Z",
      "worktree_name": "feat-user-auth",
      "project_name": "my-awesome-app"
    }
  ]
}
```

## 2. Bloodbank Event Contracts

### Event Envelope Structure

All iMi events follow Bloodbank's standard envelope:

```json
{
  "event_type": "imi.project.registered",
  "timestamp": "2026-01-21T14:30:22Z",
  "source": {
    "host": "imi-service",
    "type": "component",
    "app": "imi"
  },
  "payload": { /* event-specific data */ },
  "metadata": {
    "correlation_id": "abc123",
    "version": "2.0.0"
  }
}
```

### 2.1 Published Events (iMi → Bloodbank)

#### imi.project.registered

Published when a new project is registered.

**Routing Key:** `imi.project.registered`

**Payload:**
```json
{
  "project_id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "my-awesome-app",
  "remote_origin": "git@github.com:delorenj/my-awesome-app.git",
  "trunk_path": "/home/jarad/code/my-awesome-app/trunk-main",
  "metadata": {"language": "rust"}
}
```

#### imi.worktree.created

Published when a new worktree is created.

**Routing Key:** `imi.worktree.created`

**Payload:**
```json
{
  "worktree_id": "660e8400-e29b-41d4-a716-446655440001",
  "project_id": "550e8400-e29b-41d4-a716-446655440000",
  "project_name": "my-awesome-app",
  "type": "feat",
  "name": "feat-user-auth",
  "branch_name": "feat/user-auth",
  "path": "/home/jarad/code/my-awesome-app/feat-user-auth",
  "agent_id": "agent-coder-1"
}
```

#### imi.worktree.git_state_updated

Published when worktree git state changes.

**Routing Key:** `imi.worktree.git_state_updated`

**Payload:**
```json
{
  "worktree_id": "660e8400-e29b-41d4-a716-446655440001",
  "project_name": "my-awesome-app",
  "worktree_name": "feat-user-auth",
  "has_uncommitted_changes": true,
  "uncommitted_files_count": 5,
  "ahead_of_trunk": 3,
  "behind_trunk": 0,
  "status": "uncommitted"
}
```

#### imi.worktree.merged

Published when a worktree is marked as merged.

**Routing Key:** `imi.worktree.merged`

**Payload:**
```json
{
  "worktree_id": "660e8400-e29b-41d4-a716-446655440001",
  "project_name": "my-awesome-app",
  "worktree_name": "feat-user-auth",
  "branch_name": "feat/user-auth",
  "merged_by": "agent-merger-1",
  "merge_commit_hash": "f6e5d4c3b2a1",
  "merged_at": "2026-01-21T15:00:00Z"
}
```

#### imi.activity.logged

Published when agent activity is logged.

**Routing Key:** `imi.activity.logged`

**Payload:**
```json
{
  "activity_id": "770e8400-e29b-41d4-a716-446655440002",
  "agent_id": "agent-coder-1",
  "worktree_id": "660e8400-e29b-41d4-a716-446655440001",
  "activity_type": "modified",
  "file_path": "src/auth/jwt.rs",
  "description": "Implemented JWT token validation"
}
```

### 2.2 Consumed Events (Bloodbank → iMi)

#### imi.project.register (Command)

Command to register a new project.

**Routing Key:** `imi.project.register`

**Queue:** `imi_commands_queue`

**Payload:** Same as POST /projects/register request

**Response Event:** `imi.project.registered` (success) or `imi.project.registration_failed` (error)

#### imi.worktree.create (Command)

Command to create a new worktree.

**Routing Key:** `imi.worktree.create`

**Queue:** `imi_commands_queue`

**Payload:** Same as POST /projects/{project_id}/worktrees request

**Response Event:** `imi.worktree.created` (success) or `imi.worktree.creation_failed` (error)

#### git.sync_completed

Consumes git sync events to update worktree state.

**Routing Key:** `git.sync_completed`

**Queue:** `imi_git_sync_queue`

**Payload:**
```json
{
  "worktree_path": "/home/jarad/code/my-awesome-app/feat-user-auth",
  "has_uncommitted": true,
  "uncommitted_count": 5,
  "ahead": 3,
  "behind": 0,
  "last_commit_hash": "a1b2c3d4e5f6"
}
```

**Action:** Calls PATCH /worktrees/{worktree_id}/git-state

## 3. MCP Tool Contracts

### Tool Registration

iMi exposes MCP tools via FastMCP server on port 8081.

**Server Configuration:**
```json
{
  "mcpServers": {
    "imi": {
      "command": "uv",
      "args": ["run", "imi", "mcp"],
      "env": {
        "IMI_DATABASE_URL": "postgresql://localhost:5432/imi_registry"
      }
    }
  }
}
```

### 3.1 create_project

Create a new project.

**Schema:**
```json
{
  "name": "create_project",
  "description": "Register a new project in iMi Project Registry",
  "inputSchema": {
    "type": "object",
    "properties": {
      "name": {"type": "string"},
      "remote_origin": {"type": "string"},
      "default_branch": {"type": "string", "default": "main"},
      "trunk_path": {"type": "string"},
      "metadata": {"type": "object"}
    },
    "required": ["name", "remote_origin"]
  }
}
```

**Returns:** Same as POST /projects/register

### 3.2 create_worktree

Create a new worktree.

**Schema:**
```json
{
  "name": "create_worktree",
  "description": "Create a new git worktree for a project",
  "inputSchema": {
    "type": "object",
    "properties": {
      "project_id": {"type": "string", "format": "uuid"},
      "type": {"type": "string"},
      "name": {"type": "string"},
      "branch_name": {"type": "string"},
      "path": {"type": "string"},
      "agent_id": {"type": "string"},
      "metadata": {"type": "object"}
    },
    "required": ["project_id", "type", "name", "branch_name", "path"]
  }
}
```

**Returns:** Same as POST /projects/{project_id}/worktrees

### 3.3 get_inflight_work

Query in-flight work.

**Schema:**
```json
{
  "name": "get_inflight_work",
  "description": "Get all worktrees with uncommitted or unmerged work",
  "inputSchema": {
    "type": "object",
    "properties": {
      "project_id": {"type": "string", "format": "uuid"}
    }
  }
}
```

**Returns:** Same as GET /inflight-work

### 3.4 update_git_state

Update worktree git state.

**Schema:**
```json
{
  "name": "update_git_state",
  "description": "Update git state for a worktree",
  "inputSchema": {
    "type": "object",
    "properties": {
      "worktree_id": {"type": "string", "format": "uuid"},
      "has_uncommitted": {"type": "boolean"},
      "uncommitted_count": {"type": "integer"},
      "ahead": {"type": "integer"},
      "behind": {"type": "integer"},
      "last_commit_hash": {"type": "string"},
      "last_commit_message": {"type": "string"}
    },
    "required": ["worktree_id"]
  }
}
```

**Returns:** Same as PATCH /worktrees/{worktree_id}/git-state

### 3.5 log_activity

Log agent activity.

**Schema:**
```json
{
  "name": "log_activity",
  "description": "Log agent activity for a worktree",
  "inputSchema": {
    "type": "object",
    "properties": {
      "agent_id": {"type": "string"},
      "worktree_id": {"type": "string", "format": "uuid"},
      "activity_type": {"type": "string", "enum": ["created", "modified", "deleted", "committed", "pushed", "merged", "synced", "other"]},
      "description": {"type": "string"},
      "file_path": {"type": "string"},
      "metadata": {"type": "object"}
    },
    "required": ["agent_id", "worktree_id", "activity_type", "description"]
  }
}
```

**Returns:** Same as POST /activities

## 4. PostgreSQL Function Contracts

For trusted components with direct database access.

### 4.1 register_project()

```sql
SELECT register_project(
    p_name TEXT,
    p_remote_origin TEXT,
    p_default_branch TEXT DEFAULT 'main',
    p_trunk_path TEXT DEFAULT NULL,
    p_metadata JSONB DEFAULT '{}'::jsonb
) RETURNS UUID;
```

### 4.2 register_worktree()

```sql
SELECT register_worktree(
    p_project_id UUID,
    p_type_name TEXT,
    p_name TEXT,
    p_branch_name TEXT,
    p_path TEXT,
    p_agent_id TEXT DEFAULT NULL,
    p_metadata JSONB DEFAULT '{}'::jsonb
) RETURNS UUID;
```

### 4.3 update_worktree_git_state()

```sql
SELECT update_worktree_git_state(
    p_worktree_id UUID,
    p_has_uncommitted BOOLEAN DEFAULT NULL,
    p_uncommitted_count INTEGER DEFAULT NULL,
    p_ahead INTEGER DEFAULT NULL,
    p_behind INTEGER DEFAULT NULL,
    p_last_commit_hash TEXT DEFAULT NULL,
    p_last_commit_message TEXT DEFAULT NULL
) RETURNS VOID;
```

### 4.4 get_inflight_work()

```sql
SELECT * FROM get_inflight_work(
    p_project_id UUID DEFAULT NULL
) RETURNS TABLE (
    worktree_id UUID,
    worktree_name TEXT,
    branch_name TEXT,
    status TEXT,
    uncommitted_count INTEGER,
    ahead INTEGER,
    behind INTEGER,
    agent_id TEXT,
    last_activity TIMESTAMPTZ
);
```

### 4.5 log_activity()

```sql
SELECT log_activity(
    p_agent_id TEXT,
    p_worktree_id UUID,
    p_activity_type TEXT,
    p_description TEXT,
    p_file_path TEXT DEFAULT NULL,
    p_metadata JSONB DEFAULT '{}'::jsonb
) RETURNS UUID;
```

## 5. Integration Patterns

### 5.1 Flume Integration

**Use Case:** Task-worktree lifecycle management

**Pattern:**
1. Flume creates task via its own API
2. Flume calls `POST /projects/{project_id}/worktrees` with task metadata
3. iMi publishes `imi.worktree.created` event
4. Flume consumes event and links task to worktree_id

### 5.2 Jelmore Integration

**Use Case:** Session-aware worktree context

**Pattern:**
1. Jelmore queries `GET /inflight-work` to find work
2. Jelmore spawns agent in specific worktree
3. Agent calls `POST /activities` to log work
4. iMi publishes `imi.activity.logged` event
5. Jelmore consumes event for observability

### 5.3 Bloodbank Integration

**Use Case:** Event-driven project/worktree creation

**Pattern:**
1. External trigger publishes `imi.project.register` command
2. iMi FastStream consumer receives command
3. iMi calls `register_project()` function
4. iMi publishes `imi.project.registered` event
5. Downstream consumers react (create directories, clone repo, etc.)

### 5.4 Direct Database Access

**Use Case:** High-performance reads from analytics

**Pattern:**
1. Component connects to PostgreSQL with read-only credentials
2. Component queries views (`v_projects_summary`, `v_inflight_work`)
3. Component does NOT call mutation functions
4. Write operations MUST go through REST API or events

## 6. Error Handling

### HTTP Error Responses

All error responses follow this structure:

```json
{
  "success": false,
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Invalid remote origin format",
    "details": {
      "field": "remote_origin",
      "constraint": "Must match pattern: git@github.com:user/repo.git"
    }
  }
}
```

**Error Codes:**
- `VALIDATION_ERROR`: Input validation failed
- `NOT_FOUND`: Resource doesn't exist
- `CONFLICT`: Resource already exists or constraint violation
- `INTERNAL_ERROR`: Database or system error

### Event Error Handling

Failed command events publish error events:

```json
{
  "event_type": "imi.project.registration_failed",
  "timestamp": "2026-01-21T14:30:22Z",
  "payload": {
    "request": { /* original request */ },
    "error": {
      "code": "VALIDATION_ERROR",
      "message": "Invalid remote origin format"
    }
  }
}
```

## 7. Versioning and Compatibility

**API Version:** v1 (current)

**Compatibility Promise:**
- Additive changes (new fields, new endpoints) are non-breaking
- Field removals or type changes require major version bump
- Event schema changes are versioned in metadata.version field
- PostgreSQL functions use named parameters for backward compatibility

**Deprecation Policy:**
- Deprecated endpoints marked in docs 3 months before removal
- Old versions supported for 6 months after new version release
- Breaking changes communicated via Bloodbank `imi.schema.deprecated` events

## 8. Security Considerations

**Authentication:**
- REST API: JWT tokens with scoped permissions
- Bloodbank: Queue-level authentication via RabbitMQ ACLs
- MCP Tools: Trusted localhost only (no network exposure)
- PostgreSQL: Role-based access control (RBAC)

**Authorization Scopes:**
- `imi:read`: Read-only access to projects/worktrees
- `imi:write`: Create/update projects/worktrees
- `imi:admin`: Delete operations and maintenance functions

**Rate Limiting:**
- REST API: 100 req/min per API key
- Event publishing: 1000 events/min per component
- Database functions: No rate limit (trusted internal use)

---

**Maintained by**: iMi Development Team
**Change Log**: `/home/delorenj/code/iMi/trunk-main/CHANGELOG.md`
**Implementation Status**: See `/home/delorenj/code/iMi/trunk-main/docs/implementation-status.md`
