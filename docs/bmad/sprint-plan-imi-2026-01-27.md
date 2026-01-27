# Sprint Plan: iMi Agent Workflow Commands

**Date:** 2026-01-27
**Scrum Master:** Jarad DeLorenzo
**Project Level:** 2
**Total Stories:** 5
**Total Points:** 20
**Planned Sprints:** 1

---

## Executive Summary

This sprint implements the three missing iMi commands required for Yi agent integration: claim, release, and metadata management. These commands enable proper agent worktree allocation, release protocols with clean state checks, and task source linking through metadata.

**Key Metrics:**
- Total Stories: 5
- Total Points: 20
- Sprints: 1
- Team Capacity: 30 points per sprint
- Target Completion: 2026-02-10 (2 weeks)

---

## Story Inventory

### STORY-001: Implement `imi claim` command

**Priority:** Must Have

**User Story:**
As a Yi agent
I want to claim exclusive access to a worktree
So that I can work without conflicts from other agents

**Acceptance Criteria:**
- [ ] CLI command `imi claim <name> --yi-id <id>` added to src/cli.rs
- [ ] Handler checks if worktree already claimed (error if agent_id set)
- [ ] Updates `worktrees.agent_id` column in database
- [ ] Creates `.iMi/presence/<worktree>.lock` file with agent metadata
- [ ] Logs activity to `agent_activities` table with type 'claimed'
- [ ] Returns success with worktree metadata in JSON mode
- [ ] Supports `--force` flag for emergency override
- [ ] Error handling: worktree not found, already claimed, missing yi-id

**Technical Notes:**
- Add `Claim` variant to `Commands` enum in src/cli.rs
- Add `handle_claim_command()` in src/main.rs
- Add `claim_worktree(worktree_id, agent_id)` method in src/database.rs
- Add `create_lock_file(worktree_name, agent_id)` in src/local.rs
- Lock file format: JSON with {agent_id, timestamp, hostname}

**Dependencies:**
None

**Estimate:** 5 points

---

### STORY-002: Implement `imi release` command

**Priority:** Must Have

**User Story:**
As a Yi agent
I want to release a worktree after completing my work
So that other agents can claim it and accountability is maintained

**Acceptance Criteria:**
- [ ] CLI command `imi release <name> --yi-id <id>` added to src/cli.rs
- [ ] Handler verifies agent owns the worktree (agent_id matches)
- [ ] Checks git status for uncommitted changes (fail if dirty)
- [ ] Clears `worktrees.agent_id` column (set to NULL)
- [ ] Removes `.iMi/presence/<worktree>.lock` file
- [ ] Logs activity to `agent_activities` table with type 'released'
- [ ] Returns success in JSON mode
- [ ] Error handling: not owner, dirty state, worktree not found

**Technical Notes:**
- Add `Release` variant to `Commands` enum in src/cli.rs
- Add `handle_release_command()` in src/main.rs
- Add `release_worktree(worktree_id, agent_id)` method in src/database.rs
- Use GitManager to check git status (reuse existing code)
- Add `remove_lock_file(worktree_name)` in src/local.rs
- Fail fast on dirty state with clear error message

**Dependencies:**
- STORY-001 (claim must exist for release to make sense)

**Estimate:** 5 points

---

### STORY-003: Implement `imi metadata set` command

**Priority:** Must Have

**User Story:**
As a Yi agent
I want to set metadata on a worktree
So that I can link it to task sources (Plane tickets, Bloodbank correlations, Yi orchestrators)

**Acceptance Criteria:**
- [ ] CLI command `imi metadata set --worktree <name> --key <k> --value <v>` added
- [ ] Handler loads existing metadata JSONB from database
- [ ] Merges new key-value pair with existing metadata
- [ ] Updates `worktrees.metadata` column with merged JSON
- [ ] Returns success in JSON mode
- [ ] Supports nested keys with dot notation (e.g., `--key "task.priority"`)
- [ ] Error handling: worktree not found, invalid JSON value

