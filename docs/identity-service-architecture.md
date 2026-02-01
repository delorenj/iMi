# iMi Identity Service Architecture

## Executive Summary

This document outlines the Identity Service Architecture for iMi, designed to treat all actors (humans and Yi agents) as **entities** with equal standing in the 33GOD ecosystem. The architecture uses token-based authentication to resolve identities to workspace contexts, enabling secure, isolated, and auditable collaboration between humans and autonomous agents.

## Core Principles

### 1. Entity Abstraction
**Decision**: Unify humans and Yi agents under a single "entity" concept.

**Rationale**:
- Yi agents are designed to be autonomous software employees, not second-class citizens
- Treating all actors equally simplifies access control, auditing, and workspace management
- Enables seamless collaboration patterns (ticket assignment, workspace access, code review)
- Future-proofs against different agent types (Yi variants, third-party agents)

**Implementation**:
```sql
-- No entity_type enum - all entities are equal
-- Internal metadata can track integration details (e.g., flume_id for Yi agents)

CREATE TABLE entities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL UNIQUE,  -- Unique identifier (e.g., 'delorenj', 'yi-backend-001')
    display_name TEXT,           -- Optional friendly name
    workspace_root TEXT NOT NULL UNIQUE,
    auth_token_hash TEXT NOT NULL UNIQUE,
    token_created_at TIMESTAMPTZ DEFAULT NOW(),
    token_expires_at TIMESTAMPTZ,  -- NULL = never expires
    flume_id UUID UNIQUE,        -- Yi agent reference (NULL for humans, internal use only)
    metadata JSONB DEFAULT '{}'::jsonb,  -- Extensible attributes
    active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);
```

### 2. Token-Based Identity Resolution
**Decision**: Use bearer tokens as the primary authentication mechanism.

**Rationale**:
- **Stateless**: No session management required, scales horizontally
- **Yi-compatible**: Agents can be provisioned with tokens programmatically
- **Revocable**: Tokens can be invalidated without changing entity credentials
- **Auditable**: Each token resolves to exactly one entity with full context
- **Integration-ready**: Standard OAuth2/JWT patterns can be layered on top

**Token Lifecycle**:
```
1. Registration
   └─> Entity created with bcrypt-hashed token
   └─> Token stored securely in entity's config (humans: ~/.iMi/token, Yi: Flume vault)

2. Authentication
   └─> Client sends token with request
   └─> iMi validates hash against entities table
   └─> Resolves to entity ID + workspace context

3. Usage
   └─> All operations tagged with entity_id
   └─> Workspace modifications logged to workspace_access_log
   └─> Cross-workspace access triggers courtesy notifications

4. Rotation (optional)
   └─> Generate new token
   └─> Update entities.auth_token_hash
   └─> Invalidate old token
   └─> Update client config

5. Revocation
   └─> Set entities.active = FALSE
   └─> Token immediately invalid
   └─> Entity retains audit history
```

**Security Considerations**:
- Tokens hashed with bcrypt (cost factor 12)
- 256-bit random tokens (base64url encoded, ~43 chars)
- Environment-based token storage (IMI_TOKEN env var)
- No tokens in logs or error messages
- Rate limiting per entity (future)

### 3. Workspace Isolation
**Decision**: Each entity owns a completely isolated workspace directory with full project clones.

**Rationale**:
- **True isolation**: No shared state between entities (unlike git worktrees)
- **Concurrent safety**: Entities can work on same project without conflicts
- **Agent autonomy**: Yi agents have full control over their workspace
- **Accountability**: All modifications traceable to specific entity
- **Social protocol**: Cross-workspace access requires explicit intent (ticket assignment)

**Workspace Structure**:
```
/home/delorenj/33GOD/workspaces/
├── delorenj/              # No humans/ subdirectory
│   ├── iMi/
│   │   ├── trunk-main/    # Full clone
│   │   ├── feat-auth/     # Worktree
│   │   └── fix-sync/      # Worktree
│   └── other-project/
├── yi-backend-001/        # Yi agents are peers, not segregated
│   └── iMi/
│       └── feat-api/
└── yi-frontend-002/
    └── iMi/
        └── feat-ui/
```

**Workspace Schema**:
```sql
CREATE TABLE workspaces (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id UUID NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    clone_path TEXT NOT NULL UNIQUE,
    clone_created_at TIMESTAMPTZ DEFAULT NOW(),
    last_accessed_at TIMESTAMPTZ,
    metadata JSONB DEFAULT '{}'::jsonb,
    UNIQUE(entity_id, project_id)
);

CREATE TABLE workspace_access_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    accessor_entity_id UUID NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    access_type TEXT NOT NULL,  -- 'clone', 'modify', 'view', 'delete'
    file_path TEXT,
    plane_ticket_id TEXT,       -- Optional ticket reference
    timestamp TIMESTAMPTZ DEFAULT NOW(),
    metadata JSONB DEFAULT '{}'::jsonb
);
```

