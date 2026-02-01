# SQLite to PostgreSQL Migration Strategy

**Version**: 2.0.0
**Component**: iMi Project Registry
**Purpose**: Step-by-step migration from SQLite to PostgreSQL
**Created**: 2026-01-21

## Overview

This document outlines the strategy for migrating iMi from SQLite to PostgreSQL while maintaining system availability and data integrity. The migration addresses schema normalization issues and positions iMi as the authoritative Project Registry for 33GOD.

## Migration Goals

1. **Zero Data Loss**: All projects, worktrees, and activities migrated intact
2. **Minimal Downtime**: Phased approach with optional dual-write period
3. **Schema Normalization**: Fix TEXT-based foreign keys, enforce UUID relationships
4. **Backward Compatibility**: Maintain CLI and MCP tool interfaces during transition
5. **Rollback Capability**: Full rollback to SQLite if issues arise

## Pre-Migration Assessment

### Current SQLite Schema Issues

**Identified Problems:**

1. **Broken Foreign Keys**
   - `worktrees.repo_name` references `repositories.name` (TEXT)
   - Should reference `repositories.id` (UUID)

2. **Unused Primary Keys**
   - `repositories.id` UUID exists but never used as FK target
   - No referential integrity enforcement

3. **Missing Features**
   - No in-flight work tracking (uncommitted changes, ahead/behind trunk)
   - No agent activity logging
   - Limited concurrent access (file-level locking)

4. **Extensibility Constraints**
   - Schema changes require migrations
   - No JSONB metadata support

### Data Volume Estimate

Run assessment queries before migration:

```bash
# Count records in each table
imi stats --json

# Expected output structure:
# {
#   "repositories": 25,
#   "worktrees": 47,
#   "worktree_types": 6
# }
```

Typical small installation: <100 projects, <500 worktrees, <5000 activities

## Migration Phases

### Phase 1: PostgreSQL Setup

**Goal:** Provision PostgreSQL and validate schema

**Steps:**

1. **Install PostgreSQL 14+**
   ```bash
   # macOS
   brew install postgresql@14
   brew services start postgresql@14

   # Linux
   sudo apt-get install postgresql-14
   sudo systemctl start postgresql
   ```

2. **Create Database and User**
   ```bash
   createdb imi_registry

   psql imi_registry <<EOF
   CREATE USER imi_app WITH PASSWORD 'secure_password_here';
   GRANT ALL PRIVILEGES ON DATABASE imi_registry TO imi_app;
   \c imi_registry
   GRANT ALL ON SCHEMA public TO imi_app;
   EOF
   ```

3. **Run Migrations**
   ```bash
   cd /home/delorenj/code/iMi/trunk-main

   psql imi_registry < migrations/001_create_schema.sql
   psql imi_registry < migrations/002_functions_and_helpers.sql

   # Verify
   psql imi_registry -c "SELECT * FROM get_registry_stats();"
   ```

4. **Validate Schema**
   ```bash
   # Check tables exist
   psql imi_registry -c "\dt"

   # Check functions exist
   psql imi_registry -c "\df"

   # Check indexes exist
   psql imi_registry -c "\di"
   ```

**Rollback:** Drop database and start over

**Duration:** M effort (includes installation, configuration, validation)

### Phase 2: Data Extraction from SQLite

**Goal:** Export all data from SQLite in PostgreSQL-compatible format

**Steps:**

