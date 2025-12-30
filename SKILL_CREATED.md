# iMi Claude Skill Created

## Skill Location

**Installed:** `~/.claude/skills/33god-imi-worktree-management.skill`

## Skill Overview

Comprehensive Claude skill for iMi worktree management integrated with the 33GOD agentic pipeline.

### What It Teaches Claude

**Core Capabilities:**
1. Database-driven worktree type system (feat, fix, aiops, devops, review, custom)
2. Hierarchical command structure (`imi add <type> <name>`)
3. Project creation with GitHub integration and stack-specific boilerplate
4. MCP tool integration (10 tools for Claude Desktop)
5. Opinionated worktree conventions and naming rules

**When It Triggers:**
- Working with git worktrees or feature branches
- Managing worktree types
- Bootstrapping new projects
- iMi CLI operations or MCP tool usage
- 33GOD ecosystem tasks (Bloodbank events, Jelmore sessions, Flume tasks)

### Skill Components

**SKILL.md (Core):**
- Overview of iMi and 33GOD integration
- 5 core capabilities with examples
- Opinionated worktree conventions
- Common workflows (feature dev, PR review, project bootstrapping)
- JSON output mode for programmatic access

**references/mcp-tools-reference.md:**
- Complete schemas for all 10 MCP tools
- Parameter documentation
- Return value examples
- Usage patterns and workflows
- Error handling reference

**references/worktree-conventions.md:**
- Directory structure philosophy
- Naming rules (worktree dir, git branch, trunk)
- Workflow patterns (feature dev, bug fix, PR review, AI ops, DevOps)
- Database schema documentation
- 33GOD ecosystem integration (Bloodbank, Jelmore, Flume)
- Navigation shortcuts and shell aliases

**references/project-creation.md:**
- Stack detection logic (PythonFastAPI, ReactVite, Generic)
- Boilerplate templates for each stack
- GitHub integration workflow
- Input modes (concept, PRD, payload)
- Service configuration patterns (TODO: native postgres/redis/qdrant)

## Usage in Future Sessions

Future Claude instances will automatically detect when to use this skill based on the description triggers. The skill provides:

1. **Procedural Knowledge:** How to use iMi's opinionated conventions effectively
2. **Tool Reference:** Complete MCP tool documentation
3. **Workflow Patterns:** Best practices for feature dev, reviews, project creation
4. **Ecosystem Integration:** How iMi fits into 33GOD pipeline

## Example Triggers

**User asks:**
- "Create a feature worktree for user authentication"
- "What worktree types are available?"
- "Bootstrap a FastAPI project called TaskMaster"
- "Review PR #123"
- "Navigate to the mcp-server worktree"

**Claude will:**
1. Load the skill
2. Use appropriate MCP tools or CLI commands
3. Follow iMi's opinionated conventions
4. Integrate with 33GOD ecosystem patterns

## Skill Development

**Created Using:** `/home/delorenj/.claude/skills/skill-creator`

**Development Steps:**
1. Initialized with `init_skill.py 33god-imi-worktree-management`
2. Wrote SKILL.md with capabilities-based structure
3. Created 3 reference files with detailed documentation
4. Validated and packaged with `package_skill.py`

**Validation:** ✅ Passed all checks

**Package:** `/home/delorenj/.claude/skills/33god-imi-worktree-management.skill`

## Integration Complete

**Phase 1:** JSON Output Support ✅
- Global --json flag across all 16 commands
- Standardized response format

**Phase 1.5:** CLI Architecture Refactor ✅
- Database-driven type system
- Hierarchical commands

**Phase 1.7:** Project Creation ✅
- GitHub integration
- Stack-specific scaffolding

**Phase 2:** FastMCP Server ✅
- 10 MCP tools exposing iMi operations
- stdio transport for Claude Desktop
- Subprocess wrapper with JSON parsing

**Phase 3:** Claude Skill ✅
- Comprehensive skill documentation
- MCP tool reference
- Worktree conventions deep dive
- Project creation patterns

## Next Steps

1. **Test Skill in Claude Desktop:**
   - Restart Claude Desktop after installing MCP server
   - Ask Claude to create a worktree
   - Verify skill triggers and tool usage

2. **Iterate Based on Usage:**
   - Note any missing workflows or edge cases
   - Update reference files as needed
   - Add more examples to SKILL.md

3. **Expand 33GOD Integration:**
   - Document Bloodbank event patterns
   - Add Jelmore session coordination examples
   - Include Flume task lifecycle workflows
