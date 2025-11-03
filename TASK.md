# TASK

Implement `repair` command.

## Problem

Git worktree metadata stores absolute paths in three places:

1. .git/worktrees/<name>/gitdir - points to the worktree's .git file
2. <worktree>/.git - points back to the main repo's worktree metadata
3. .git/worktrees/<name>/commondir - points to the main repo's .git directory
4. iMi main database located in ~/.config/iMi/ - stores absolute paths to the iMi sandboxes

When moving a repo, all four needed updating from the old path to the new one.

## Solution

Implement a `repair` command that implicitly detects moved repos and updates the paths accordingly.

## Implementation Complete ✅

The `repair` command has been successfully implemented with the following features:

### Command Structure
- Added `Repair` command to CLI enum in `src/cli.rs:189-193`
- Added handler `handle_repair_command` in `src/main.rs:495-503`
- Command dispatcher integration in `src/main.rs:123-125`

### Core Repair Logic
Implemented `repair_paths` method in `src/worktree.rs:1490-1621` that:

1. **Repairs Git Worktree Metadata:**
   - Updates `.git/worktrees/<name>/gitdir` with correct absolute path to worktree's .git file
   - Updates `.git/worktrees/<name>/commondir` with correct path to main repo's .git directory
   - Updates `<worktree>/.git` file with correct gitdir reference

2. **Repairs iMi Database:**
   - Updates worktree paths in the database for each worktree
   - Updates repository path in the database
   - Preserves all other metadata (branch names, worktree types, agent IDs, etc.)

3. **Error Handling:**
   - Collects all errors encountered during repair
   - Reports summary of repaired items
   - Provides detailed error messages for any failures
   - Continues processing even if individual repairs fail

### Usage
```bash
imi repair [REPO]
```

Arguments:
- `[REPO]` - Optional repository name. If not specified, uses the current repository.

### Features
- Automatically detects current repository location
- Repairs all worktrees for the specified repository
- Updates both Git metadata and iMi database
- Provides detailed progress output with emojis
- Reports summary of all repaired items
- Error-tolerant: continues processing after individual failures
