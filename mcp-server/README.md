# iMi MCP Server

FastMCP server exposing iMi worktree management commands as LLM-consumable tools for Claude Desktop and other MCP clients.

## Overview

The iMi MCP server wraps the iMi CLI via subprocess, parses JSON output, and exposes 10 core worktree management operations as MCP tools.

## Installation

```bash
# Install dependencies
cd mcp-server
uv sync

# Run server (stdio transport for Claude Desktop)
uv run imi-mcp
```

## Configuration

Configure via environment variables:

```bash
export IMI_MCP_IMI_BINARY_PATH="imi"          # Path to iMi binary
export IMI_MCP_TIMEOUT_SECONDS=30             # Command timeout
export IMI_MCP_LOG_LEVEL="INFO"               # Logging level
```

Or create `.env` file in mcp-server directory.

## Claude Desktop Integration

Add to `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "imi": {
      "command": "uv",
      "args": ["--directory", "/home/delorenj/code/iMi/mcp-server", "run", "imi-mcp"],
      "env": {
        "IMI_MCP_IMI_BINARY_PATH": "imi"
      }
    }
  }
}
```

## Available Tools

1. `create_worktree` - Create feature/fix/aiops/devops worktrees
2. `create_review_worktree` - Create PR review worktrees
3. `list_worktrees` - List all worktrees and projects
4. `create_project` - Bootstrap new projects with GitHub integration
5. `remove_worktree` - Remove worktrees
6. `navigate_worktree` - Navigate to worktrees (returns path)
7. `sync_worktrees` - Sync database with git state
8. `prune_worktrees` - Clean up stale worktrees
9. `show_status` - Show worktree status
10. `list_types` - List available worktree types

## Architecture

```
mcp-server/
├── src/imi_mcp/
│   ├── config.py          # Pydantic settings
│   ├── schemas.py         # Input/output models
│   ├── cli_wrapper.py     # Subprocess iMi execution
│   ├── tools.py           # MCP tool definitions
│   ├── server.py          # FastMCP initialization
│   └── __main__.py        # Entry point
└── tests/                 # Pytest suite
```

## Development

```bash
# Install dev dependencies
uv sync

# Run tests
uv run pytest

# Lint
uv run ruff check .
```

## Implementation Status

- [x] Project structure
- [x] Configuration management
- [x] Pydantic schemas
- [x] CLI wrapper with JSON parsing
- [x] MCP tools implementation (10 tools)
- [x] Server initialization
- [x] Entry point
- [x] Environment configuration
- [ ] Tests (pytest suite)
- [ ] Claude Desktop integration testing

## Next Steps

1. **Testing**: Write pytest suite for CLI wrapper and tools
2. **Integration**: Test with Claude Desktop
3. **Documentation**: Add tool usage examples and troubleshooting guide
4. **Phase 3**: Create Claude skill file (SKILL.md)

See main implementation plan at `~/.claude/plans/structured-tinkering-stearns.md`
