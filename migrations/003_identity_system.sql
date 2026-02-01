-- ============================================================================
-- iMi Identity System - Entity-Based Workspace Isolation
-- Version: 2.0.0
-- Purpose: Token-based identity for unified human/Yi-agent workspace management
-- ============================================================================

-- ============================================================================
-- Core Identity Tables
-- ============================================================================

-- Entities: Unified identity for all actors (humans, Yi agents, service accounts)
-- No user-facing distinction between entity types
CREATE TABLE entities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Identity
    name TEXT NOT NULL UNIQUE,  -- Unique identifier (e.g., 'delorenj', 'yi-backend-001')
    display_name TEXT,           -- Optional friendly name

    -- Workspace
    workspace_root TEXT NOT NULL UNIQUE,  -- Entity's isolated workspace directory

    -- Authentication
    auth_token_hash TEXT NOT NULL UNIQUE,  -- Bcrypt-hashed token
    token_created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    token_expires_at TIMESTAMPTZ,  -- NULL = never expires

    -- Integration (internal use only)
    flume_id UUID UNIQUE,  -- Yi agent reference (NULL for humans)
    metadata JSONB DEFAULT '{}'::jsonb,  -- Extensible attributes

    -- Lifecycle
    active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraints
    CONSTRAINT entities_name_check CHECK (length(name) > 0),
    CONSTRAINT entities_workspace_root_check CHECK (length(workspace_root) > 0)
);

-- Workspaces: Entity-owned project clones
CREATE TABLE workspaces (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id UUID NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,

    -- Clone details
    clone_path TEXT NOT NULL UNIQUE,
    clone_created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_accessed_at TIMESTAMPTZ,

    -- Metadata
    metadata JSONB DEFAULT '{}'::jsonb,

    -- Constraints
    UNIQUE(entity_id, project_id)  -- One clone per project per entity
);

-- Workspace Access Log: Accountability for cross-entity access
CREATE TABLE workspace_access_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    accessor_entity_id UUID NOT NULL REFERENCES entities(id) ON DELETE CASCADE,

    -- Access details
    access_type TEXT NOT NULL CHECK (access_type IN ('clone', 'modify', 'view', 'delete')),
    file_path TEXT,
    plane_ticket_id TEXT,  -- Optional Plane ticket reference

    -- Context
    metadata JSONB DEFAULT '{}'::jsonb,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================================
-- Indexes
-- ============================================================================

-- Entities indexes
CREATE INDEX idx_entities_active ON entities (active) WHERE active = TRUE;
CREATE INDEX idx_entities_flume_id ON entities (flume_id) WHERE flume_id IS NOT NULL;
CREATE INDEX idx_entities_metadata ON entities USING gin (metadata);

-- Workspaces indexes
CREATE INDEX idx_workspaces_entity ON workspaces (entity_id);
CREATE INDEX idx_workspaces_project ON workspaces (project_id);
CREATE INDEX idx_workspaces_last_accessed ON workspaces (last_accessed_at DESC);

-- Access log indexes
CREATE INDEX idx_workspace_access_log_workspace ON workspace_access_log (workspace_id);
CREATE INDEX idx_workspace_access_log_accessor ON workspace_access_log (accessor_entity_id);
CREATE INDEX idx_workspace_access_log_timestamp ON workspace_access_log (timestamp DESC);
CREATE INDEX idx_workspace_access_log_ticket ON workspace_access_log (plane_ticket_id) WHERE plane_ticket_id IS NOT NULL;

-- ============================================================================
-- Triggers
-- ============================================================================

-- Auto-update updated_at timestamp
CREATE TRIGGER entities_updated_at
    BEFORE UPDATE ON entities
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Auto-update workspace last_accessed_at on access log
CREATE OR REPLACE FUNCTION update_workspace_access_time()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE workspaces
    SET last_accessed_at = NEW.timestamp
    WHERE id = NEW.workspace_id;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER workspace_access_log_update_time
    AFTER INSERT ON workspace_access_log
    FOR EACH ROW
    EXECUTE FUNCTION update_workspace_access_time();

