# iMi CLI Refactor & Project Creation

## Progress

### Phase 1: JSON Output Support ✅ COMPLETE
- Added global `--json` flag to CLI (src/cli.rs:22)
- Implemented JsonResponse helper structure (src/main.rs:29-58)
- Updated all 16 command handlers to support JSON output
- Tested and verified JSON mode works across all commands
- Non-JSON (colored terminal) mode preserved and working

### Phase 1.5: CLI Architecture Refactor ✅ COMPLETE
**Database-Driven Worktree Types**
- Created `worktree_types` table in database (src/database.rs:106-125)
- Added `WorktreeType` struct with full CRUD operations (src/database.rs:50-751)
- Implemented built-in type seeding: feat, fix, aiops, devops, review
- Protection against removing built-in types

**Hierarchical Command Structure**
- New `imi add <type> <name>` command replaces flat structure (src/cli.rs:27-43)
- New `imi types` subcommand for type management (src/cli.rs:45-47, 254-284)
  - `imi types list` - Show all available types with metadata
  - `imi types add <name>` - Add custom worktree types
  - `imi types remove <name>` - Remove custom types (builtin protection)
- Backwards compatibility: deprecated `feat`, `fix`, `aiops`, `devops` commands still work

**Custom Worktree Support**
- Implemented `create_custom_worktree()` method (src/worktree.rs:103-131)
- Types defined in database control branch/worktree naming conventions
- Full JSON output support for all type management commands

**Testing Verified**
- ✅ List builtin types shows all 5 default types
- ✅ Add custom type with auto-generated prefixes
- ✅ Remove custom type (and protection for builtins)
- ✅ JSON output mode for all type commands
- ✅ Backwards compatibility with deprecated commands

### Next Steps
**Phase 1.6: Enhanced Command Features**
- Implement implicit type detection for `imi merge <name>` command
- Enhanced `imi project create` with registry-first logic
- Enhanced `imi project clone` with registry-first logic

### Future: Phase 2 - FastMCP Server
Ready to begin MCP server implementation per implementation plan.

## Usage

### New Hierarchical Commands

```bash
# List available worktree types
imi types list
imi types list --json

# Add custom worktree type
imi types add experiment --description "Experimental features"
imi types add prototype --branch-prefix "proto/" --worktree-prefix "proto-"

# Remove custom type (builtin types protected)
imi types remove experiment

# Create worktrees using unified add command
imi add feat "user-authentication"
imi add fix "login-bug"
imi add experiment "new-ui-concept"

# Old commands still work (deprecated)
imi feat "user-authentication"  # Shows deprecation warning
imi fix "login-bug"
```

### Project Creation

```
imi project create [--concept|-c] "An android app that helps you plan lego builds. Use Flutter." [--name|-n] "LegoJoe"

# Use a markdown doc to describe concept
iMi prooject create [--prd|-p] /some/markdown.md #if name is null, will look to prd or concept for explicit name and fallback to deciding on its own.

# Or use Bloodbank command
imi.project.create

{
  "concept": "Blah Blah",
  "name": "SomeProjectName"
}

# Can also use arbitrary structured json data to describe anything
# Vague stuff will just be guessed. Wrong? Who cares! It's awesome.

{
 "name": "MyProject",
 "api": "FastAPI",
 "frontend": "react dashboard",
 "mise-tasks": [
  "hello-world"
  ]
}
```

## Instructions

Use bmad workflow.
Spawn a staff level architect to answer questions for me.
Best guesses are fine.
Continue from phase to phase until acceptance criteria are met

## Acceptance Criteria

Above example commands result in:

- new gh repo for each `create`
- works via CLI or Bloodbank command
- gh repo contains the base boilerplate for whatever stack the repo requires
  - Bootstrapped with mise.toml, .mise/tasks
  - Python apps bootstrapped with UV, hatchling packaging
  - React apps bootstrapped with `bun`, Typescript, tailwindcss, vite, shadcn
  - Containerization with docker compose, `compose.yml`.
  - If postgres required, use native postgres on 192.168.1.12:5432 $DEFAULT_USERNAME:$DEFAULT_PASSWORD
  - If redis required, use native passwordless redis on 192.168.1.12:6743
  - If qdrant required, use qdrant.delo.sh
  - Sensible readme added
  - Public repo, unless otherwise specified
