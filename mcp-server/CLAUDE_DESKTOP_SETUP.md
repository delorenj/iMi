# Claude Desktop Integration Guide

## Prerequisites

1. **iMi CLI installed and in PATH**
   ```bash
   which imi  # Should return /home/delorenj/.cargo/bin/imi
   ```

2. **GitHub authentication configured** (for project creation)
   ```bash
   gh auth status
   # or
   export GITHUB_TOKEN=<your-token>
   ```

3. **UV installed** (for running the MCP server)
   ```bash
   which uv  # Should return UV installation path
   ```

## Installation

1. **Locate your Claude Desktop config file:**
   - Linux: `~/.config/Claude/claude_desktop_config.json`
   - macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`

2. **Add iMi MCP server configuration:**

```json
{
  "mcpServers": {
    "imi": {
      "command": "uv",
      "args": [
        "--directory",
        "/home/delorenj/code/iMi/feat-mcp-server/mcp-server",
        "run",
        "imi-mcp"
      ],
      "env": {
        "IMI_MCP_IMI_BINARY_PATH": "imi",
        "IMI_MCP_TIMEOUT_SECONDS": "30",
        "IMI_MCP_LOG_LEVEL": "INFO"
      }
    }
  }
}
```

3. **Restart Claude Desktop**

## Verification

After restarting Claude Desktop, you should see the iMi MCP server in the available tools.

Ask Claude:
- "List all my worktrees"
- "Create a new feature worktree called user-authentication"
- "Show me available worktree types"
- "Create a new project with concept: A FastAPI backend for managing tasks"

## Troubleshooting

### Server not appearing in Claude Desktop

1. Check Claude Desktop logs:
   ```bash
   # Linux
   tail -f ~/.config/Claude/logs/mcp*.log

   # macOS
   tail -f ~/Library/Logs/Claude/mcp*.log
   ```

2. Verify server starts manually:
   ```bash
   cd /home/delorenj/code/iMi/feat-mcp-server/mcp-server
   uv run imi-mcp
   ```
   Should see: "iMi MCP server initialized..."

### Tools not working

1. **Check iMi binary is accessible:**
   ```bash
   which imi
   imi --version
   ```

2. **Check environment variables:**
   ```bash
   echo $GITHUB_TOKEN  # For project creation
   ```

3. **Test iMi commands directly:**
   ```bash
   imi list --json
   imi types list --json
   ```

### Permission errors

Ensure the mcp-server directory is readable:
```bash
ls -la /home/delorenj/code/iMi/feat-mcp-server/mcp-server
```

## Available Tools

Once connected, Claude Desktop will have access to 10 iMi tools:

1. **create_worktree** - Create new worktrees (feat, fix, aiops, devops, custom)
2. **create_review_worktree** - Create PR review worktrees
3. **list_worktrees** - List all worktrees and repositories
4. **show_status** - Show worktree git status
5. **create_project** - Bootstrap new projects with GitHub integration
6. **navigate_worktree** - Find worktree paths via fuzzy search
7. **remove_worktree** - Remove worktrees with branch cleanup options
8. **sync_worktrees** - Sync database with git state
9. **prune_worktrees** - Clean up stale worktree references
10. **list_types** - List available worktree types

## Example Interactions

```
You: "Create a feature worktree for user authentication"
Claude: [Uses create_worktree tool]
→ Worktree created at /home/delorenj/code/myproject/feat-user-authentication

You: "List all my active worktrees"
Claude: [Uses list_worktrees tool]
→ Shows all worktrees across all repositories

You: "Create a new FastAPI project called TaskMaster"
Claude: [Uses create_project tool]
→ GitHub repo created, boilerplate generated, git initialized
```

## Architecture

```
Claude Desktop
    ↓
MCP Protocol (stdio)
    ↓
FastMCP Server (Python/UV)
    ↓
Subprocess Execution
    ↓
iMi CLI (Rust binary)
    ↓
Git Operations + Database
```

## Development Mode

For development and testing, run the server manually:

```bash
cd /home/delorenj/code/iMi/feat-mcp-server/mcp-server

# Install dependencies
uv sync

# Run server (stdio transport)
uv run imi-mcp

# Run with debug logging
IMI_MCP_LOG_LEVEL=DEBUG uv run imi-mcp
```

## Next Steps

After successful Claude Desktop integration:

1. Test all 10 tools via Claude Desktop interface
2. Document any edge cases or issues
3. Add pytest test suite
4. Create Claude skill file (Phase 3)
