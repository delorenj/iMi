-- ============================================================================
-- iMi Project Registry - Helper Functions
-- Version: 2.0.0
-- Purpose: Database functions for common operations and utilities
-- ============================================================================

-- ============================================================================
-- Project Management Functions
-- ============================================================================

-- Register a new project (ensures uniqueness)
CREATE OR REPLACE FUNCTION register_project(
    p_name TEXT,
    p_remote_origin TEXT,
    p_default_branch TEXT DEFAULT 'main',
    p_trunk_path TEXT DEFAULT NULL,
    p_metadata JSONB DEFAULT '{}'::jsonb
) RETURNS UUID AS $$
DECLARE
    v_project_id UUID;
    v_trunk_path TEXT;
BEGIN
    -- Validate remote origin format
    IF p_remote_origin !~ '^git@github\.com:[^/]+/.+\.git$' THEN
        RAISE EXCEPTION 'Invalid remote origin format: %', p_remote_origin;
    END IF;

    -- Generate trunk path if not provided
    v_trunk_path := COALESCE(
        p_trunk_path,
        format('%s/code/%s/trunk-%s',
            (SELECT COALESCE(NULLIF(current_setting('imi.home_dir', TRUE), ''), '/home/' || current_user)),
            p_name,
            p_default_branch
        )
    );

    -- Insert or return existing project
    INSERT INTO projects (name, remote_origin, default_branch, trunk_path, metadata)
    VALUES (p_name, p_remote_origin, p_default_branch, v_trunk_path, p_metadata)
    ON CONFLICT (remote_origin) DO UPDATE
        SET updated_at = NOW(),
            active = TRUE
    RETURNING id INTO v_project_id;

    RETURN v_project_id;
END;
$$ LANGUAGE plpgsql;

-- Get project by remote origin (most common lookup)
CREATE OR REPLACE FUNCTION get_project_by_origin(
    p_remote_origin TEXT
) RETURNS TABLE (
    id UUID,
    name TEXT,
    remote_origin TEXT,
    default_branch TEXT,
    trunk_path TEXT,
    active BOOLEAN,
    created_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ
) AS $$
BEGIN
    RETURN QUERY
    SELECT p.id, p.name, p.remote_origin, p.default_branch, p.trunk_path,
           p.active, p.created_at, p.updated_at
    FROM projects p
    WHERE p.remote_origin = p_remote_origin
      AND p.active = TRUE;
END;
$$ LANGUAGE plpgsql;

-- Deactivate project (soft delete)
CREATE OR REPLACE FUNCTION deactivate_project(
    p_project_id UUID
) RETURNS VOID AS $$
BEGIN
    -- Deactivate all worktrees first
    UPDATE worktrees
    SET active = FALSE,
        updated_at = NOW()
    WHERE project_id = p_project_id;

    -- Deactivate project
    UPDATE projects
    SET active = FALSE,
        updated_at = NOW()
    WHERE id = p_project_id;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- Worktree Management Functions
-- ============================================================================

-- Register a new worktree
CREATE OR REPLACE FUNCTION register_worktree(
    p_project_id UUID,
    p_type_name TEXT,
    p_name TEXT,
    p_branch_name TEXT,
    p_path TEXT,
    p_agent_id TEXT DEFAULT NULL,
    p_metadata JSONB DEFAULT '{}'::jsonb
) RETURNS UUID AS $$
DECLARE
    v_worktree_id UUID;
    v_type_id INTEGER;
BEGIN
    -- Get type ID
    SELECT id INTO v_type_id
    FROM worktree_types
    WHERE name = p_type_name;

    IF v_type_id IS NULL THEN
        RAISE EXCEPTION 'Worktree type not found: %', p_type_name;
    END IF;

    -- Insert worktree
    INSERT INTO worktrees (
        project_id, type_id, name, branch_name, path, agent_id, metadata
    ) VALUES (
        p_project_id, v_type_id, p_name, p_branch_name, p_path, p_agent_id, p_metadata
    )
    ON CONFLICT (project_id, name) DO UPDATE
        SET active = TRUE,
            updated_at = NOW(),
            path = EXCLUDED.path,
            branch_name = EXCLUDED.branch_name
    RETURNING id INTO v_worktree_id;

    RETURN v_worktree_id;
END;
$$ LANGUAGE plpgsql;

-- Update worktree git state
CREATE OR REPLACE FUNCTION update_worktree_git_state(
    p_worktree_id UUID,
    p_has_uncommitted BOOLEAN DEFAULT NULL,
    p_uncommitted_count INTEGER DEFAULT NULL,
    p_ahead INTEGER DEFAULT NULL,
    p_behind INTEGER DEFAULT NULL,
    p_last_commit_hash TEXT DEFAULT NULL,
    p_last_commit_message TEXT DEFAULT NULL
) RETURNS VOID AS $$
BEGIN
    UPDATE worktrees
    SET
        has_uncommitted_changes = COALESCE(p_has_uncommitted, has_uncommitted_changes),
        uncommitted_files_count = COALESCE(p_uncommitted_count, uncommitted_files_count),
        ahead_of_trunk = COALESCE(p_ahead, ahead_of_trunk),
        behind_trunk = COALESCE(p_behind, behind_trunk),
        last_commit_hash = COALESCE(p_last_commit_hash, last_commit_hash),
        last_commit_message = COALESCE(p_last_commit_message, last_commit_message),
        last_sync_at = NOW(),
        updated_at = NOW()
    WHERE id = p_worktree_id;
