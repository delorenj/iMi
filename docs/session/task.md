# Tasks

With iMi there is no `clone`. Cloning is just something that is necessary when working with a repo. The way to translate iMi intent is to succinctly define why this directory is here. 

## Implement This

e.g. `iMi feat add-logging` means "I want to add logging to this repo. If i'm not in the repo, i would add `--repo <repo_name>` to the command and it would find it's registered location and work there. If its not registered, it would clone delorenj/<repo_name> to the IMI_ROOT and initialize it. We can squeeze a ton of logic into this command because CoC is king. We end up with a behaviorally rich command that is ideal to map to MCP tools since the commands almost describe themselves.

Equivalent using flag aliases:
`iMi f add-logging -r <repo_name>`

Or let iMi name the branch:
`iMi f "implement password reset"`

Even let iMi categorize the task:
`iMi "extract an interface from the naming rules"`

And if you're smart, you might have a task file filled with context and instructions. So just use that:
`iMi docs/task.md`

## Acceptance Criteria

### Core Command Interface
- [ ] **AC-001**: `iMi feat <name>` creates feature worktree with branch `feat/<name>`
- [ ] **AC-002**: `iMi review <pr_number>` creates PR review worktree from existing PR
- [ ] **AC-003**: `iMi fix <name>` creates bugfix worktree with branch `fix/<name>`
- [ ] **AC-004**: `iMi aiops <name>` creates AI operations worktree with branch `aiops/<name>`
- [ ] **AC-005**: `iMi devops <name>` creates DevOps worktree with branch `devops/<name>`
- [ ] **AC-006**: `iMi trunk` switches to main trunk worktree
- [ ] **AC-007**: All commands support `--repo <name>` flag for remote operation

### Repository Management
- [ ] **AC-008**: Auto-clone from `delorenj/<repo_name>` if repo not found locally
- [ ] **AC-009**: Register repo locations in SQLite database for tracking
- [ ] **AC-010**: Support IMI_ROOT environment variable for base directory
- [ ] **AC-011**: Default to `~/code/` if IMI_ROOT not set
- [ ] **AC-012**: Maintain one clone per repo with multiple worktrees as siblings

### Directory Structure Convention
- [ ] **AC-013**: Create worktrees as sibling directories to main clone
- [ ] **AC-014**: Name worktrees as `<prefix>-<branch_name>` (e.g., `feat-add-logging`)
- [ ] **AC-015**: Main branch worktree named `trunk-<branch>` (e.g., `trunk-main`)
- [ ] **AC-016**: Create `sync/` directory for shared configuration
- [ ] **AC-017**: Support `sync/global/` for pipeline-wide sync
- [ ] **AC-018**: Support `sync/repo/` for repository-specific sync

### Symlink Management
- [ ] **AC-019**: Auto-symlink dotfiles from `sync/repo/` to all worktrees
- [ ] **AC-020**: Support configurable symlink file list (.env, .vscode/, etc.)
> Note: Should it be symlinked or copied? Symlinked is easier to maintain, copied is safer. Maybe a config option?

- [ ] **AC-021**: Handle symlink conflicts gracefully with user prompt
- [ ] **AC-022**: Maintain symlinks when creating new worktrees

### Database Tracking
- [ ] **AC-023**: SQLite database at `~/.config/iMi/iMi.db`
- [ ] **AC-024**: Track all worktrees with metadata (created, last_accessed, agent_id)
- [ ] **AC-025**: Record agent activities and file changes
- [ ] **AC-026**: Support multi-host coordination through database sync
> Note: This should be centralized. Imagine having multiple trello boards. There is a central server already spec'd out I call [Jelmore](/home/delorenj/code/projects/33god/jelmore). This could be a future integration.

### Real-time Monitoring
  > Note: This will be an event based architecture. We can use something like [watchdog](https://pypi.org/project/watchdog/) to monitor file changes and update the database in real-time. The 33GOD pipeline includes a NATS event bus for agent communication, and general pub/sub messaging, observability, etc. This could be a future integration,
- [ ] **AC-027**: `iMi monitor` shows real-time file changes across worktrees
- [ ] **AC-028**: Display git status (ahead/behind, dirty files) for each worktree
- [ ] **AC-029**: Track which agents are active in which worktrees
- [ ] **AC-030**: Show performance metrics and activity timestamps

### Status and Listing
- [ ] **AC-031**: `iMi status` displays all active worktrees by type
- [ ] **AC-032**: `iMi list` shows comprehensive worktree inventory
- [ ] **AC-033**: Show branch synchronization status with upstream
- [ ] **AC-034**: Display recent agent activities per worktree

### Git Integration
- [ ] **AC-035**: All feature branches created from fresh `trunk` fetch
- [ ] **AC-036**: Auto-fetch from origin before creating new worktrees
- [ ] **AC-037**: Support `git worktree` commands under the hood
- [ ] **AC-038**: Handle git authentication seamlessly
- [ ] **AC-039**: Validate git repository state before operations

### Error Handling & Validation
- [ ] **AC-040**: Graceful handling of missing git repositories
- [ ] **AC-041**: Clear error messages for invalid branch names
- [ ] **AC-042**: Validate worktree doesn't already exist before creation
- [ ] **AC-043**: Handle network failures during clone operations
- [ ] **AC-044**: Recover from corrupted database state

### Configuration Management
- [ ] **AC-045**: Support `~/.config/iMi/config.toml` for user preferences
- [ ] **AC-046**: Convention over configuration - minimal required setup
- [ ] **AC-047**: Configurable default branch name (main/master)
- [ ] **AC-048**: Configurable remote name (origin)
- [ ] **AC-049**: Configurable symlink file patterns


### Testing Requirements
- [ ] **AC-060**: Unit tests for all core commands with >90% coverage
- [ ] **AC-061**: Integration tests with real git repositories
- [ ] **AC-063**: Error scenario testing (network failures, permissions, etc.)
- [ ] **AC-064**: Cross-platform compatibility testing (Linux, macOS, Windows)