1. **Create Extraction Script**

   `/home/delorenj/code/iMi/trunk-main/scripts/extract-sqlite-data.sh`:

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

   # Extract worktrees
   sqlite3 "$SQLITE_DB" <<EOF
   .headers on
   .mode csv
   .output ${OUTPUT_DIR}/worktrees.csv
   SELECT
       w.id as worktree_id,
       r.id as project_id,
       wt.id as type_id,
       w.name,
       w.branch_name,
       w.path,
       NULL as agent_id,
       0 as has_uncommitted_changes,
       0 as uncommitted_files_count,
       0 as ahead_of_trunk,
       0 as behind_trunk,
       NULL as last_commit_hash,
       NULL as last_commit_message,
       NULL as last_sync_at,
       NULL as merged_at,
       NULL as merged_by,
       NULL as merge_commit_hash,
       '{}' as metadata,
       w.created_at,
       w.updated_at,
       1 as active
   FROM worktrees w
   JOIN repositories r ON r.name = w.repo_name
   JOIN worktree_types wt ON wt.name = w.type;
   EOF

   echo "Extraction complete: $OUTPUT_DIR"
   ls -lh "$OUTPUT_DIR"
   ```

2. **Run Extraction**
   ```bash
   chmod +x scripts/extract-sqlite-data.sh
   ./scripts/extract-sqlite-data.sh
   ```

3. **Validate CSV Files**
   ```bash
   head /tmp/imi-migration/projects.csv
   head /tmp/imi-migration/worktrees.csv

   # Check record counts match
   wc -l /tmp/imi-migration/*.csv
   ```

**Rollback:** Re-run extraction script if data looks incorrect

**Duration:** XS effort (script execution takes seconds)

### Phase 3: Data Import to PostgreSQL

**Goal:** Load extracted data into PostgreSQL with proper UUID mapping

**Steps:**

1. **Create Import Script**

   `/home/delorenj/code/iMi/trunk-main/scripts/import-postgres-data.sh`:

   ```bash
   #!/usr/bin/env bash
   set -euo pipefail

   INPUT_DIR="/tmp/imi-migration"
   DB="imi_registry"

   # Import projects
   psql "$DB" <<EOF
   \copy projects (id, name, remote_origin, default_branch, trunk_path, description, metadata, created_at, updated_at, active)
   FROM '${INPUT_DIR}/projects.csv'
   WITH (FORMAT csv, HEADER true, DELIMITER ',');
   EOF

   # Import worktrees
   psql "$DB" <<EOF
   \copy worktrees (id, project_id, type_id, name, branch_name, path, agent_id, has_uncommitted_changes, uncommitted_files_count, ahead_of_trunk, behind_trunk, last_commit_hash, last_commit_message, last_sync_at, merged_at, merged_by, merge_commit_hash, metadata, created_at, updated_at, active)
   FROM '${INPUT_DIR}/worktrees.csv'
   WITH (FORMAT csv, HEADER true, DELIMITER ',', NULL 'NULL');
   EOF

   # Verify counts
   echo "Projects imported:"
   psql "$DB" -c "SELECT COUNT(*) FROM projects;"

   echo "Worktrees imported:"
   psql "$DB" -c "SELECT COUNT(*) FROM worktrees;"

   # Validate foreign keys
   echo "Validating foreign key integrity:"
   psql "$DB" -c "
   SELECT
       w.name AS worktree_name,
       w.project_id,
       p.id AS project_exists
   FROM worktrees w
   LEFT JOIN projects p ON p.id = w.project_id
   WHERE p.id IS NULL;
   "
   ```

2. **Run Import**
   ```bash
   chmod +x scripts/import-postgres-data.sh
   ./scripts/import-postgres-data.sh
   ```

3. **Validate Data Integrity**
   ```bash
   # Check for orphaned worktrees (should return 0)
   psql imi_registry -c "
   SELECT COUNT(*)
   FROM worktrees w
   LEFT JOIN projects p ON p.id = w.project_id
   WHERE p.id IS NULL;
   "

   # Check for duplicate projects (should return 0)
   psql imi_registry -c "
   SELECT remote_origin, COUNT(*)
   FROM projects
   WHERE active = TRUE
   GROUP BY remote_origin
   HAVING COUNT(*) > 1;
   "

   # Sample data validation
   psql imi_registry -c "SELECT * FROM v_projects_summary LIMIT 5;"
   psql imi_registry -c "SELECT * FROM v_worktrees_detail LIMIT 5;"
   ```

**Rollback:** `psql imi_registry < migrations/999_rollback.sql` and re-run Phase 1

**Duration:** XS effort (import takes seconds)

### Phase 4: Git State Synchronization

**Goal:** Populate in-flight work tracking fields from current git state

**Steps:**

1. **Create Git Sync Script**

   `/home/delorenj/code/iMi/trunk-main/scripts/sync-git-state.sh`:

   ```bash
   #!/usr/bin/env bash
   set -euo pipefail

   DB="imi_registry"

   # Get all active worktrees
   psql "$DB" -t -A -F'|' -c "
   SELECT id, path
   FROM worktrees
   WHERE active = TRUE;
   " | while IFS='|' read -r worktree_id path; do
       if [[ ! -d "$path" ]]; then
           echo "WARNING: Worktree path doesn't exist: $path"
           continue
       fi

       cd "$path"

       # Check uncommitted changes
       uncommitted_count=$(git status --porcelain | wc -l)
       has_uncommitted=$([[ $uncommitted_count -gt 0 ]] && echo "TRUE" || echo "FALSE")

       # Check ahead/behind trunk
       trunk_branch=$(git rev-parse --abbrev-ref origin/HEAD | sed 's@origin/@@')
       ahead=$(git rev-list --count HEAD ^origin/"$trunk_branch" 2>/dev/null || echo 0)
       behind=$(git rev-list --count origin/"$trunk_branch" ^HEAD 2>/dev/null || echo 0)

       # Get last commit info
       last_commit_hash=$(git rev-parse HEAD)
       last_commit_message=$(git log -1 --pretty=%s)

       # Update database
       psql "$DB" <<EOF
   UPDATE worktrees
   SET has_uncommitted_changes = $has_uncommitted,
       uncommitted_files_count = $uncommitted_count,
       ahead_of_trunk = $ahead,
       behind_trunk = $behind,
       last_commit_hash = '$last_commit_hash',
       last_commit_message = '$last_commit_message',
       last_sync_at = NOW()
   WHERE id = '$worktree_id';
   EOF

       echo "Synced: $path (uncommitted=$uncommitted_count, ahead=$ahead, behind=$behind)"
   done

   echo "Git state synchronization complete"
   ```

2. **Run Git Sync**
   ```bash
   chmod +x scripts/sync-git-state.sh
   ./scripts/sync-git-state.sh
   ```

3. **Verify In-Flight Work Tracking**
   ```bash
   psql imi_registry -c "SELECT * FROM get_inflight_work();"
   psql imi_registry -c "SELECT * FROM v_inflight_work;"
   ```

**Rollback:** Git state can be re-synced at any time without data loss

**Duration:** S effort (depends on number of worktrees, typically <1 minute)

### Phase 5: Application Cutover

**Goal:** Switch iMi CLI and MCP tools to PostgreSQL

**Steps:**

1. **Update Configuration**

   Edit `~/.config/imi/config.toml`:

   ```toml
   [database]
   # Old SQLite config (comment out)
   # type = "sqlite"
   # path = "~/.local/share/imi/database.db"

   # New PostgreSQL config
   type = "postgresql"
   url = "postgresql://imi_app:secure_password_here@localhost:5432/imi_registry"
   pool_size = 10
   connection_timeout = 30
   ```

2. **Update Environment Variables**

   Add to `~/.zshrc`:

   ```bash
   export IMI_DATABASE_TYPE="postgresql"
   export IMI_DATABASE_URL="postgresql://imi_app:secure_password_here@localhost:5432/imi_registry"
   ```

3. **Test CLI Operations**
   ```bash
   # Test read operations
   imi list --json
   imi types list --json

   # Test write operations (on test project)
   imi project create --name test-migration --remote git@github.com:test/test.git
   imi add feat test-worktree

   # Verify in database
   psql imi_registry -c "SELECT * FROM projects WHERE name = 'test-migration';"
   psql imi_registry -c "SELECT * FROM worktrees WHERE name = 'feat-test-worktree';"

   # Cleanup test data
   psql imi_registry -c "DELETE FROM projects WHERE name = 'test-migration';"
   ```

4. **Test MCP Tools**

   Update `~/Library/Application Support/Claude/claude_desktop_config.json`:

   ```json
   {
     "mcpServers": {
       "imi": {
         "command": "uv",
         "args": ["run", "imi", "mcp"],
         "env": {
           "IMI_DATABASE_URL": "postgresql://imi_app:secure_password_here@localhost:5432/imi_registry"
         }
       }
     }
   }
   ```

   Restart Claude Desktop and test MCP tools.

5. **Monitor for Issues**
   ```bash
   # Watch PostgreSQL logs
   tail -f /usr/local/var/log/postgresql@14.log

   # Monitor connections
   psql imi_registry -c "SELECT * FROM pg_stat_activity WHERE datname = 'imi_registry';"

   # Check for slow queries
   psql imi_registry -c "SELECT * FROM pg_stat_statements ORDER BY mean_exec_time DESC LIMIT 10;"
   ```

**Rollback:** Revert config changes, restart services

**Duration:** S effort (configuration + testing)

### Phase 6: Dual-Write Period (Optional)

**Goal:** Write to both SQLite and PostgreSQL for safety during transition

**When to Use:** If rollback risk is high or downtime is unacceptable

**Steps:**

1. **Enable Dual-Write Mode**

   Edit `~/.config/imi/config.toml`:

   ```toml
   [database]
   type = "postgresql"
   url = "postgresql://imi_app:secure_password_here@localhost:5432/imi_registry"

   [database.fallback]
   enabled = true
   type = "sqlite"
   path = "~/.local/share/imi/database.db"
   ```

2. **Monitor Sync**

   Log all write operations and verify both databases receive updates.

3. **Duration**

   Run dual-write for 1-7 days depending on confidence level.

4. **Disable SQLite**

   Once confident, remove fallback configuration and archive SQLite database.

**Duration:** Variable (1-7 days monitoring)

### Phase 7: SQLite Archival

**Goal:** Archive old SQLite database for emergency rollback

**Steps:**

1. **Create Archive**
   ```bash
   mkdir -p ~/backups/imi-migration-$(date +%Y%m%d)

   # Copy SQLite database
   cp ~/.local/share/imi/database.db ~/backups/imi-migration-$(date +%Y%m%d)/

   # Copy extraction CSVs
   cp /tmp/imi-migration/*.csv ~/backups/imi-migration-$(date +%Y%m%d)/

   # Create metadata file
   cat > ~/backups/imi-migration-$(date +%Y%m%d)/README.md <<EOF
   # iMi SQLite to PostgreSQL Migration Archive

   **Migration Date:** $(date)
   **SQLite Version:** $(sqlite3 --version)
   **PostgreSQL Version:** $(psql --version)

   ## File Contents
   - database.db: Original SQLite database
   - projects.csv: Extracted projects data
   - worktrees.csv: Extracted worktrees data

   ## Record Counts
   - Projects: $(sqlite3 ~/.local/share/imi/database.db "SELECT COUNT(*) FROM repositories;")
   - Worktrees: $(sqlite3 ~/.local/share/imi/database.db "SELECT COUNT(*) FROM worktrees;")

   ## Rollback Procedure
   See /home/delorenj/code/iMi/trunk-main/docs/migration-sqlite-to-postgres.md
   EOF

   # Compress archive
   tar -czf ~/backups/imi-migration-$(date +%Y%m%d).tar.gz \
       -C ~/backups imi-migration-$(date +%Y%m%d)

   echo "Archive created: ~/backups/imi-migration-$(date +%Y%m%d).tar.gz"
   ```

2. **Verify Archive**
   ```bash
   tar -tzf ~/backups/imi-migration-$(date +%Y%m%d).tar.gz
   ```

3. **Remove Original SQLite Files** (after 30 days)
   ```bash
   # DO NOT do this immediately - wait at least 30 days
   # rm -rf ~/.local/share/imi/database.db
   ```

**Duration:** XS effort (backup creation)

## Rollback Procedures

### Emergency Rollback (Within 24 hours)

If critical issues discovered immediately after cutover:

1. **Revert Configuration**
   ```bash
   # Edit ~/.config/imi/config.toml
   [database]
   type = "sqlite"
   path = "~/.local/share/imi/database.db"
   ```

2. **Restart Services**
   ```bash
   # If running as systemd service
   systemctl restart imi-api

   # If using MCP, restart Claude Desktop
   ```

3. **Verify SQLite Still Works**
   ```bash
   imi list --json
   ```

**Data Loss Risk:** Any writes to PostgreSQL after cutover will be lost

### Full Rollback (After 24 hours)

If PostgreSQL has been running and accumulating new data:

1. **Extract New Data from PostgreSQL**
   ```bash
   # Export new projects created since migration
   psql imi_registry -c "
   \copy (
       SELECT * FROM projects
       WHERE created_at > '2026-01-21 14:00:00'
   ) TO '/tmp/new-projects.csv' WITH CSV HEADER;
   "

   # Export new worktrees created since migration
   psql imi_registry -c "
   \copy (
       SELECT * FROM worktrees
       WHERE created_at > '2026-01-21 14:00:00'
   ) TO '/tmp/new-worktrees.csv' WITH CSV HEADER;
   "
   ```

2. **Manually Import to SQLite**

   This requires custom SQL since schemas don't match. Not recommended.

3. **Alternative: Keep PostgreSQL, Fix Issues**

   Rollback should be last resort. Prefer fixing PostgreSQL issues.

## Validation Checklist

After each phase, validate:

- [ ] Record counts match between source and destination
- [ ] Foreign key relationships intact (no orphaned records)
- [ ] UUID references work correctly
- [ ] CLI operations succeed (list, create, update, delete)
- [ ] MCP tools respond correctly
- [ ] Git state synchronization working
- [ ] Performance acceptable (<100ms for typical queries)
- [ ] No error logs in PostgreSQL
- [ ] Backup created and verified

## Performance Benchmarks

Compare before/after migration:

```bash
# SQLite baseline
time imi list --json > /dev/null

# PostgreSQL comparison
time imi list --json > /dev/null

# Expected results:
# SQLite: ~50ms (local file access)
# PostgreSQL: ~20ms (better indexing, no file locking)
```

## Post-Migration Optimization

After successful migration:

1. **Analyze Tables**
   ```bash
   psql imi_registry -c "ANALYZE projects;"
   psql imi_registry -c "ANALYZE worktrees;"
   psql imi_registry -c "ANALYZE agent_activities;"
   ```

2. **Set Up Automated Maintenance**

   Create cron job:
   ```bash
   # /etc/cron.daily/imi-maintenance
   #!/usr/bin/env bash
   psql imi_registry -c "SELECT cleanup_old_activities(90);"
   psql imi_registry -c "SELECT prune_inactive_worktrees(30);"
   psql imi_registry -c "SELECT maintenance_vacuum();"
   ```

3. **Configure Backups**
   ```bash
   # /etc/cron.daily/imi-backup
   #!/usr/bin/env bash
   pg_dump imi_registry | gzip > ~/backups/imi-registry-$(date +%Y%m%d).sql.gz

   # Keep last 30 days
   find ~/backups -name "imi-registry-*.sql.gz" -mtime +30 -delete
   ```

4. **Monitor Performance**

   Enable `pg_stat_statements`:
   ```sql
   CREATE EXTENSION IF NOT EXISTS pg_stat_statements;

   -- Check slow queries weekly
   SELECT
       query,
       calls,
       total_exec_time,
       mean_exec_time,
       max_exec_time
   FROM pg_stat_statements
   ORDER BY mean_exec_time DESC
   LIMIT 20;
   ```

## Timeline Summary

**Estimated Total Effort:**

- Phase 1 (PostgreSQL Setup): M effort
- Phase 2 (Data Extraction): XS effort
- Phase 3 (Data Import): XS effort
- Phase 4 (Git Sync): S effort
- Phase 5 (Cutover): S effort
- Phase 6 (Dual-Write, optional): Variable (1-7 days monitoring)
- Phase 7 (Archival): XS effort

**Total Active Work:** M-L effort (not counting dual-write monitoring period)

**Recommended Schedule:**

1. Day 1: Phases 1-3 (setup + data migration)
2. Day 2: Phases 4-5 (git sync + cutover)
3. Days 3-9: Phase 6 (optional dual-write monitoring)
4. Day 10: Phase 7 (archival)

## Support Resources

**Troubleshooting:**
- PostgreSQL logs: `/usr/local/var/log/postgresql@14.log`
- iMi logs: `~/.local/share/imi/logs/`
- Schema reference: `/home/delorenj/code/iMi/trunk-main/migrations/README.md`

**Rollback Scripts:**
- `/home/delorenj/code/iMi/trunk-main/migrations/999_rollback.sql`
- SQLite archive: `~/backups/imi-migration-YYYYMMDD.tar.gz`

**Getting Help:**
- Schema issues: Review `/home/delorenj/code/iMi/trunk-main/migrations/001_create_schema.sql`
- Function issues: Review `/home/delorenj/code/iMi/trunk-main/migrations/002_functions_and_helpers.sql`
- Examples: `/home/delorenj/code/iMi/trunk-main/migrations/examples.sql`

---

**Maintained by**: iMi Development Team
**Last Updated**: 2026-01-21
**Migration Status**: Ready for execution