### 4. Flume Integration (Yi Agent Registration)
**Decision**: Treat Flume as the authoritative source for Yi agent lifecycle events.

**Rationale**:
- **Separation of concerns**: Flume manages Yi infrastructure, iMi manages workspaces
- **Eventual consistency**: iMi syncs entity state from Flume via webhooks/polling
- **Graceful degradation**: iMi can operate independently if Flume unavailable
- **Future-proof**: Integration contract remains stable as Yi design evolves

**Integration API** (conceptual - Yi not yet designed):
```rust
// Flume → iMi webhook payload (future)
{
    "event": "yi.agent.registered",
    "yi_id": "yi-backend-001",
    "flume_id": "550e8400-e29b-41d4-a716-446655440000",
    "metadata": {
        "specialization": "backend-development",
        "model": "claude-3.5-sonnet",
        "version": "1.0.0"
    }
}

// iMi handler (future implementation)
async fn handle_yi_registered(payload: FlumeYiEvent) -> Result<()> {
    let token = generate_token();
    let workspace_root = format!("/home/delorenj/33GOD/workspaces/yi-agents/{}", payload.yi_id);

    sqlx::query!(
        "INSERT INTO entities (entity_type, name, workspace_root, auth_token_hash, flume_id)
         VALUES ('yi-agent', $1, $2, $3, $4)",
        payload.yi_id,
        workspace_root,
        hash_token(&token),
        payload.flume_id
    ).execute(pool).await?;

    // Provision workspace directory
    tokio::fs::create_dir_all(&workspace_root).await?;

    // Store token in Flume vault (API call)
    flume_client.store_token(&payload.flume_id, &token).await?;

    Ok(())
}
```

**Integration Points**:
- **Registration**: Flume webhook → iMi creates entity + workspace
- **Deactivation**: Flume webhook → iMi sets entity.active = FALSE
- **Token refresh**: iMi API → Flume vault update
- **Workspace sync**: Periodic reconciliation (detect orphaned workspaces)

### 5. Future-Proofing for Yi Design
**Decision**: Design identity system as a stable interface that Yi will consume.

**Rationale**:
- **Dependency inversion**: Yi depends on iMi identity contract, not vice versa
- **Extensibility**: Entity metadata JSONB allows Yi-specific attributes without schema changes
- **Abstraction**: Entity model hides Yi implementation details from workspace management
- **Stability**: Identity API remains constant as Yi architecture evolves

**Yi Design Unknowns (and how we handle them)**:

| Unknown | Identity System Design |
|---------|------------------------|
| Yi agent lifecycle | Generic entity activation/deactivation |
| Yi authentication method | Token-based, Yi stores tokens securely |
| Yi workspace preferences | JSONB metadata extensible |
| Yi scaling model | entity_type can include variants (`yi-agent-v2`) |
| Yi multi-tenancy | flume_id provides organization context |
| Yi capabilities | Metadata captures specializations |

**Extension Example**:
```sql
-- When Yi launches, we might add:
UPDATE entities
SET metadata = jsonb_set(
    metadata,
    '{yi_capabilities}',
    '["code-review", "documentation", "testing"]'::jsonb
)
WHERE entity_type = 'yi-agent' AND name = 'yi-backend-001';

-- Schema never changes, queries adapt:
SELECT * FROM entities
WHERE entity_type = 'yi-agent'
  AND metadata->>'yi_specialization' = 'backend-development';
```

## Detailed Component Design

### Entity Management Commands

```bash
# Entity registration (unified - no type distinction)
$ imi entity register
Enter your name: delorenj
Generated token: imi_tok_abc123...
Token saved to ~/.iMi/token
Workspace created at: /home/delorenj/33GOD/workspaces/delorenj

# Yi agent registration (programmatic, via Flume integration - future)
# Flume calls: imi entity register --name yi-backend-001 --flume-id <uuid>
# Token automatically stored in Flume vault

# List entities
$ imi entity list
ID                                     NAME              ACTIVE  WORKSPACE
a1b2c3d4-...                          delorenj          ✓       /home/.../delorenj
e5f6g7h8-...                          yi-backend-001    ✓       /home/.../yi-backend-001

# Deactivate entity
$ imi entity deactivate yi-backend-001
Entity yi-backend-001 deactivated. Token invalidated.

# Rotate token
$ imi entity rotate-token
New token: imi_tok_def456...
Token saved to ~/.iMi/token
```