**Technical Notes:**
- Add `Metadata` variant with `Set` subcommand to `Commands` enum
- Add `handle_metadata_command()` in src/main.rs
- Add `set_worktree_metadata(worktree_id, key, value)` in src/database.rs
- Use `serde_json::Value` for metadata manipulation
- PostgreSQL JSONB merge: `UPDATE worktrees SET metadata = metadata || '{"key":"value"}'::jsonb`

**Dependencies:**
None

**Estimate:** 3 points

---

### STORY-004: Implement `imi metadata get` command

**Priority:** Must Have

**User Story:**
As a Yi agent
I want to retrieve metadata from a worktree
So that I can read task source links and other metadata

**Acceptance Criteria:**
- [ ] CLI command `imi metadata get --worktree <name> [--key <k>]` added
- [ ] Handler loads metadata from database
- [ ] If `--key` provided: returns single value
- [ ] If no key: returns entire metadata object
- [ ] Supports JSON output mode
- [ ] Supports nested key retrieval with dot notation
- [ ] Error handling: worktree not found, key not found

**Technical Notes:**
- Add `Get` subcommand to `Metadata` variant
- Extend `handle_metadata_command()` for get operation
- Add `get_worktree_metadata(worktree_id, key)` in src/database.rs
- Use PostgreSQL JSONB operators: `metadata->>'key'` for single value
- Use `metadata` for full object

**Dependencies:**
- STORY-003 (get/set are coupled features)

**Estimate:** 2 points

---

### STORY-005: Add integration tests for agent workflows

**Priority:** Should Have

**User Story:**
As a developer
I want comprehensive integration tests for agent workflows
So that I can ensure claim/release/metadata commands work correctly

**Acceptance Criteria:**
- [ ] Test: Claim unclaimed worktree succeeds
- [ ] Test: Claim already-claimed worktree fails
- [ ] Test: Release with clean state succeeds
- [ ] Test: Release with dirty state fails
- [ ] Test: Release by non-owner fails
- [ ] Test: Metadata set/get round-trip works
- [ ] Test: Multiple metadata keys can be set
- [ ] Test: Nested metadata keys work
- [ ] Test: Lock file creation/removal works
- [ ] All tests pass in CI

**Technical Notes:**
- Create `tests/agent_workflows.rs` integration test file
- Use `tempfile` crate for temporary test databases
- Use `tokio-test` for async test support
- Mock GitManager for git status checks
- Test both JSON and non-JSON output modes

**Dependencies:**
- STORY-001, STORY-002, STORY-003, STORY-004 (testing the implementations)

**Estimate:** 5 points

---

## Sprint Allocation

### Sprint 1 (Weeks 1-2) - 20/30 points

**Goal:** Implement agent workflow commands for Yi integration

**Stories:**
- STORY-001: Implement `imi claim` command (5 points) - Must Have
- STORY-002: Implement `imi release` command (5 points) - Must Have
- STORY-003: Implement `imi metadata set` command (3 points) - Must Have
- STORY-004: Implement `imi metadata get` command (2 points) - Must Have
- STORY-005: Add integration tests for agent workflows (5 points) - Should Have

**Total:** 20 points / 30 capacity (67% utilization)

**Risks:**
- Git status checking may have edge cases (detached HEAD, merge conflicts)
- Lock file handling needs proper error recovery if filesystem issues
- JSONB nested key handling may be complex

**Dependencies:**
- None external (all dependencies internal to sprint)

---

## Feature Traceability

| Feature | Stories | Total Points | Priority |
|---------|---------|--------------|----------|
| Agent Worktree Claiming | STORY-001 | 5 | Must Have |
| Agent Worktree Release | STORY-002 | 5 | Must Have |
| Worktree Metadata Management | STORY-003, STORY-004 | 5 | Must Have |
| Integration Testing | STORY-005 | 5 | Should Have |

---

## Requirements Coverage

All requirements from architecture document and skill documentation are covered:

