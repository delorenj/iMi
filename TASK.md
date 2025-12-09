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

### Phase 1.7: Project Create Implementation ✅ SHIPPED
**Core Functionality Verified:**
- ✅ GitHub repository creation via REST API
- ✅ Stack detection (Generic, PythonFastAPI)
- ✅ Boilerplate scaffolding (mise.toml, pyproject.toml, src structure)
- ✅ FastAPI app with health endpoints
- ✅ Git initialization and remote push
- ✅ JSON output mode support
- ✅ Multiple input modes (--concept, --prd, --payload)

**Critical Bugfix:**
- Fixed runtime panic from `--json` parameter collision (global bool vs project String)
- Renamed to `--payload` for structured input

**Known Gaps (Deferred):**
- compose.yml empty (native service configuration for postgres/redis/qdrant pending)
- Stack section in README not populated
- React/Vite stack untested (only Generic and PythonFastAPI verified)

**Decision:** Ship current implementation, gaps are polish not blockers.

### Phase 1.8: Clone Command Implementation
Implement the `clone` command.

## Specification

The `clone` command should create a copy of a given repository from a remote source to the local machine. It should function identically to the `git clone` command, with the following acceptions:

- It should accept only a single argument: the name of the repository to clone
- If `user` is left out and only `name` is provided, it should default to "delorenj"
- The arg can be provided in 3 formats:
  - `name`
  - `user/name`
  - `https://github.com/user/name.git`
- The repo should be cloned into a directory named after the repository, in the `iMi System Path` (set in the `~/.config/iMi/config.toml`)
  - e.g., if cloning `repo-name`, it should create a directory called `repo-name` in the iMi System Path and clone the repo there with the name `trunk-main`
  - In this case (this server), the full path would be `/home/delorenj/code/repo-name/trunk-main`
  - If the target directory already exists then there are two possibilities:
    1. The repo is already an iMi repo, so instead of cloning, it should just `igo` to the directory and log a message indicating that the repo already exists and that it is switching to it.
    2. The repo is not an iMi repo (hasn't been initialized with iMi), so it should log a message indicating that the target directory already exists but it is not an iMi repo and it will convert it into one by initializing it with iMi after cloning. There is a script that does this `/home/delorenj/.config/zshyzsh/scripts/imify.py`. Ideally this script should be implemented as an iMi command in the future, but for now it can be called directly.
    - The script safely rearranged the existing contents of the directory into the iMi structure and then runs `iMi init`
    - `repo-name/` becomes the `repo-name/trunk-main/` directory.

### Next: Phase 2 - FastMCP Server
Ready to begin MCP server implementation per plan at ~/.claude/plans/structured-tinkering-stearns.md

## Usage
