# iMi Project Registry - PostgreSQL Schema

**Version**: 2.0.0
**Purpose**: Authoritative project registry for the 33GOD agentic pipeline
**Database**: PostgreSQL 14+

## Overview

This schema provides the foundation for iMi as the **Project Registry** component of 33GOD, tracking all projects, worktrees, and agent activities across the pipeline.

## Design Principles

### 1. **1:1 Project Identity**
- Each project has a unique UUID (`project_id`)
- Strict 1:1 mapping: `project_id` ↔ `remote_origin` (GitHub URL)
- Enforced via unique constraint and exclusion constraint on active projects
- Prevents duplicate project registrations even across distributed 33GOD hosts

### 2. **Proper Normalization**
- Foreign keys use UUIDs, not strings
- `worktrees.project_id` → `projects.id` (not repo name)
- `worktrees.type_id` → `worktree_types.id` (not type name)
- No data duplication beyond intentional denormalization for performance

### 3. **In-Flight Work Tracking**
- Worktrees track git state: uncommitted changes, ahead/behind trunk
- `has_uncommitted_changes`, `ahead_of_trunk`, `behind_trunk` fields
- Synced periodically from git operations
- Enables agents to find work in progress

### 4. **Deterministic Paths**
- `projects.trunk_path` is the canonical base path
- Helper function `get_project_working_path()` generates consistent paths
- All 33GOD components use this for path resolution

### 5. **Audit Trail**
- `agent_activities` logs all agent actions
- Immutable append-only log
- Retention policy via `cleanup_old_activities()`

### 6. **Extensibility**
- JSONB `metadata` columns on projects, worktrees, activities
- Add custom fields without schema migrations
- Full GIN indexing for metadata queries

## Core Tables

### `projects`
**Purpose**: Single source of truth for all 33GOD projects

**Key Fields**:
- `id` (UUID): Unique project identifier for 33GOD
- `remote_origin` (TEXT): GitHub remote URL, enforced unique
- `trunk_path` (TEXT): Canonical base path (e.g., `~/code/my-project/trunk-main`)
- `metadata` (JSONB): Extensible project metadata

**Constraints**:
- `remote_origin` must match pattern: `git@github.com:username/repo.git`
- Only one active project per `remote_origin`
- Exclusion constraint prevents race conditions

### `worktrees`
**Purpose**: Track all worktrees across all projects

**Key Fields**:
- `project_id` (UUID FK): References `projects.id`
- `type_id` (INTEGER FK): References `worktree_types.id`
- `has_uncommitted_changes` (BOOLEAN): Git state tracking
- `ahead_of_trunk`, `behind_trunk` (INTEGER): Branch divergence
- `merged_at` (TIMESTAMPTZ): When worktree was merged (null = active)
- `agent_id` (TEXT): Which agent owns this worktree

**Constraints**:
- Unique `(project_id, name)` per project
- Unique active path (prevents path collisions)
- Check constraints on ahead/behind counts

### `worktree_types`
**Purpose**: Classification of worktrees (feat, fix, aiops, devops, review, trunk)

**Key Fields**:
- `name` (TEXT): Type name (e.g., "feat")
- `branch_prefix` (TEXT): Git branch prefix (e.g., "feat/")
- `worktree_prefix` (TEXT): Directory prefix (e.g., "feat-")
- `is_builtin` (BOOLEAN): Protected from deletion

**Built-in Types**:
- `feat`: Feature development
- `fix`: Bug fixes
- `aiops`: AI operations (agents, rules, workflows)
- `devops`: DevOps tasks (CI, deploys)
- `review`: Pull request reviews
- `trunk`: Main branch worktree

### `agent_activities`
**Purpose**: Audit log of all agent actions

**Key Fields**:
- `agent_id` (TEXT): Agent identifier
- `worktree_id` (UUID FK): References `worktrees.id`
- `activity_type` (TEXT): Action type (created, modified, committed, etc.)
- `file_path` (TEXT): Optional file reference
- `metadata` (JSONB): Additional context

**Activity Types**:
- `created`, `modified`, `deleted`: File operations
- `committed`, `pushed`, `merged`: Git operations
- `synced`, `other`: Miscellaneous

## Indexes

### Performance Indexes
- `idx_projects_active`: Active projects only
- `idx_worktrees_project_id`: Worktree lookups by project
- `idx_worktrees_uncommitted`: Find uncommitted work
- `idx_worktrees_unmerged`: Find unmerged worktrees
- `idx_agent_activities_worktree_id`: Activity lookups

### Full-Text Search
- `idx_projects_remote_origin_trgm`: Fuzzy search on remote URLs (requires `pg_trgm`)

### Metadata Indexes
- GIN indexes on all JSONB columns for fast metadata queries

## Views

### `v_projects_summary`
Aggregated project statistics with worktree counts

### `v_worktrees_detail`
Denormalized worktree view with full context (project, type, status)

