#!/usr/bin/env bash
# =============================================================================
# register-projects-from-filesystem.sh
# =============================================================================
# Scans filesystem for .iMi directories and registers projects in PostgreSQL
#
# IDEMPOTENT: Safe to run multiple times. Uses ON CONFLICT for deduplication.
#
# Usage:
#   ./scripts/register-projects-from-filesystem.sh              # Default: ~/code
#   ./scripts/register-projects-from-filesystem.sh /path/to/scan
#   ./scripts/register-projects-from-filesystem.sh --dry-run    # Preview only
#
# Environment:
#   DATABASE_URL  - PostgreSQL connection string (default: from psql-imi.sh)
#   MAX_DEPTH     - How deep to search for .iMi directories (default: 4)
# =============================================================================

set -euo pipefail

# -----------------------------------------------------------------------------
# Configuration
# -----------------------------------------------------------------------------
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Database connection (use psql-imi.sh connection params)
export PGHOST="${PGHOST:-192.168.1.12}"
export PGPORT="${PGPORT:-5432}"
export PGDATABASE="${PGDATABASE:-imi}"
export PGUSER="${PGUSER:-imi}"
export PGPASSWORD="${PGPASSWORD:-imi_dev_password_2026}"

# Scan settings
MAX_DEPTH="${MAX_DEPTH:-4}"
DRY_RUN=false
SCAN_ROOT="${HOME}/code"

# Parse args first
for arg in "$@"; do
    case $arg in
        --dry-run)
            DRY_RUN=true
            ;;
        --help|-h)
            echo "Usage: $0 [scan_root] [--dry-run]"
            echo ""
            echo "Scans filesystem for .iMi directories and registers projects in PostgreSQL."
            echo ""
            echo "Options:"
            echo "  scan_root   Directory to scan (default: ~/code)"
            echo "  --dry-run   Preview what would be registered without making changes"
            exit 0
            ;;
        *)
            if [[ -d "$arg" ]]; then
                SCAN_ROOT="$arg"
            fi
            ;;
    esac
done

# -----------------------------------------------------------------------------
# Logging (all go to stderr to not interfere with data piping)
# -----------------------------------------------------------------------------
log_info() { echo -e "\033[0;34m[INFO]\033[0m $1" >&2; }
log_success() { echo -e "\033[0;32m[OK]\033[0m $1" >&2; }
log_warn() { echo -e "\033[0;33m[WARN]\033[0m $1" >&2; }
log_error() { echo -e "\033[0;31m[ERROR]\033[0m $1" >&2; }
log_dry() { echo -e "\033[0;35m[DRY-RUN]\033[0m $1" >&2; }

# -----------------------------------------------------------------------------
# Database helpers
# -----------------------------------------------------------------------------
db_query() {
    psql -t -A -F'|' -c "$1" 2>/dev/null
}

db_exec() {
    if $DRY_RUN; then
        log_dry "Would execute: $1"
        return 0
    fi
    psql -c "$1" >/dev/null 2>&1
}

# Test database connection
test_connection() {
    if ! psql -c "SELECT 1" >/dev/null 2>&1; then
        log_error "Cannot connect to PostgreSQL at ${PGHOST}:${PGPORT}/${PGDATABASE}"
        log_error "Ensure PostgreSQL is running and credentials are correct"
        exit 1
    fi
    log_success "Connected to PostgreSQL"
}

# -----------------------------------------------------------------------------
# Project discovery
# -----------------------------------------------------------------------------
discover_projects() {
    local scan_root="$1"

    log_info "Scanning for .iMi directories in: $scan_root (max depth: $MAX_DEPTH)"

    # Find all .iMi directories
    find "$scan_root" -maxdepth "$MAX_DEPTH" -name ".iMi" -type d 2>/dev/null | while read -r imi_dir; do
        local project_dir
        project_dir=$(dirname "$imi_dir")

        # Skip if this is a worktree subdirectory (has .iMi but no trunk-*)
        # We only want cluster hubs (parent directories of trunk-*)
        local trunk_path
        trunk_path=$(find "$project_dir" -maxdepth 1 -type d -name "trunk-*" 2>/dev/null | head -1)

        if [[ -z "$trunk_path" ]]; then
            # This .iMi is inside a worktree, not a cluster hub
            continue
        fi

        # Get git remote from trunk
        local remote_url
        remote_url=$(cd "$trunk_path" && git remote get-url origin 2>/dev/null || echo "")

        if [[ -z "$remote_url" ]]; then
            log_warn "Skipping $project_dir - no git remote configured"
            continue
        fi

        # Normalize remote URL to SSH format for constraint validation
        # git@github.com:user/repo.git
        if [[ "$remote_url" =~ ^https://github\.com/(.+)/(.+)(\.git)?$ ]]; then
            remote_url="git@github.com:${BASH_REMATCH[1]}/${BASH_REMATCH[2]}.git"
        fi

        # Ensure .git suffix
        if [[ ! "$remote_url" =~ \.git$ ]]; then
            remote_url="${remote_url}.git"
        fi

        # Get project name from directory
        local project_name
        project_name=$(basename "$project_dir")

        # Get default branch from trunk directory name
        local default_branch
        default_branch=$(basename "$trunk_path" | sed 's/trunk-//')

        echo "$project_name|$trunk_path|$remote_url|$default_branch"
    done
}

# -----------------------------------------------------------------------------
# Worktree discovery
# -----------------------------------------------------------------------------
discover_worktrees() {
    local project_dir="$1"
    local project_id="$2"

    # Find all directories that look like worktrees (not trunk, not .iMi, not hidden)
    find "$project_dir" -maxdepth 1 -type d ! -name ".*" ! -name "trunk-*" 2>/dev/null | while read -r worktree_path; do
        [[ "$worktree_path" == "$project_dir" ]] && continue

        local worktree_name
        worktree_name=$(basename "$worktree_path")

        # Must have .git or be a git worktree
        if [[ ! -d "$worktree_path/.git" ]] && [[ ! -f "$worktree_path/.git" ]]; then
            continue
        fi

        # Get branch name
        local branch_name
        branch_name=$(cd "$worktree_path" && git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "")

        [[ -z "$branch_name" ]] && continue

        # Determine worktree type from name prefix
        local worktree_type="custom"
        case "$worktree_name" in
            feat-*) worktree_type="feat" ;;
            fix-*) worktree_type="fix" ;;
            aiops-*) worktree_type="aiops" ;;
            devops-*) worktree_type="devops" ;;
            review-*) worktree_type="review" ;;
        esac

        echo "$worktree_name|$branch_name|$worktree_type|$worktree_path"
    done
}