### Workspace Management Commands

```bash
# Claim workspace for project (creates full clone)
$ imi workspace claim iMi
Cloning git@github.com:delorenj/iMi.git...
Workspace created: /home/delorenj/33GOD/workspaces/humans/delorenj/iMi
Registered trunk-main worktree.

# Within workspace, create worktrees (standard iMi commands)
$ cd /home/delorenj/33GOD/workspaces/humans/delorenj/iMi
$ imi add feat auth-refactor
Created feat-auth-refactor worktree.

# List my workspaces
$ imi workspace list
PROJECT    CLONE PATH                                    WORKTREES  LAST ACCESSED
iMi        .../humans/delorenj/iMi                      3          2 hours ago
cli-tool   .../humans/delorenj/cli-tool                 1          1 day ago

# Audit workspace access (who touched my stuff?)
$ imi workspace audit iMi
TIMESTAMP            ACCESSOR        ACTION    FILE PATH                    TICKET
2025-01-15 14:30     yi-backend-001  modify    src/commands/registry.rs     PLN-123
2025-01-15 14:25     delorenj        modify    src/cli.rs                   -

# Cross-entity access (requires ticket)
$ imi workspace access yi-backend-001/iMi --ticket PLN-456
Accessing workspace: /home/.../yi-agents/yi-backend-001/iMi
Ticket PLN-456 logged. Agent will be notified.
Current directory: /home/.../yi-agents/yi-backend-001/iMi/trunk-main
```

### Authentication Flow

```rust
// iMi CLI loads token from environment or config
fn get_auth_token() -> Result<String> {
    std::env::var("IMI_TOKEN")
        .or_else(|_| {
            let config_path = dirs::home_dir()
                .ok_or_else(|| anyhow!("No home directory"))?
                .join(".iMi/token");
            std::fs::read_to_string(config_path)
        })
        .context("No authentication token found")
}

// Every iMi command resolves entity context
async fn resolve_entity(pool: &PgPool, token: &str) -> Result<Entity> {
    let entities = sqlx::query_as!(
        Entity,
        "SELECT * FROM entities WHERE active = TRUE"
    )
    .fetch_all(pool)
    .await?;

    // Verify token hash
    for entity in entities {
        if bcrypt::verify(token, &entity.auth_token_hash)? {
            // Update last accessed
            sqlx::query!(
                "UPDATE entities SET updated_at = NOW() WHERE id = $1",
                entity.id
            ).execute(pool).await?;

            return Ok(entity);
        }
    }

    Err(anyhow!("Invalid token"))
}

// Commands use entity context for all operations
async fn handle_add_command(
    pool: &PgPool,
    entity: &Entity,
    worktree_type: &str,
    name: &str
) -> Result<()> {
    // Create worktree in entity's workspace
    let workspace_path = PathBuf::from(&entity.workspace_root)
        .join("current-project"); // Resolved from context

    // Log access
    log_workspace_access(
        pool,
        &workspace_path,
        entity.id,
        "modify",
        Some(&format!("{}-{}", worktree_type, name)),
        None
    ).await?;

    // Proceed with worktree creation...
    Ok(())
}
```

## Migration Path

### Phase 1: Database Schema
1. Create entities table
2. Create workspaces table
3. Create workspace_access_log table
4. Add functions: `register_entity()`, `claim_workspace()`

### Phase 2: Entity Registration
1. Implement `imi entity register --human` command
2. Create token generation and hashing utilities
3. Store tokens in `~/.iMi/token`
4. Migrate current user to default entity (`delorenj`)

### Phase 3: Workspace Isolation
1. Create workspace directory structure
2. Implement `imi workspace claim <project>` command
3. Update all iMi commands to use entity context
4. Migrate existing cluster hubs to entity workspaces

### Phase 4: Access Logging
1. Implement workspace_access_log writes
2. Create `imi workspace audit` command
3. Add cross-workspace access with ticket validation

### Phase 5: Flume Integration Hooks (future)
1. Define Flume webhook contract
2. Implement Yi agent registration endpoint
3. Add token provisioning to Flume vault
4. Build reconciliation job for orphaned workspaces

## Security Considerations

### Token Security
- **Generation**: `rand::thread_rng()` with 256-bit entropy
- **Storage**: File permissions 0600 (user-only read/write)
- **Transmission**: Environment variables only (no CLI args)
- **Hashing**: bcrypt cost factor 12 (2^12 = 4096 rounds)
- **Rotation**: On-demand via `imi entity rotate-token`