-- ============================================================================
-- Helper Functions
-- ============================================================================

-- Register a new entity (unified - no type distinction)
CREATE OR REPLACE FUNCTION register_entity(
    p_name TEXT,
    p_workspace_root TEXT,
    p_token_hash TEXT,
    p_flume_id UUID DEFAULT NULL,
    p_metadata JSONB DEFAULT '{}'::jsonb
) RETURNS UUID AS $$
DECLARE
    v_entity_id UUID;
BEGIN
    INSERT INTO entities (
        name, workspace_root, auth_token_hash, flume_id, metadata
    ) VALUES (
        p_name, p_workspace_root, p_token_hash, p_flume_id, p_metadata
    )
    ON CONFLICT (name) DO UPDATE SET
        auth_token_hash = EXCLUDED.auth_token_hash,
        token_created_at = NOW(),
        updated_at = NOW(),
        active = TRUE
    RETURNING id INTO v_entity_id;

    RETURN v_entity_id;
END;
$$ LANGUAGE plpgsql;

-- Claim workspace for a project (creates entity-owned clone)
CREATE OR REPLACE FUNCTION claim_workspace(
    p_entity_id UUID,
    p_project_id UUID,
    p_clone_path TEXT,
    p_metadata JSONB DEFAULT '{}'::jsonb
) RETURNS UUID AS $$
DECLARE
    v_workspace_id UUID;
BEGIN
    INSERT INTO workspaces (
        entity_id, project_id, clone_path, metadata
    ) VALUES (
        p_entity_id, p_project_id, p_clone_path, p_metadata
    )
    ON CONFLICT (entity_id, project_id) DO UPDATE SET
        last_accessed_at = NOW(),
        metadata = workspaces.metadata || EXCLUDED.metadata
    RETURNING id INTO v_workspace_id;

    RETURN v_workspace_id;
END;
$$ LANGUAGE plpgsql;

-- Log workspace access
CREATE OR REPLACE FUNCTION log_workspace_access(
    p_workspace_id UUID,
    p_accessor_entity_id UUID,
    p_access_type TEXT,
    p_file_path TEXT DEFAULT NULL,
    p_plane_ticket_id TEXT DEFAULT NULL,
    p_metadata JSONB DEFAULT '{}'::jsonb
) RETURNS UUID AS $$
DECLARE
    v_log_id UUID;
BEGIN
    INSERT INTO workspace_access_log (
        workspace_id, accessor_entity_id, access_type, file_path, plane_ticket_id, metadata
    ) VALUES (
        p_workspace_id, p_accessor_entity_id, p_access_type, p_file_path, p_plane_ticket_id, p_metadata
    )
    RETURNING id INTO v_log_id;

    RETURN v_log_id;
END;
$$ LANGUAGE plpgsql;

-- Resolve entity from token (authentication)
CREATE OR REPLACE FUNCTION get_entity_by_token_hash(
    p_token_hash TEXT
) RETURNS TABLE (
    id UUID,
    name TEXT,
    workspace_root TEXT,
    flume_id UUID,
    active BOOLEAN
) AS $$
BEGIN
    RETURN QUERY
    SELECT e.id, e.name, e.workspace_root, e.flume_id, e.active
    FROM entities e
    WHERE e.auth_token_hash = p_token_hash
      AND e.active = TRUE;
END;
$$ LANGUAGE plpgsql;

