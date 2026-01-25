-- ============================================================================
-- iMi Project Registry - PostgreSQL Schema
-- Version: 2.0.0
-- Purpose: Authoritative project registry for 33GOD agentic pipeline
-- ============================================================================

-- Enable UUID generation
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- ============================================================================
-- Core Tables
-- ============================================================================

-- Projects: Single source of truth for all 33GOD projects
-- Enforces 1:1 mapping between project_id and GitHub remote origin
CREATE TABLE projects (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    remote_origin TEXT NOT NULL,
    default_branch TEXT NOT NULL DEFAULT 'main',
    trunk_path TEXT NOT NULL,

    -- Metadata
    description TEXT,
    metadata JSONB DEFAULT '{}'::jsonb,

    -- Lifecycle
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    active BOOLEAN NOT NULL DEFAULT TRUE,

    -- Constraints
    CONSTRAINT projects_name_check CHECK (length(name) > 0),
    CONSTRAINT projects_remote_origin_check CHECK (remote_origin ~ '^git@github\.com:[^/]+/.+\.git$'),
    CONSTRAINT projects_trunk_path_check CHECK (length(trunk_path) > 0),
    CONSTRAINT projects_unique_remote_origin UNIQUE (remote_origin),
    CONSTRAINT projects_unique_active_remote EXCLUDE USING btree (remote_origin WITH =) WHERE (active = TRUE)
);

-- Worktree types: Built-in and custom worktree classifications
CREATE TABLE worktree_types (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    branch_prefix TEXT NOT NULL,
    worktree_prefix TEXT NOT NULL,
    description TEXT,
    is_builtin BOOLEAN NOT NULL DEFAULT FALSE,

    -- Metadata
    color TEXT DEFAULT '#6B7280',
    icon TEXT,
    metadata JSONB DEFAULT '{}'::jsonb,

    -- Lifecycle
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraints
    CONSTRAINT worktree_types_name_check CHECK (length(name) > 0),
    CONSTRAINT worktree_types_branch_prefix_check CHECK (branch_prefix IS NOT NULL),
    CONSTRAINT worktree_types_worktree_prefix_check CHECK (length(worktree_prefix) > 0)
);

-- Worktrees: All worktrees across all projects
CREATE TABLE worktrees (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    type_id INTEGER NOT NULL REFERENCES worktree_types(id) ON DELETE RESTRICT,

    -- Identity
    name TEXT NOT NULL,
    branch_name TEXT NOT NULL,
    path TEXT NOT NULL,

    -- Agent ownership
    agent_id TEXT,

    -- Git state tracking (in-flight work)
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

    -- Metadata
    metadata JSONB DEFAULT '{}'::jsonb,

    -- Lifecycle
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    active BOOLEAN NOT NULL DEFAULT TRUE,

    -- Constraints
    CONSTRAINT worktrees_name_check CHECK (length(name) > 0),
    CONSTRAINT worktrees_branch_name_check CHECK (length(branch_name) > 0),
    CONSTRAINT worktrees_path_check CHECK (length(path) > 0),
    CONSTRAINT worktrees_unique_project_name UNIQUE (project_id, name),
    CONSTRAINT worktrees_unique_active_path EXCLUDE USING btree (path WITH =) WHERE (active = TRUE),
    CONSTRAINT worktrees_ahead_check CHECK (ahead_of_trunk >= 0),
    CONSTRAINT worktrees_behind_check CHECK (behind_trunk >= 0),
    CONSTRAINT worktrees_uncommitted_count_check CHECK (uncommitted_files_count >= 0)
);

-- Agent activities: Audit log of all agent actions
CREATE TABLE agent_activities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_id TEXT NOT NULL,
    worktree_id UUID NOT NULL REFERENCES worktrees(id) ON DELETE CASCADE,

    -- Activity details
    activity_type TEXT NOT NULL,
    file_path TEXT,
    description TEXT NOT NULL,

    -- Additional context
    metadata JSONB DEFAULT '{}'::jsonb,

    -- Lifecycle
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraints
    CONSTRAINT agent_activities_agent_id_check CHECK (length(agent_id) > 0),
    CONSTRAINT agent_activities_activity_type_check CHECK (activity_type IN (
        'created', 'modified', 'deleted', 'committed', 'pushed', 'merged', 'synced', 'other'
    ))
);

