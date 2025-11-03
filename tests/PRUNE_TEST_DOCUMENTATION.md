# Prune Fix Test Suite Documentation

## Overview

This test suite validates the fix for the pruning issue described in `TASK.md`, where worktree directories manually deleted outside of Git should be properly cleaned up by both Git references and the iMi database.

## Problem Statement (from TASK.md)

When a worktree directory is manually deleted (e.g., `rm -rf feat-pr-validation-fix`), the Git references and database entries remain stale. The `imi prune` command should detect and clean up these orphaned references.

**Observed Issue:**
- `git worktree prune` doesn't remove the stale reference
- `imi prune` fails with "Git repository not found" when run from wrong directory
- Database entries remain active even after manual deletion

**Expected Behavior:**
- `imi prune` should detect manually deleted worktrees
- Git worktree references should be removed
- Database entries should be deactivated
- Orphaned directories should be cleaned up

## Test Suite Structure

### Location
- **File**: `tests/prune_fix_tests.rs`
- **Test Type**: Integration tests with serial execution
- **Requires**: Real Git repository, filesystem operations, database

### Test Fixture

The `PruneTestFixture` struct provides a complete test environment:

```rust
struct PruneTestFixture {
    _temp_dir: TempDir,           // Temporary test directory
    repo_path: PathBuf,            // Path to Git repository
    imi_path: PathBuf,             // iMi root path
    trunk_path: PathBuf,           // Trunk worktree path
    config: Config,                // iMi configuration
    db: Database,                  // Database instance
    git: GitManager,               // Git operations manager
    manager: WorktreeManager,      // Worktree manager
}
```

**Fixture Capabilities:**
- Creates real Git repository with initial commit
- Initializes iMi database with schema
- Provides helper methods for worktree operations
- Handles cleanup automatically via TempDir

## Test Cases

### Test 1: Basic Manual Deletion and Prune
**Test**: `test_prune_after_manual_deletion()`

**Purpose**: Reproduces the exact scenario from TASK.md

**Steps:**
1. Create a worktree `feat-test-feature`
2. Verify it exists in Git, database, and filesystem
3. Manually delete the worktree directory (`rm -rf`)
4. Verify references still exist in Git and database
5. Run `imi prune`
6. Verify all references are cleaned up

**Expected Results:**
- ✅ Git worktree reference removed
- ✅ Database entry deactivated
- ✅ Filesystem already clean
- ✅ Git admin directory removed

**Run Command:**
```bash
cargo test test_prune_after_manual_deletion -- --nocapture
```

---

### Test 2: Multiple Stale Worktrees
**Test**: `test_prune_multiple_stale_worktrees()`

**Purpose**: Validate batch pruning of multiple worktrees

**Steps:**
1. Create 3 worktrees: `feat-feature-1`, `feat-feature-2`, `feat-feature-3`
2. Manually delete worktrees 1 and 3, keep worktree 2
3. Run `imi prune`
4. Verify selective cleanup

**Expected Results:**
- ✅ Worktrees 1 and 3 removed from Git and database
- ✅ Worktree 2 preserved (still valid)
- ✅ No false positives

---

### Test 3: Dry-Run Mode
**Test**: `test_prune_dry_run()`

**Purpose**: Verify dry-run flag doesn't make destructive changes

**Steps:**
1. Create and delete a worktree
2. Run `imi prune --dry-run`
3. Verify reporting without removal

**Expected Results:**
- ✅ Reports what would be removed
- ⚠️  May still clean up database (implementation detail)
- ✅ No Git references removed in dry-run

---

### Test 4: Git Admin Directory Cleanup
**Test**: `test_git_admin_directory_cleanup()`

**Purpose**: Verify `.git/worktrees/<name>` directories are removed

**Steps:**
1. Create a worktree
2. Count admin directories in `.git/worktrees/`
3. Manually delete worktree
4. Verify admin directory still exists
5. Run prune
6. Verify admin directory removed

**Expected Results:**
- ✅ Admin directory exists after manual deletion
- ✅ Admin directory removed after prune
- ✅ Count decreases correctly