### Access Control
- **Workspace boundaries**: Enforced by filesystem permissions + audit log
- **Cross-entity access**: Requires explicit ticket reference
- **Token invalidation**: Immediate via `active = FALSE` flag
- **Audit trail**: All modifications logged with entity_id

### Future Enhancements
- **Token expiration**: Optional TTL for Yi agents
- **Role-based access**: Entity metadata can include roles/permissions
- **Multi-factor auth**: Human entities could require additional verification
- **Rate limiting**: Per-entity request throttling

## Alignment with Yi Design Goals

This identity architecture is designed to be **Yi-agnostic** while remaining **Yi-ready**:

✅ **Works today**: Humans can use it immediately
✅ **Works tomorrow**: Yi agents plug in via Flume integration
✅ **Works forever**: Extensible metadata supports unknown Yi features
✅ **Works anywhere**: Token-based auth scales to distributed Yi clusters

When Yi is designed, the identity system will:
- Provide stable authentication interface
- Enable workspace isolation per agent
- Support audit trails for agent actions
- Allow Flume to manage Yi lifecycle
- Remain unchanged as Yi evolves

## Appendix: SQL Schema

```sql
-- Core entity model
CREATE TYPE entity_type AS ENUM ('human', 'yi-agent', 'service-account');

CREATE TABLE entities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_type entity_type NOT NULL,
    name TEXT NOT NULL UNIQUE,
    display_name TEXT,
    workspace_root TEXT NOT NULL UNIQUE,
    auth_token_hash TEXT NOT NULL UNIQUE,
    token_created_at TIMESTAMPTZ DEFAULT NOW(),
    token_expires_at TIMESTAMPTZ,
    flume_id UUID UNIQUE,
    metadata JSONB DEFAULT '{}'::jsonb,
    active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_entities_active ON entities(active);
CREATE INDEX idx_entities_flume_id ON entities(flume_id) WHERE flume_id IS NOT NULL;
CREATE INDEX idx_entities_metadata ON entities USING gin(metadata);

-- Workspace ownership
CREATE TABLE workspaces (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id UUID NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    clone_path TEXT NOT NULL UNIQUE,
    clone_created_at TIMESTAMPTZ DEFAULT NOW(),
    last_accessed_at TIMESTAMPTZ,
    metadata JSONB DEFAULT '{}'::jsonb,
    UNIQUE(entity_id, project_id)
);

CREATE INDEX idx_workspaces_entity ON workspaces(entity_id);
CREATE INDEX idx_workspaces_project ON workspaces(project_id);

-- Access audit log
CREATE TABLE workspace_access_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    accessor_entity_id UUID NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    access_type TEXT NOT NULL CHECK (access_type IN ('clone', 'modify', 'view', 'delete')),
    file_path TEXT,
    plane_ticket_id TEXT,
    timestamp TIMESTAMPTZ DEFAULT NOW(),
    metadata JSONB DEFAULT '{}'::jsonb
);

CREATE INDEX idx_workspace_access_log_workspace ON workspace_access_log(workspace_id);
CREATE INDEX idx_workspace_access_log_accessor ON workspace_access_log(accessor_entity_id);
CREATE INDEX idx_workspace_access_log_timestamp ON workspace_access_log(timestamp DESC);

-- Helper functions
CREATE OR REPLACE FUNCTION register_entity(
    p_entity_type entity_type,
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
        entity_type, name, workspace_root, auth_token_hash, flume_id, metadata
    ) VALUES (
        p_entity_type, p_name, p_workspace_root, p_token_hash, p_flume_id, p_metadata
    )
    ON CONFLICT (name) DO UPDATE SET
        auth_token_hash = EXCLUDED.auth_token_hash,
        token_created_at = NOW(),
        updated_at = NOW()
    RETURNING id INTO v_entity_id;

    RETURN v_entity_id;
END;
$$ LANGUAGE plpgsql;

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
```

## Conclusion

This Identity Service Architecture provides a **stable, secure, and extensible** foundation for treating Yi agents as first-class entities in the 33GOD ecosystem. By abstracting identity resolution to token-based authentication and isolating workspaces per entity, we enable:

- **Immediate value**: Humans can use the system today
- **Future integration**: Yi agents plug in seamlessly via Flume
- **Auditability**: All actions traceable to entities
- **Scalability**: Stateless auth supports distributed Yi clusters
- **Flexibility**: JSONB metadata adapts to unknown Yi requirements

The architecture respects the principle that **Yi design is pending**, providing integration hooks without making assumptions about Yi internals. When Yi is designed, it will consume the identity system as a stable interface, not dictate its implementation.