-- ============================================================================
-- Indexes for Performance
-- ============================================================================

-- Projects indexes
CREATE INDEX idx_projects_active ON projects (active) WHERE active = TRUE;
CREATE INDEX idx_projects_name ON projects (name) WHERE active = TRUE;
CREATE INDEX idx_projects_remote_origin_trgm ON projects USING gin (remote_origin gin_trgm_ops);
CREATE INDEX idx_projects_metadata ON projects USING gin (metadata);

-- Worktrees indexes
CREATE INDEX idx_worktrees_project_id ON worktrees (project_id) WHERE active = TRUE;
CREATE INDEX idx_worktrees_type_id ON worktrees (type_id);
CREATE INDEX idx_worktrees_active ON worktrees (active) WHERE active = TRUE;
CREATE INDEX idx_worktrees_agent_id ON worktrees (agent_id) WHERE agent_id IS NOT NULL AND active = TRUE;
CREATE INDEX idx_worktrees_uncommitted ON worktrees (has_uncommitted_changes) WHERE has_uncommitted_changes = TRUE AND active = TRUE;
CREATE INDEX idx_worktrees_unmerged ON worktrees (project_id, merged_at) WHERE merged_at IS NULL AND active = TRUE;
CREATE INDEX idx_worktrees_last_sync ON worktrees (last_sync_at) WHERE active = TRUE;
CREATE INDEX idx_worktrees_metadata ON worktrees USING gin (metadata);

-- Agent activities indexes
CREATE INDEX idx_agent_activities_worktree_id ON agent_activities (worktree_id);
CREATE INDEX idx_agent_activities_agent_id ON agent_activities (agent_id);
CREATE INDEX idx_agent_activities_created_at ON agent_activities (created_at DESC);
CREATE INDEX idx_agent_activities_type ON agent_activities (activity_type);
CREATE INDEX idx_agent_activities_metadata ON agent_activities USING gin (metadata);

-- ============================================================================
-- Triggers for Automation
-- ============================================================================

-- Auto-update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER projects_updated_at
    BEFORE UPDATE ON projects
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER worktrees_updated_at
    BEFORE UPDATE ON worktrees
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Sync uncommitted files count with flag
CREATE OR REPLACE FUNCTION sync_uncommitted_changes()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.uncommitted_files_count > 0 THEN
        NEW.has_uncommitted_changes = TRUE;
    ELSE
        NEW.has_uncommitted_changes = FALSE;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER worktrees_sync_uncommitted
    BEFORE INSERT OR UPDATE ON worktrees
    FOR EACH ROW
    EXECUTE FUNCTION sync_uncommitted_changes();

-- ============================================================================
-- Seed Data
-- ============================================================================

-- Built-in worktree types
INSERT INTO worktree_types (name, branch_prefix, worktree_prefix, description, is_builtin, color, icon) VALUES
    ('feat', 'feat/', 'feat-', 'Feature development', TRUE, '#10B981', '🚀'),
    ('fix', 'fix/', 'fix-', 'Bug fixes', TRUE, '#EF4444', '🐛'),
    ('aiops', 'aiops/', 'aiops-', 'AI operations (agents, rules, MCP configs, workflows)', TRUE, '#8B5CF6', '🤖'),
    ('devops', 'devops/', 'devops-', 'DevOps tasks (CI, repo organization, deploys)', TRUE, '#3B82F6', '⚙️'),
    ('review', 'pr-review/', 'pr-', 'Pull request reviews', TRUE, '#F59E0B', '👀'),
    ('trunk', '', 'trunk-', 'Main branch worktree', TRUE, '#6B7280', '🌳')
ON CONFLICT (name) DO NOTHING;

-- ============================================================================
-- Views for Common Queries
-- ============================================================================

-- Active projects with worktree counts
CREATE VIEW v_projects_summary AS
SELECT
    p.id,
    p.name,
    p.remote_origin,
    p.trunk_path,
    p.created_at,
    p.updated_at,
    COUNT(w.id) FILTER (WHERE w.active = TRUE) AS active_worktrees_count,
    COUNT(w.id) FILTER (WHERE w.active = TRUE AND w.has_uncommitted_changes = TRUE) AS uncommitted_worktrees_count,
    COUNT(w.id) FILTER (WHERE w.active = TRUE AND w.merged_at IS NULL) AS unmerged_worktrees_count,
    MAX(w.updated_at) AS last_worktree_activity