### `v_inflight_work`
All worktrees with uncommitted or unmerged work

### `v_recent_agent_activity`
Recent agent activities with context

## Functions

### Project Management
- `register_project()`: Register new project with duplicate prevention
- `get_project_by_origin()`: Lookup by GitHub URL
- `deactivate_project()`: Soft delete project and worktrees

### Worktree Management
- `register_worktree()`: Register new worktree
- `update_worktree_git_state()`: Sync git state from external source
- `mark_worktree_merged()`: Mark worktree as merged and deactivate
- `get_worktree_by_path()`: Lookup by filesystem path

### Agent Operations
- `log_activity()`: Log agent action
- `get_agent_recent_work()`: Get agent's recent activity

### Query Helpers
- `get_inflight_work()`: Find all in-flight work
- `get_project_working_path()`: Generate deterministic paths
- `search_projects()`: Fuzzy search projects

### Maintenance
- `cleanup_old_activities()`: Prune old activity logs
- `prune_inactive_worktrees()`: Remove old inactive worktrees
- `maintenance_vacuum()`: Vacuum and analyze all tables
- `get_registry_stats()`: Overall statistics

## Triggers

### Auto-Update Timestamps
- `projects_updated_at`: Updates `updated_at` on modification
- `worktrees_updated_at`: Updates `updated_at` on modification

### State Synchronization
- `worktrees_sync_uncommitted`: Syncs `has_uncommitted_changes` with `uncommitted_files_count`

## Migration Guide

### Initial Setup

```bash
# Create database
createdb imi_registry

# Run migrations
psql imi_registry < migrations/001_create_schema.sql
psql imi_registry < migrations/002_functions_and_helpers.sql
```

### Rollback

```bash
psql imi_registry < migrations/999_rollback.sql
```

### From SQLite

See `docs/migration_sqlite_to_postgres.md` for detailed migration procedure.

## Usage Examples

### Register a Project

```sql
SELECT register_project(
    'my-project',
    'git@github.com:delorenj/my-project.git',
    'main',
    '/home/jarad/code/my-project/trunk-main'
);
```

### Register a Worktree

```sql
SELECT register_worktree(
    'project-uuid-here',
    'feat',
    'feat-user-auth',
    'feat/user-auth',
    '/home/jarad/code/my-project/feat-user-auth',
    'agent-1'
);
```

### Update Git State

```sql
SELECT update_worktree_git_state(
    'worktree-uuid-here',
    p_has_uncommitted := TRUE,
    p_uncommitted_count := 3,
    p_ahead := 2,
    p_behind := 0
);
```

### Find In-Flight Work

```sql
SELECT * FROM get_inflight_work();
```

### Log Agent Activity

```sql
SELECT log_activity(
    'agent-1',
    'worktree-uuid-here',
    'modified',
    'Updated authentication logic',
    'src/auth.rs'
);
```

## Performance Considerations

### Connection Pooling
Use PgBouncer or similar for connection pooling:
- Transaction pooling mode recommended
- Pool size: 10-20 connections per application instance

### Query Optimization
- Use prepared statements for frequent queries
- Leverage partial indexes on active records only
- Use views for complex joins

### Maintenance Schedule
- `cleanup_old_activities(90)`: Weekly (retain 90 days)
- `prune_inactive_worktrees(30)`: Monthly (prune after 30 days inactive)
- `maintenance_vacuum()`: Monthly

### Monitoring Queries

```sql
-- Check database size
SELECT pg_size_pretty(pg_database_size('imi_registry'));

-- Check table sizes
SELECT
    schemaname,
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename))
FROM pg_tables
WHERE schemaname = 'public'
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;

-- Check index usage
SELECT
    schemaname,
    tablename,
    indexname,
    idx_scan,
    idx_tup_read,
    idx_tup_fetch
FROM pg_stat_user_indexes
ORDER BY idx_scan DESC;
```

## Future Enhancements

### Distributed Deployment
- Add `host_id` to projects for multi-host tracking
- Implement consensus mechanism for project registration
- Use PostgreSQL logical replication for cross-host sync

### Advanced Features
- Materialized views for expensive aggregations
- Partition `agent_activities` by created_at for better performance
- Add full-text search on descriptions and commit messages
- Implement change data capture (CDC) for event streaming

### Monitoring & Alerting
- Track schema version in metadata table
- Add health check functions for monitoring
- Implement query performance tracking

## Schema Version History

- **2.0.0** (2026-01-21): Initial PostgreSQL schema
  - Normalized design with proper FKs
  - In-flight work tracking
  - JSONB metadata support
  - Comprehensive indexes and functions

## References

- [PostgreSQL Documentation](https://www.postgresql.org/docs/14/)
- [33GOD Architecture](../docs/architecture-33god.md)
- [iMi Design Decisions](../docs/design_decisions.md)