# -----------------------------------------------------------------------------
# Registration
# -----------------------------------------------------------------------------
register_project() {
    local name="$1"
    local trunk_path="$2"
    local remote_url="$3"
    local default_branch="$4"

    if $DRY_RUN; then
        log_dry "Would register project: $name"
        log_dry "  trunk_path: $trunk_path"
        log_dry "  remote_url: $remote_url"
        log_dry "  default_branch: $default_branch"
        echo "DRY_RUN_UUID"
        return 0
    fi

    # Use register_project() function - idempotent via ON CONFLICT
    local project_id
    project_id=$(psql -t -A -c "SELECT register_project('$name', '$remote_url', '$default_branch', '$trunk_path', '{}'::jsonb);" 2>/dev/null)

    if [[ -n "$project_id" ]]; then
        echo "$project_id"
    else
        log_error "Failed to register project: $name"
        echo ""
    fi
}

register_worktree() {
    local project_id="$1"
    local worktree_name="$2"
    local branch_name="$3"
    local worktree_type="$4"
    local worktree_path="$5"

    if $DRY_RUN; then
        log_dry "  Would register worktree: $worktree_name ($worktree_type)"
        return 0
    fi

    # Use register_worktree() function - idempotent via ON CONFLICT
    local worktree_id
    worktree_id=$(psql -t -A -c "SELECT register_worktree('$project_id', '$worktree_type', '$worktree_name', '$branch_name', '$worktree_path', NULL, '{}'::jsonb);" 2>&1)

    if [[ "$worktree_id" =~ ^[0-9a-f-]{36}$ ]]; then
        log_success "  Registered worktree: $worktree_name ($worktree_id)"
    elif [[ "$worktree_id" == *"already exists"* ]] || [[ "$worktree_id" == *"duplicate"* ]]; then
        log_info "  Worktree already registered: $worktree_name"
    else
        log_warn "  Could not register worktree: $worktree_name (type: $worktree_type may not exist)"
    fi
}

# -----------------------------------------------------------------------------
# Main
# -----------------------------------------------------------------------------
main() {
    echo "=============================================="
    echo " iMi Project Registry - Filesystem Scanner"
    echo "=============================================="
    echo ""

    if $DRY_RUN; then
        log_warn "DRY RUN MODE - No changes will be made"
        echo ""
    fi

    test_connection

    echo ""
    log_info "Discovering projects..."
    echo ""

    local projects_registered=0
    local projects_skipped=0
    local worktrees_registered=0

    # Discover and register projects
    while IFS='|' read -r name trunk_path remote_url default_branch; do
        [[ -z "$name" ]] && continue

        log_info "Processing: $name"
        log_info "  Trunk: $trunk_path"
        log_info "  Remote: $remote_url"

        local project_id
        project_id=$(register_project "$name" "$trunk_path" "$remote_url" "$default_branch")

        if [[ -n "$project_id" ]] && [[ "$project_id" != "DRY_RUN_UUID" ]]; then
            if [[ "$project_id" =~ ^[0-9a-f-]{36}$ ]]; then
                log_success "Registered project: $name ($project_id)"
                ((projects_registered++))

                # Discover and register worktrees
                local project_dir
                project_dir=$(dirname "$trunk_path")

                while IFS='|' read -r wt_name wt_branch wt_type wt_path; do
                    [[ -z "$wt_name" ]] && continue
                    register_worktree "$project_id" "$wt_name" "$wt_branch" "$wt_type" "$wt_path"
                    ((worktrees_registered++))
                done < <(discover_worktrees "$project_dir" "$project_id")
            else
                log_warn "Unexpected project ID format: $project_id"
                ((projects_skipped++))
            fi
        elif [[ "$project_id" == "DRY_RUN_UUID" ]]; then
            ((projects_registered++))
        else
            ((projects_skipped++))
        fi

        echo ""
    done < <(discover_projects "$SCAN_ROOT")

    echo "=============================================="
    echo " Summary"
    echo "=============================================="
    echo "Projects processed: $projects_registered"
    echo "Projects skipped: $projects_skipped"
    echo "Worktrees discovered: $worktrees_registered"

    if $DRY_RUN; then
        echo ""
        log_warn "DRY RUN - Run without --dry-run to apply changes"
    fi

    # Show current database state
    if ! $DRY_RUN; then
        echo ""
        log_info "Current database state:"
        psql -c "SELECT * FROM get_registry_stats();"
    fi
}

main "$@"
