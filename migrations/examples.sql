-- ============================================================================
-- iMi Project Registry - Usage Examples
-- Version: 2.0.0
-- Purpose: Practical examples of common operations
-- ============================================================================

-- ============================================================================
-- Example 1: Register a New Project
-- ============================================================================

-- Register a project (returns project_id)
DO $$
DECLARE
    v_project_id UUID;
BEGIN
    v_project_id := register_project(
        p_name := 'my-awesome-app',
        p_remote_origin := 'git@github.com:delorenj/my-awesome-app.git',
        p_default_branch := 'main',
        p_trunk_path := '/home/jarad/code/my-awesome-app/trunk-main',
        p_metadata := '{"language": "rust", "framework": "axum"}'::jsonb
    );

    RAISE NOTICE 'Registered project: %', v_project_id;
END $$;

-- ============================================================================
-- Example 2: Lookup Existing Project
-- ============================================================================

-- Get project by remote origin
SELECT * FROM get_project_by_origin('git@github.com:delorenj/my-awesome-app.git');

-- Search projects by name
SELECT * FROM search_projects('awesome');

-- ============================================================================
-- Example 3: Create Worktrees
-- ============================================================================

-- Create a feature worktree
DO $$
DECLARE
    v_project_id UUID;
    v_worktree_id UUID;
BEGIN
    -- Get project ID first
    SELECT id INTO v_project_id
    FROM projects
    WHERE name = 'my-awesome-app'
      AND active = TRUE;

    -- Register feature worktree
    v_worktree_id := register_worktree(
        p_project_id := v_project_id,
        p_type_name := 'feat',
        p_name := 'feat-user-authentication',
        p_branch_name := 'feat/user-authentication',
        p_path := '/home/jarad/code/my-awesome-app/feat-user-authentication',
        p_agent_id := 'agent-coder-1',
        p_metadata := '{"story_id": "PROJ-123", "priority": "high"}'::jsonb
    );

    RAISE NOTICE 'Created worktree: %', v_worktree_id;
END $$;

-- ============================================================================
-- Example 4: Update Git State (from external sync)
-- ============================================================================

-- Update worktree with uncommitted changes
DO $$
DECLARE
    v_worktree_id UUID;
BEGIN
    -- Get worktree ID
    SELECT id INTO v_worktree_id
    FROM worktrees
    WHERE name = 'feat-user-authentication'
      AND active = TRUE
    LIMIT 1;

    -- Update git state
    PERFORM update_worktree_git_state(
        p_worktree_id := v_worktree_id,
        p_has_uncommitted := TRUE,
        p_uncommitted_count := 5,
        p_ahead := 3,
        p_behind := 0,
        p_last_commit_hash := 'a1b2c3d4e5f6',
        p_last_commit_message := 'Add authentication endpoints'
    );

    RAISE NOTICE 'Updated git state for worktree: %', v_worktree_id;
END $$;

-- ============================================================================
-- Example 5: Log Agent Activity
-- ============================================================================

-- Log file modification
DO $$
DECLARE
    v_worktree_id UUID;
    v_activity_id UUID;
BEGIN
    -- Get worktree ID
    SELECT id INTO v_worktree_id
    FROM worktrees
    WHERE name = 'feat-user-authentication'
      AND active = TRUE
    LIMIT 1;

    -- Log activity
    v_activity_id := log_activity(
        p_agent_id := 'agent-coder-1',
        p_worktree_id := v_worktree_id,
        p_activity_type := 'modified',
        p_description := 'Implemented JWT token validation',
        p_file_path := 'src/auth/jwt.rs',
        p_metadata := '{"lines_added": 45, "lines_removed": 12}'::jsonb
    );

    RAISE NOTICE 'Logged activity: %', v_activity_id;
END $$;

-- ============================================================================
-- Example 6: Query In-Flight Work
-- ============================================================================

-- Get all in-flight work across all projects
SELECT
    worktree_name,
    branch_name,
    status,
    uncommitted_count,
    ahead,
    behind,
    agent_id
FROM get_inflight_work()
ORDER BY last_activity DESC;

-- Get in-flight work for specific project
SELECT *
FROM get_inflight_work(
    (SELECT id FROM projects WHERE name = 'my-awesome-app' LIMIT 1)
);

-- ============================================================================
-- Example 7: Get Deterministic Paths
-- ============================================================================

-- Get trunk path
SELECT get_project_working_path(
    (SELECT id FROM projects WHERE name = 'my-awesome-app' LIMIT 1),
    NULL
);

-- Get feature worktree path
SELECT get_project_working_path(
    (SELECT id FROM projects WHERE name = 'my-awesome-app' LIMIT 1),
    'feat-user-authentication'
);

-- ============================================================================
-- Example 8: Mark Worktree as Merged
-- ============================================================================

DO $$
DECLARE
    v_worktree_id UUID;
BEGIN
    -- Get worktree ID
    SELECT id INTO v_worktree_id
    FROM worktrees
    WHERE name = 'feat-user-authentication'
      AND active = TRUE
    LIMIT 1;

    -- Mark as merged (deactivates worktree)
    PERFORM mark_worktree_merged(
        p_worktree_id := v_worktree_id,
        p_merged_by := 'agent-merger-1',
        p_merge_commit_hash := 'f6e5d4c3b2a1'
    );

    RAISE NOTICE 'Marked worktree as merged: %', v_worktree_id;
