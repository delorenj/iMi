# iMi JSON Output Reference

All iMi commands support JSON output via the global `--json` flag.

## JSON Response Format

All JSON responses follow this structure:

```json
{
  "success": boolean,
  "data": {
    // Command-specific data (only present on success)
  },
  "error": "Error message" // Only present on failure
}
```

## Command Examples

### Feature Worktree Creation

```bash
imi feat my-feature --json
```

**Success Response:**
```json
{
  "success": true,
  "data": {
    "worktree_path": "/home/user/code/myproject/feat-my-feature",
    "worktree_name": "feat-my-feature",
    "message": "Feature worktree created successfully"
  }
}
```

**Error Response:**
```json
{
  "success": false,
  "error": "Branch feat-my-feature already exists"
}
```

### PR Review Worktree

```bash
imi review 123 --json
```

**Success Response:**
```json
{
  "success": true,
  "data": {
    "worktree_path": "/home/user/code/myproject/pr-review-123",
    "pr_number": 123,
    "message": "Review worktree created successfully"
  }
}
```

### Fix Worktree

```bash
imi fix login-bug --json
```

**Success Response:**
```json
{
  "success": true,
  "data": {
    "worktree_path": "/home/user/code/myproject/fix-login-bug",
    "worktree_name": "fix-login-bug",
    "message": "Fix worktree created successfully"
  }
}
```

### Trunk Navigation

```bash
imi trunk --json
```

**Success Response:**
```json
{
  "success": true,
  "data": {
    "worktree_path": "/home/user/code/myproject/trunk-main",
    "message": "Trunk worktree located"
  }
}
```

### Worktree Removal

```bash
imi remove feat-my-feature --json
```

**Success Response:**
```json
{
  "success": true,
  "data": {
    "worktree_name": "feat-my-feature",
    "message": "Worktree removed successfully"
  }
}
```

### Close Worktree

```bash
imi close feat-my-feature --json
```

**Success Response:**
```json
{
  "success": true,
  "data": {
    "message": "Worktree closed successfully",
    "worktree_name": "feat-my-feature",
    "trunk_path": "/home/user/code/myproject/trunk-main"
  }
}
```

### Sync Database

```bash
imi sync --json
```

**Success Response:**
```json
{
  "success": true,
  "data": {
    "message": "Database synced successfully"
  }
}
```

### Prune Stale Worktrees

```bash
imi prune --json
```

**Success Response:**
```json
{
  "success": true,
  "data": {
    "message": "Cleanup complete",
    "dry_run": false
  }
}
```

### Merge Worktree

```bash
imi merge feat-my-feature --json
```

**Success Response:**
```json
{
  "success": true,
  "data": {
    "message": "Worktree merged successfully",
    "worktree_name": "feat-my-feature"
  }
}
```

### Navigate (Go)

```bash
imi go feat-my-feature --json
```

**Success Response:**
```json
{
  "success": true,
  "data": {
    "target_path": "/home/user/code/myproject/feat-my-feature"
  }
}
```

### Worktree Type Management

#### List Types

```bash
imi types list --json
```

**Success Response:**
```json
{
  "success": true,
  "data": {
    "count": 6,
    "types": [
      {
        "name": "feat",
        "branch_prefix": "feat/",
        "worktree_prefix": "feat-",
        "description": "Feature development",
        "is_builtin": true
      },
      {
        "name": "experiment",
        "branch_prefix": "experiment/",
        "worktree_prefix": "experiment-",
        "description": "Experimental features",
        "is_builtin": false
      }
    ]
  }
}
```

#### Add Type

```bash
imi types add experiment --description "Experimental features" --json
```

**Success Response:**
```json
{
  "success": true,
  "data": {
    "message": "Worktree type 'experiment' added successfully!",
    "type": {
      "name": "experiment",
      "branch_prefix": "experiment/",
      "worktree_prefix": "experiment-",
      "description": "Experimental features"
    }
  }
}
```

#### Remove Type

```bash
imi types remove experiment --json
```

**Success Response:**
```json
{
  "success": true,
  "data": {
    "message": "Worktree type 'experiment' removed successfully"
  }
}
```

**Error Response (builtin protection):**
```json
{
  "success": false,
  "error": "Cannot remove builtin worktree type 'feat'"
}
```

### Unified Add Command

```bash
imi add feat "user-auth" --json
```

**Success Response:**
```json
{
  "success": true,
  "data": {
    "worktree_path": "/home/user/code/myproject/feat-user-auth",
    "worktree_name": "feat-user-auth",
    "worktree_type": "feat",
    "message": "feat worktree created successfully"
  }
}
```

### Project Creation

```bash
imi project create --concept "A FastAPI app for task management" --name TaskMaster --json
```

**Success Response:**
```json
{
  "success": true,
  "data": {
    "message": "Project created successfully",
    "project_name": "TaskMaster",
    "project_path": "/home/user/code/TaskMaster",
    "stack": "PythonFastAPI",
    "github_url": "https://github.com/username/TaskMaster"
  }
}
```

### Initialize Repository

```bash
imi init owner/repo --json
```

**Success Response:**
```json
{
  "success": true,
  "data": {
    "message": "Repository initialized successfully",
    "repo": "owner/repo"
  }
}
```

## Commands with Limited JSON Support

Some commands are inherently interactive and have limited JSON support:

### List Command
Currently returns placeholder response. Full implementation pending.

```json
{
  "success": true,
  "data": {
    "message": "List command in JSON mode not yet fully implemented",
    "note": "Use non-JSON mode for detailed listing"
  }
}
```

### Status Command
Currently returns placeholder response. Full implementation pending.

```json
{
  "success": true,
  "data": {
    "message": "Status command in JSON mode not yet fully implemented",
    "note": "Use non-JSON mode for detailed status"
  }
}
```

### Monitor Command
Not supported in JSON mode (requires interactive terminal).

```json
{
  "success": false,
  "error": "Monitor command does not support JSON mode (interactive mode only)"
}
```

## MCP Server Implementation Notes

- All command output is sent to stdout as a single JSON object
- Errors are returned as JSON with `success: false` and an `error` field
- The MCP server should parse stdout and convert to appropriate MCP tool responses
- Commands that modify state (create, remove, merge) should be idempotent where possible
- The `--json` flag is global and works with all commands

## Future Enhancements

The following commands will receive enhanced JSON output in future updates:
- `list` - Will return array of worktrees/projects with metadata
- `status` - Will return structured status data for all worktrees
- `go` - May include additional navigation metadata

See the implementation plan for timeline and details.
