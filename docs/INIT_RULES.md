# iMi Init Command Rules

## Two-Root Architecture

### Global iMi Root
- **Purpose**: Where agents get cloned and managed
- **Location**: User preference (default: `~/.iMi`, preferred: `/home/delorenj/code`)
- **Storage**: `~/.config/iMi/config.toml` → `root_path`
- **Scope**: System-wide, one per host

### Repository Path
- **Purpose**: Individual repository being initialized
- **Location**: The actual repository directory
- **Storage**: Database `repositories.path` field
- **Scope**: Per repository

## Path Detection Logic

### When in Trunk Directory (`trunk-*`)
```rust
// Current: /path/to/repo/trunk-main
// Repository path: /path/to/repo (parent)
// Repository name: repo
let repo_dir = current_dir.parent()?;
let repo_name = repo_dir.file_name()?.to_str()?;
```

### When at Repository Root
```rust
// Current: /path/to/repo
// Repository path: /path/to/repo (current)
// Repository name: repo
let repo_name = current_dir.file_name()?.to_str()?;
```

## Database Operations

### Repository Registration
- Check existence: `database.get_repository(&repo_name)`
- Create only if not exists
- Store repository path (not parent or grandparent)

### Worktree Registration
- Only for trunk directories
- Extract branch from directory name (`trunk-main` → `main`)
- Store current directory path for worktree

## Configuration Management

### New Installation
- Set `root_path = "/home/delorenj/code"`
- Create default config structure

### Force Flag Behavior
- Override existing configuration
- Update global iMi root path
- Preserve database integrity

### No Force Flag
- Skip if config exists
- Show current configuration
- Exit gracefully (not an error)

## Validation Rules

1. **Never go up more than one level** from current directory
2. **Global root ≠ Repository path** (two separate concepts)
3. **Database stores repository paths**, not parent directories
4. **Config stores global root**, not repository-specific paths
5. **Trunk detection**: Directory name starts with `trunk-`