---

### Test 5: Orphaned Directory Cleanup
**Test**: `test_orphaned_directory_cleanup()`

**Purpose**: Detect and remove directories that look like worktrees but aren't registered

**Steps:**
1. Create a fake directory `feat-orphaned` (not in Git)
2. Verify it's not registered anywhere
3. Run prune with `--force` flag
4. Verify directory is removed

**Expected Results:**
- ✅ Detects unregistered worktree-like directories
- ✅ Removes orphaned directories with confirmation/force
- ✅ Reports size reclaimed

---

### Test 6: Valid Worktrees Preserved
**Test**: `test_prune_preserves_valid_worktrees()`

**Purpose**: Ensure prune doesn't remove valid, active worktrees

**Steps:**
1. Create a valid worktree
2. Run prune
3. Verify worktree still exists everywhere

**Expected Results:**
- ✅ Valid worktree directory preserved
- ✅ Git reference preserved
- ✅ Database entry preserved
- ✅ No false positives

---

### Test 7: Database-Only Cleanup
**Test**: `test_database_cleanup_only()`

**Purpose**: Verify database cleanup when Git already cleaned up

**Steps:**
1. Create worktree
2. Remove from Git and filesystem manually
3. Verify still in database
4. Run prune
5. Verify database entry deactivated

**Expected Results:**
- ✅ Detects database entries without Git references
- ✅ Deactivates stale database entries
- ✅ Handles partial cleanup states

---

### Test 8: Corrupted Gitdir Edge Case
**Test**: `test_corrupted_gitdir()`

**Purpose**: Handle corrupted `.git` file in worktree

**Steps:**
1. Create worktree
2. Corrupt `.git` file with invalid gitdir path
3. Delete worktree directory
4. Run prune
5. Verify graceful handling

**Expected Results:**
- ✅ Either succeeds with cleanup
- ✅ Or fails gracefully with clear error
- ✅ Doesn't crash or hang

---

### Test 9: Full Workflow Integration
**Test**: `test_full_prune_workflow()`

**Purpose**: Comprehensive simulation of real-world usage

**Steps:**
1. Create 3 worktrees
2. Apply mixed scenarios:
   - Manual deletion (TASK.md scenario)
   - Valid worktree (keep)
   - Orphaned directory
   - Another manual deletion
3. Run single prune command
4. Verify all scenarios handled correctly

**Expected Results:**
- ✅ Manual deletions cleaned up
- ✅ Valid worktrees preserved
- ✅ Orphaned directories removed
- ✅ Complete state consistency

---

### Test 10: Performance Test
**Test**: `test_prune_performance()`

**Purpose**: Validate performance with many worktrees

**Steps:**
1. Create 10 worktrees
2. Delete every other one (5 stale)
3. Measure prune execution time
4. Verify correctness of cleanup

**Expected Results:**
- ✅ Prune completes in reasonable time
- ✅ All stale worktrees removed
- ✅ All valid worktrees preserved
- ✅ Performance is acceptable for larger codebases

---

## Running the Tests

### Run All Prune Tests
```bash
cargo test --test prune_fix_tests -- --nocapture
```

### Run Specific Test
```bash
cargo test test_prune_after_manual_deletion -- --nocapture
```

### Run with Detailed Output
```bash
RUST_LOG=debug cargo test --test prune_fix_tests -- --nocapture --test-threads=1
```

### Run Serial Tests (Recommended)
All tests use `#[serial]` annotation to prevent race conditions:
```bash
cargo test --test prune_fix_tests
```

## Test Requirements

### Dependencies
```toml
[dev-dependencies]
tempfile = "3.10"          # Temporary directories
tokio-test = "0.4"          # Async test support
serial_test = "0.9.0"       # Serial test execution
```

### System Requirements
- Git installed and in PATH
- Write permissions for temp directories
- Sufficient disk space for test repositories

### Environment Setup
No special environment variables required. Tests are self-contained.

## Expected Test Output