END $$;

-- ============================================================================
-- Example 9: Query Agent Activity
-- ============================================================================

-- Get recent work for specific agent
SELECT * FROM get_agent_recent_work('agent-coder-1', 20);

-- Get all recent activity
SELECT * FROM v_recent_agent_activity
ORDER BY created_at DESC
LIMIT 50;

-- ============================================================================
-- Example 10: Find Worktree by Path
-- ============================================================================

-- Lookup worktree from filesystem context
SELECT * FROM get_worktree_by_path('/home/jarad/code/my-awesome-app/feat-user-authentication');

-- ============================================================================
-- Example 11: Project Summary Stats
-- ============================================================================

-- Get summary for all projects
SELECT * FROM v_projects_summary
ORDER BY last_worktree_activity DESC NULLS LAST;

-- Get detailed worktree info
SELECT * FROM v_worktrees_detail
WHERE active = TRUE
ORDER BY updated_at DESC;

-- ============================================================================
-- Example 12: Registry Statistics
-- ============================================================================

-- Get overall stats
SELECT * FROM get_registry_stats();

-- Custom aggregations
SELECT
    wt.name AS worktree_type,
    COUNT(*) AS total_worktrees,
    COUNT(*) FILTER (WHERE w.active = TRUE) AS active_worktrees,
    COUNT(*) FILTER (WHERE w.has_uncommitted_changes = TRUE) AS uncommitted_worktrees,
    COUNT(*) FILTER (WHERE w.merged_at IS NULL) AS unmerged_worktrees
FROM worktrees w
JOIN worktree_types wt ON wt.id = w.type_id
GROUP BY wt.name
ORDER BY active_worktrees DESC;

-- ============================================================================
-- Example 13: Maintenance Operations
-- ============================================================================

-- Clean up old activities (older than 90 days)
SELECT cleanup_old_activities(90);

-- Prune inactive worktrees (inactive for 30+ days)
SELECT prune_inactive_worktrees(30);

-- Run maintenance vacuum
SELECT maintenance_vacuum();

-- ============================================================================
-- Example 14: Complex Queries
-- ============================================================================

-- Find stale worktrees (no activity in 7+ days, uncommitted changes)
SELECT
    p.name AS project_name,
    w.name AS worktree_name,
    w.agent_id,
    w.uncommitted_files_count,
    w.updated_at AS last_activity
FROM worktrees w
JOIN projects p ON p.id = w.project_id
WHERE w.active = TRUE
  AND w.has_uncommitted_changes = TRUE
  AND w.updated_at < NOW() - INTERVAL '7 days'
ORDER BY w.updated_at ASC;

-- Find agents with most activity in last 24 hours
SELECT
    agent_id,
    COUNT(*) AS activity_count,
    COUNT(DISTINCT worktree_id) AS worktrees_touched,
    MAX(created_at) AS last_activity
FROM agent_activities
WHERE created_at > NOW() - INTERVAL '24 hours'
GROUP BY agent_id
ORDER BY activity_count DESC;

-- Find diverged branches (both ahead and behind trunk)
SELECT
    p.name AS project_name,
    w.name AS worktree_name,
    w.ahead_of_trunk,
    w.behind_trunk,
    w.last_sync_at
FROM worktrees w
JOIN projects p ON p.id = w.project_id
WHERE w.active = TRUE
  AND w.ahead_of_trunk > 0
  AND w.behind_trunk > 0
ORDER BY (w.ahead_of_trunk + w.behind_trunk) DESC;

-- ============================================================================
-- Example 15: Metadata Queries (JSONB)
-- ============================================================================

-- Find projects by language
SELECT name, remote_origin, metadata->'language' AS language
FROM projects
WHERE metadata @> '{"language": "rust"}'::jsonb
  AND active = TRUE;

-- Find high-priority worktrees
SELECT
    p.name AS project_name,
    w.name AS worktree_name,
    w.metadata->'story_id' AS story_id,
    w.metadata->'priority' AS priority
FROM worktrees w
JOIN projects p ON p.id = w.project_id
WHERE w.metadata @> '{"priority": "high"}'::jsonb
  AND w.active = TRUE;

-- Update metadata
UPDATE worktrees
SET metadata = metadata || '{"reviewed": true, "reviewer": "agent-reviewer-1"}'::jsonb
WHERE name = 'feat-user-authentication';

-- ============================================================================
-- Example 16: Transaction Example (Atomicity)
-- ============================================================================

-- Create project and worktree atomically
DO $$
DECLARE
    v_project_id UUID;
    v_worktree_id UUID;
BEGIN
    -- Start transaction (implicit in DO block)

    -- Register project
    v_project_id := register_project(
        'new-project',
        'git@github.com:delorenj/new-project.git'
    );

    -- Create trunk worktree
    v_worktree_id := register_worktree(
        v_project_id,
        'trunk',
        'trunk-main',
        'main',
        '/home/jarad/code/new-project/trunk-main'
    );

    RAISE NOTICE 'Created project % and trunk worktree %', v_project_id, v_worktree_id;

    -- Commit happens automatically if no exception
EXCEPTION
    WHEN OTHERS THEN
        -- Rollback happens automatically on exception
        RAISE NOTICE 'Failed to create project: %', SQLERRM;
        RAISE;
END $$;