-- Get entity's workspaces
CREATE OR REPLACE FUNCTION get_entity_workspaces(
    p_entity_id UUID
) RETURNS TABLE (
    workspace_id UUID,
    project_id UUID,
    project_name TEXT,
    clone_path TEXT,
    last_accessed_at TIMESTAMPTZ
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        w.id,
        p.id,
        p.name,
        w.clone_path,
        w.last_accessed_at
    FROM workspaces w
    JOIN projects p ON p.id = w.project_id
    WHERE w.entity_id = p_entity_id
    ORDER BY w.last_accessed_at DESC NULLS LAST;
END;
$$ LANGUAGE plpgsql;

-- Get all workspaces across all entities (global view)
CREATE OR REPLACE FUNCTION get_all_workspaces()
RETURNS TABLE (
    workspace_id UUID,
    entity_name TEXT,
    project_name TEXT,
    clone_path TEXT,
    last_accessed_at TIMESTAMPTZ
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        w.id,
        e.name,
        p.name,
        w.clone_path,
        w.last_accessed_at
    FROM workspaces w
    JOIN entities e ON e.id = w.entity_id
    JOIN projects p ON p.id = w.project_id
    WHERE e.active = TRUE
    ORDER BY w.last_accessed_at DESC NULLS LAST;
END;
$$ LANGUAGE plpgsql;

-- Audit workspace access (who touched what)
CREATE OR REPLACE FUNCTION get_workspace_access_audit(
    p_workspace_id UUID,
    p_limit INTEGER DEFAULT 100
) RETURNS TABLE (
    timestamp TIMESTAMPTZ,
    accessor_name TEXT,
    access_type TEXT,
    file_path TEXT,
    plane_ticket_id TEXT
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        wal.timestamp,
        e.name,
        wal.access_type,
        wal.file_path,
        wal.plane_ticket_id
    FROM workspace_access_log wal
    JOIN entities e ON e.id = wal.accessor_entity_id
    WHERE wal.workspace_id = p_workspace_id
    ORDER BY wal.timestamp DESC
    LIMIT p_limit;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- Views
-- ============================================================================

-- Active entities with workspace counts
CREATE VIEW v_entities_summary AS
SELECT
    e.id,
    e.name,
    e.display_name,
    e.workspace_root,
    e.flume_id,
    e.created_at,
    e.updated_at,
    COUNT(w.id) AS workspace_count
FROM entities e
LEFT JOIN workspaces w ON w.entity_id = e.id
WHERE e.active = TRUE
GROUP BY e.id;

-- Workspaces with entity and project details
CREATE VIEW v_workspaces_detail AS
SELECT
    w.id AS workspace_id,
    e.id AS entity_id,
    e.name AS entity_name,
    p.id AS project_id,
    p.name AS project_name,
    p.remote_origin,
    w.clone_path,
    w.clone_created_at,
    w.last_accessed_at
FROM workspaces w
JOIN entities e ON e.id = w.entity_id
JOIN projects p ON p.id = w.project_id
WHERE e.active = TRUE;

-- ============================================================================
-- Migration Notes
-- ============================================================================

-- This migration introduces the identity system for workspace isolation.
-- Existing cluster hub model (shared .iMi directory) will be migrated to
-- entity workspaces in a future migration.
--
-- Key concepts:
-- - Entities: Unified actors (no user-facing type distinction)
-- - Workspaces: Entity-owned project clones (isolated directories)
-- - Access logs: Accountability for cross-entity workspace access
-- - Token-based auth: All entities authenticate via $IMI_IDENTITY_TOKEN

-- ============================================================================
-- Comments
-- ============================================================================

COMMENT ON TABLE entities IS 'Unified identity for all actors (humans, Yi agents, service accounts) with token-based authentication';
COMMENT ON TABLE workspaces IS 'Entity-owned project clones for workspace isolation';
COMMENT ON TABLE workspace_access_log IS 'Audit log for cross-entity workspace access and accountability';

COMMENT ON COLUMN entities.name IS 'Unique identifier (e.g., delorenj, yi-backend-001)';
COMMENT ON COLUMN entities.workspace_root IS 'Entity''s isolated workspace directory';
COMMENT ON COLUMN entities.flume_id IS 'Yi agent reference for Flume integration (NULL for humans, internal use only)';
COMMENT ON COLUMN workspaces.clone_path IS 'Full path to entity''s clone of this project';
COMMENT ON COLUMN workspace_access_log.plane_ticket_id IS 'Optional Plane ticket justifying cross-entity access';
