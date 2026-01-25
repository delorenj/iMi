-- ============================================================================
-- iMi Project Registry - Rollback
-- Version: 2.0.0
-- Purpose: Complete rollback of schema to clean slate
-- ============================================================================

-- Drop views
DROP VIEW IF EXISTS v_recent_agent_activity;
DROP VIEW IF EXISTS v_inflight_work;
DROP VIEW IF EXISTS v_worktrees_detail;
DROP VIEW IF EXISTS v_projects_summary;

-- Drop functions
DROP FUNCTION IF EXISTS get_registry_stats();
DROP FUNCTION IF EXISTS maintenance_vacuum();
DROP FUNCTION IF EXISTS prune_inactive_worktrees(INTEGER);
DROP FUNCTION IF EXISTS cleanup_old_activities(INTEGER);
DROP FUNCTION IF EXISTS search_projects(TEXT);
DROP FUNCTION IF EXISTS get_project_working_path(UUID, TEXT);
DROP FUNCTION IF EXISTS get_inflight_work(UUID);
DROP FUNCTION IF EXISTS get_agent_recent_work(TEXT, INTEGER);
DROP FUNCTION IF EXISTS log_activity(TEXT, UUID, TEXT, TEXT, TEXT, JSONB);
DROP FUNCTION IF EXISTS get_worktree_by_path(TEXT);
DROP FUNCTION IF EXISTS mark_worktree_merged(UUID, TEXT, TEXT);
DROP FUNCTION IF EXISTS update_worktree_git_state(UUID, BOOLEAN, INTEGER, INTEGER, INTEGER, TEXT, TEXT);
DROP FUNCTION IF EXISTS register_worktree(UUID, TEXT, TEXT, TEXT, TEXT, TEXT, JSONB);
DROP FUNCTION IF EXISTS deactivate_project(UUID);
DROP FUNCTION IF EXISTS get_project_by_origin(TEXT);
DROP FUNCTION IF EXISTS register_project(TEXT, TEXT, TEXT, TEXT, JSONB);

-- Drop triggers
DROP TRIGGER IF EXISTS worktrees_sync_uncommitted ON worktrees;
DROP TRIGGER IF EXISTS worktrees_updated_at ON worktrees;
DROP TRIGGER IF EXISTS projects_updated_at ON projects;

-- Drop trigger functions
DROP FUNCTION IF EXISTS sync_uncommitted_changes();
DROP FUNCTION IF EXISTS update_updated_at_column();

-- Drop tables (in reverse dependency order)
DROP TABLE IF EXISTS agent_activities;
DROP TABLE IF EXISTS worktrees;
DROP TABLE IF EXISTS worktree_types;
DROP TABLE IF EXISTS projects;

-- Drop extensions (only if not used by other schemas)
-- DROP EXTENSION IF EXISTS pgcrypto;