END;
$$ LANGUAGE plpgsql;

-- Mark worktree as merged
CREATE OR REPLACE FUNCTION mark_worktree_merged(
    p_worktree_id UUID,
    p_merged_by TEXT,
    p_merge_commit_hash TEXT
) RETURNS VOID AS $$
BEGIN
    UPDATE worktrees
    SET
        merged_at = NOW(),
        merged_by = p_merged_by,
        merge_commit_hash = p_merge_commit_hash,
        active = FALSE,
        updated_at = NOW()
    WHERE id = p_worktree_id;
END;
$$ LANGUAGE plpgsql;

-- Get worktree by path (useful for directory-based context)
CREATE OR REPLACE FUNCTION get_worktree_by_path(
    p_path TEXT
) RETURNS TABLE (
    id UUID,
    project_id UUID,
    project_name TEXT,
    worktree_name TEXT,
    branch_name TEXT,
    type_name TEXT,
    agent_id TEXT,
    active BOOLEAN
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        w.id,
        w.project_id,
        p.name,
        w.name,
        w.branch_name,
        wt.name,
        w.agent_id,
        w.active
    FROM worktrees w
    JOIN projects p ON p.id = w.project_id
    JOIN worktree_types wt ON wt.id = w.type_id
    WHERE w.path = p_path
      AND w.active = TRUE;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- Agent Activity Functions
-- ============================================================================

-- Log agent activity
CREATE OR REPLACE FUNCTION log_activity(
    p_agent_id TEXT,
    p_worktree_id UUID,
    p_activity_type TEXT,
    p_description TEXT,
    p_file_path TEXT DEFAULT NULL,
    p_metadata JSONB DEFAULT '{}'::jsonb
) RETURNS UUID AS $$
DECLARE
    v_activity_id UUID;
BEGIN
    INSERT INTO agent_activities (
        agent_id, worktree_id, activity_type, description, file_path, metadata
    ) VALUES (
        p_agent_id, p_worktree_id, p_activity_type, p_description, p_file_path, p_metadata
    )
    RETURNING id INTO v_activity_id;

    -- Touch the worktree to update its timestamp
    UPDATE worktrees
    SET updated_at = NOW()
    WHERE id = p_worktree_id;

    RETURN v_activity_id;
END;
$$ LANGUAGE plpgsql;

-- Get agent's recent work
CREATE OR REPLACE FUNCTION get_agent_recent_work(
    p_agent_id TEXT,
    p_limit INTEGER DEFAULT 50
) RETURNS TABLE (
    activity_id UUID,
    activity_type TEXT,
    description TEXT,
    file_path TEXT,
    created_at TIMESTAMPTZ,
    worktree_name TEXT,
    project_name TEXT,
    worktree_type TEXT
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        aa.id,
        aa.activity_type,
        aa.description,
        aa.file_path,
        aa.created_at,
        w.name,
        p.name,
        wt.name
    FROM agent_activities aa
    JOIN worktrees w ON w.id = aa.worktree_id
    JOIN projects p ON p.id = w.project_id
    JOIN worktree_types wt ON wt.id = w.type_id
    WHERE aa.agent_id = p_agent_id
    ORDER BY aa.created_at DESC
    LIMIT p_limit;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- Query Helpers
-- ============================================================================

-- Get all in-flight work for a project
CREATE OR REPLACE FUNCTION get_inflight_work(
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
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        w.id,
        w.name,
        w.branch_name,
        CASE
            WHEN w.has_uncommitted_changes THEN 'uncommitted'
            WHEN w.ahead_of_trunk > 0 AND w.behind_trunk = 0 THEN 'ahead'
            WHEN w.ahead_of_trunk = 0 AND w.behind_trunk > 0 THEN 'behind'
            WHEN w.ahead_of_trunk > 0 AND w.behind_trunk > 0 THEN 'diverged'
            ELSE 'clean'
        END,
        w.uncommitted_files_count,
        w.ahead_of_trunk,
        w.behind_trunk,
        w.agent_id,
        w.updated_at
    FROM worktrees w
    WHERE w.active = TRUE
      AND w.merged_at IS NULL
      AND (p_project_id IS NULL OR w.project_id = p_project_id)
      AND (
        w.has_uncommitted_changes = TRUE
        OR w.ahead_of_trunk > 0
        OR w.behind_trunk > 0
      )
    ORDER BY w.updated_at DESC;
END;
$$ LANGUAGE plpgsql;

-- Get deterministic working path for project
CREATE OR REPLACE FUNCTION get_project_working_path(
    p_project_id UUID,
    p_worktree_name TEXT DEFAULT NULL
) RETURNS TEXT AS $$
DECLARE
    v_trunk_path TEXT;
    v_parent_dir TEXT;
BEGIN
    -- Get trunk path
    SELECT trunk_path INTO v_trunk_path
    FROM projects
    WHERE id = p_project_id
      AND active = TRUE;

    IF v_trunk_path IS NULL THEN
        RAISE EXCEPTION 'Project not found: %', p_project_id;
    END IF;

    -- If no worktree specified, return trunk
    IF p_worktree_name IS NULL THEN
        RETURN v_trunk_path;
    END IF;

    -- Extract parent directory and append worktree name
    v_parent_dir := regexp_replace(v_trunk_path, '/[^/]+$', '');
    RETURN v_parent_dir || '/' || p_worktree_name;
END;
$$ LANGUAGE plpgsql;

-- Search projects by name or origin (fuzzy)
CREATE OR REPLACE FUNCTION search_projects(
    p_query TEXT
) RETURNS TABLE (
    id UUID,
    name TEXT,
    remote_origin TEXT,
    trunk_path TEXT,
    relevance REAL
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        p.id,
        p.name,
        p.remote_origin,
        p.trunk_path,
        GREATEST(
            similarity(p.name, p_query),
            similarity(p.remote_origin, p_query)
        ) AS relevance
    FROM projects p
    WHERE p.active = TRUE
      AND (
        p.name ILIKE '%' || p_query || '%'
        OR p.remote_origin ILIKE '%' || p_query || '%'
      )
    ORDER BY relevance DESC, p.updated_at DESC
    LIMIT 20;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- Maintenance Functions
-- ============================================================================

-- Clean up old agent activities (retention policy)
CREATE OR REPLACE FUNCTION cleanup_old_activities(
    p_retention_days INTEGER DEFAULT 90
) RETURNS INTEGER AS $$
DECLARE
    v_deleted_count INTEGER;
BEGIN
    DELETE FROM agent_activities
    WHERE created_at < NOW() - (p_retention_days || ' days')::INTERVAL;

    GET DIAGNOSTICS v_deleted_count = ROW_COUNT;
    RETURN v_deleted_count;
END;
$$ LANGUAGE plpgsql;

-- Prune inactive worktrees older than threshold
CREATE OR REPLACE FUNCTION prune_inactive_worktrees(
    p_inactive_days INTEGER DEFAULT 30
) RETURNS INTEGER AS $$
DECLARE
    v_pruned_count INTEGER;
BEGIN
    DELETE FROM worktrees
    WHERE active = FALSE
      AND updated_at < NOW() - (p_inactive_days || ' days')::INTERVAL;

    GET DIAGNOSTICS v_pruned_count = ROW_COUNT;
    RETURN v_pruned_count;
END;
$$ LANGUAGE plpgsql;

-- Vacuum and analyze all tables
CREATE OR REPLACE FUNCTION maintenance_vacuum()
RETURNS VOID AS $$
BEGIN
    VACUUM ANALYZE projects;
    VACUUM ANALYZE worktrees;
    VACUUM ANALYZE worktree_types;
    VACUUM ANALYZE agent_activities;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- Statistics Functions
-- ============================================================================

-- Get overall registry statistics
CREATE OR REPLACE FUNCTION get_registry_stats()
RETURNS TABLE (
    total_projects BIGINT,
    active_projects BIGINT,
    total_worktrees BIGINT,
    active_worktrees BIGINT,
    in_flight_worktrees BIGINT,
    total_activities BIGINT,
    activities_last_24h BIGINT
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        (SELECT COUNT(*) FROM projects),
        (SELECT COUNT(*) FROM projects WHERE active = TRUE),
        (SELECT COUNT(*) FROM worktrees),
        (SELECT COUNT(*) FROM worktrees WHERE active = TRUE),
        (SELECT COUNT(*) FROM worktrees
         WHERE active = TRUE
           AND merged_at IS NULL
           AND (has_uncommitted_changes = TRUE OR ahead_of_trunk > 0)),
        (SELECT COUNT(*) FROM agent_activities),
        (SELECT COUNT(*) FROM agent_activities
         WHERE created_at > NOW() - INTERVAL '24 hours');
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- Comments
-- ============================================================================

COMMENT ON FUNCTION register_project IS 'Register a new project with automatic path generation and duplicate prevention';
COMMENT ON FUNCTION register_worktree IS 'Register a new worktree with automatic type lookup';
COMMENT ON FUNCTION update_worktree_git_state IS 'Update worktree git state from external sync';
COMMENT ON FUNCTION get_worktree_by_path IS 'Lookup worktree by filesystem path';
COMMENT ON FUNCTION get_inflight_work IS 'Get all worktrees with uncommitted or unmerged work';
COMMENT ON FUNCTION get_project_working_path IS 'Get deterministic working path for a project/worktree';