| Requirement | Story | Sprint |
|-------------|-------|--------|
| imi claim command with --yi-id | STORY-001 | 1 |
| imi release command with --yi-id | STORY-002 | 1 |
| imi metadata set command | STORY-003 | 1 |
| imi metadata get command | STORY-004 | 1 |
| Lock file management (.iMi/presence/) | STORY-001, STORY-002 | 1 |
| Agent activity logging | STORY-001, STORY-002 | 1 |
| Clean state checking on release | STORY-002 | 1 |
| JSONB metadata operations | STORY-003, STORY-004 | 1 |
| Integration tests | STORY-005 | 1 |

---

## Risks and Mitigation

**High:**
None

**Medium:**
- Git status edge cases (detached HEAD, merge conflicts, submodules)
  - Mitigation: Comprehensive testing, fail-safe error handling

- Lock file race conditions on shared filesystems
  - Mitigation: Use atomic file operations, handle EEXIST errors gracefully

**Low:**
- JSONB nested key parsing complexity
  - Mitigation: Use existing serde_json functionality, extensive unit tests

---

## Dependencies

**Internal:**
- All stories depend on existing database schema (worktrees.agent_id, worktrees.metadata)
- Lock file management depends on .iMi/ directory structure

**External:**
None

---

## Definition of Done

For a story to be considered complete:
- [ ] Code implemented and committed
- [ ] Unit tests written and passing (≥80% coverage)
- [ ] Integration tests passing (if applicable)
- [ ] Code compiles with no warnings
- [ ] Documentation updated (command help text, README)
- [ ] Skill documentation updated (33god-creating-and-working-with-projects)
- [ ] Manual testing completed
- [ ] No breaking changes to existing commands

---

## Implementation Notes

**File Modifications Required:**

1. **src/cli.rs:**
   - Add `Claim`, `Release`, `Metadata` commands to `Commands` enum
   - Define command parameters and arguments

2. **src/main.rs:**
   - Add `handle_claim_command()`
   - Add `handle_release_command()`
   - Add `handle_metadata_command()`
   - Wire up command routing in main match statement

3. **src/database.rs:**
   - Add `claim_worktree(worktree_id: &Uuid, agent_id: &str) -> Result<()>`
   - Add `release_worktree(worktree_id: &Uuid, agent_id: &str) -> Result<()>`
   - Add `set_worktree_metadata(worktree_id: &Uuid, key: &str, value: serde_json::Value) -> Result<()>`
   - Add `get_worktree_metadata(worktree_id: &Uuid, key: Option<&str>) -> Result<serde_json::Value>`

4. **src/local.rs:**
   - Add `create_lock_file(imi_dir: &Path, worktree_name: &str, agent_id: &str) -> Result<()>`
   - Add `remove_lock_file(imi_dir: &Path, worktree_name: &str) -> Result<()>`
   - Add `read_lock_file(imi_dir: &Path, worktree_name: &str) -> Result<LockFileData>`

5. **tests/agent_workflows.rs:**
   - New integration test file with comprehensive workflow tests

**Database Schema:**
No schema changes needed. Existing schema already supports:
- `worktrees.agent_id TEXT` (for claim/release)
- `worktrees.metadata JSONB` (for metadata operations)
- `agent_activities` table (for activity logging)

**Lock File Format:**
```json
{
  "agent_id": "claude-sonnet-4.5-agent-001",
  "claimed_at": "2026-01-27T15:30:00Z",
  "hostname": "devbox.33god.ai",
  "worktree_id": "uuid"
}
```

---

## Next Steps

**Immediate:** Begin Sprint 1

Run `/dev-story STORY-001` to start implementing the `imi claim` command.

**Sprint cadence:**
- Sprint length: 2 weeks
- Sprint planning: Monday Week 1
- Sprint review: Friday Week 2
- Sprint retrospective: Friday Week 2

**Post-Sprint:**
After completing these commands, Yi agent integration will be unblocked. The Yi orchestrator can then:
- Claim worktrees before assigning tasks to agents
- Release worktrees after agents complete work
- Link worktrees to task sources via metadata
- Track agent activities through the audit log

---

**This plan was created using BMAD Method v6 - Phase 4 (Implementation Planning)**