### Successful Run
```
running 10 tests
🗑️  Manually deleting worktree directory: /tmp/.tmpXXX/feat-test-feature
🧹 Running prune command...
✅ Verifying cleanup...
✅ Test passed: Prune successfully cleaned up manually deleted worktree
test test_prune_after_manual_deletion ... ok
...
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### Test Failure Example
```
assertion failed: Worktree should be removed from Git
thread 'test_prune_after_manual_deletion' panicked at tests/prune_fix_tests.rs:142
```

## Edge Cases Covered

### 1. Corrupted Git State
- **Scenario**: `.git` file points to invalid path
- **Handling**: Graceful failure or forced cleanup
- **Test**: `test_corrupted_gitdir()`

### 2. Locked Worktrees
- **Scenario**: Worktree directory exists but locked
- **Handling**: Skip with warning or force removal
- **Coverage**: Implicitly tested in workflow test

### 3. Orphaned Directories
- **Scenario**: Directory matches pattern but not registered
- **Handling**: Prompt for removal or auto-remove with `--force`
- **Test**: `test_orphaned_directory_cleanup()`

### 4. Partial Cleanup State
- **Scenario**: Git removed but database entry remains
- **Handling**: Detect and clean database
- **Test**: `test_database_cleanup_only()`

### 5. Multiple Concurrent Prunes
- **Scenario**: Prune command run multiple times
- **Handling**: Idempotent operation
- **Coverage**: Tests use `#[serial]` to prevent actual concurrency

## Validation Checklist

After running tests, verify:

- [ ] All 10 tests pass
- [ ] No panic or crash conditions
- [ ] Git references properly removed
- [ ] Database entries properly deactivated
- [ ] Filesystem cleaned up correctly
- [ ] Valid worktrees never removed
- [ ] Performance acceptable (< 5s for 10 worktrees)
- [ ] Dry-run mode safe
- [ ] Error messages clear and actionable
- [ ] Edge cases handled gracefully

## Integration with CI/CD

### GitHub Actions Example
```yaml
name: Prune Tests

on: [push, pull_request]

jobs:
  test-prune:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run prune tests
        run: cargo test --test prune_fix_tests -- --nocapture
```

## Troubleshooting

### Tests Fail with "Git repository not found"
**Cause**: Test fixture failed to initialize Git repo
**Solution**: Ensure Git is installed and accessible in PATH

### Tests Fail with Database Errors
**Cause**: Permission issues or stale temp directories
**Solution**: Clean temp directories: `rm -rf /tmp/.tmp*`

### Tests Hang or Timeout
**Cause**: Missing `#[serial]` annotation or deadlock
**Solution**: Ensure all tests use `#[serial]` attribute

### Inconsistent Test Results
**Cause**: Race conditions between tests
**Solution**: Run with `--test-threads=1` flag

## Future Test Enhancements

### Planned Additions
1. **Network-based tests**: Test with remote repositories
2. **Concurrency tests**: Multiple prune operations
3. **Large-scale tests**: 100+ worktrees
4. **Recovery tests**: Restore from backup after failed prune
5. **Benchmark suite**: Performance regression detection

### Test Coverage Goals
- [x] Manual deletion scenario (TASK.md)
- [x] Git reference cleanup
- [x] Database cleanup
- [x] Orphaned directory detection
- [x] Edge cases (corruption, partial state)
- [ ] Remote tracking branch cleanup
- [ ] Submodule handling
- [ ] LFS object cleanup

## Maintenance Notes

### When to Update Tests
- After modifying `prune_stale_worktrees()` implementation
- When adding new prune flags or options
- After Git version upgrades
- When database schema changes

### Test Data Cleanup
Tests use `TempDir` which automatically cleans up on drop. No manual cleanup required.

## Contributing

When adding new prune functionality:
1. Add corresponding test case
2. Document expected behavior
3. Update this documentation
4. Ensure all existing tests still pass
5. Add integration with main test suite

## References

- **TASK.md**: Original problem description
- **src/worktree.rs**: Prune implementation
- **src/git.rs**: Git operations
- **src/database.rs**: Database operations

## License

Same as main project (MIT)