FROM projects p
LEFT JOIN worktrees w ON w.project_id = p.id
WHERE p.active = TRUE
GROUP BY p.id;

-- Worktrees with full context
CREATE VIEW v_worktrees_detail AS
SELECT
    w.id,
    w.project_id,
    p.name AS project_name,
    p.remote_origin,
    w.name AS worktree_name,
    w.branch_name,
    w.path,
    wt.name AS type_name,
    wt.branch_prefix,
    wt.worktree_prefix,
    wt.color AS type_color,
    wt.icon AS type_icon,
    w.agent_id,
    w.has_uncommitted_changes,
    w.uncommitted_files_count,
    w.ahead_of_trunk,
    w.behind_trunk,
    w.last_sync_at,
    w.merged_at,
    w.created_at,
    w.updated_at,
    w.active
FROM worktrees w
JOIN projects p ON p.id = w.project_id
JOIN worktree_types wt ON wt.id = w.type_id;

-- In-flight work view
CREATE VIEW v_inflight_work AS
SELECT
    w.id,
    w.project_id,
    p.name AS project_name,
    w.name AS worktree_name,
    w.branch_name,
    w.agent_id,
    w.has_uncommitted_changes,
    w.uncommitted_files_count,
    w.ahead_of_trunk,
    w.behind_trunk,
    w.last_sync_at,
    CASE
        WHEN w.has_uncommitted_changes THEN 'uncommitted'
        WHEN w.ahead_of_trunk > 0 AND w.behind_trunk = 0 THEN 'ahead'
        WHEN w.ahead_of_trunk = 0 AND w.behind_trunk > 0 THEN 'behind'
        WHEN w.ahead_of_trunk > 0 AND w.behind_trunk > 0 THEN 'diverged'
        ELSE 'clean'
    END AS status,
    w.created_at,
    w.updated_at
FROM worktrees w
JOIN projects p ON p.id = w.project_id
WHERE w.active = TRUE
  AND w.merged_at IS NULL
  AND (
    w.has_uncommitted_changes = TRUE
    OR w.ahead_of_trunk > 0
    OR w.behind_trunk > 0
  );

-- Recent agent activity
CREATE VIEW v_recent_agent_activity AS
SELECT
    aa.id,
    aa.agent_id,
    aa.activity_type,
    aa.description,
    aa.file_path,
    aa.created_at,
    w.name AS worktree_name,
    p.name AS project_name,
    wt.name AS worktree_type
FROM agent_activities aa
JOIN worktrees w ON w.id = aa.worktree_id
JOIN projects p ON p.id = w.project_id
JOIN worktree_types wt ON wt.id = w.type_id
ORDER BY aa.created_at DESC;

-- ============================================================================
-- Comments
-- ============================================================================

COMMENT ON TABLE projects IS 'Authoritative registry of all 33GOD projects with 1:1 mapping to GitHub remote origins';
COMMENT ON TABLE worktrees IS 'All worktrees across all projects with git state tracking';
COMMENT ON TABLE worktree_types IS 'Classification of worktrees (feat, fix, aiops, devops, etc)';
COMMENT ON TABLE agent_activities IS 'Audit log of all agent actions across all worktrees';

COMMENT ON COLUMN projects.remote_origin IS 'GitHub remote origin URL - must be unique across active projects';
COMMENT ON COLUMN projects.trunk_path IS 'Canonical base path for this project (e.g., ~/code/my-project/trunk-main)';
COMMENT ON COLUMN worktrees.has_uncommitted_changes IS 'Whether this worktree has uncommitted changes (synced from git)';
COMMENT ON COLUMN worktrees.ahead_of_trunk IS 'Number of commits ahead of trunk (synced from git)';
COMMENT ON COLUMN worktrees.behind_trunk IS 'Number of commits behind trunk (synced from git)';
COMMENT ON COLUMN worktrees.last_sync_at IS 'Last time git state was synced for this worktree';
