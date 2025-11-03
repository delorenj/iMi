# Prune Test Execution Guide

## Quick Start

### Run All Tests
```bash
# Using helper script (recommended)
./scripts/run-prune-tests.sh

# Or directly with cargo
cargo test --test prune_fix_tests -- --nocapture
```

### Run Specific Test
```bash
# Using helper script
./scripts/run-prune-tests.sh -t test_prune_after_manual_deletion -o

# Or directly with cargo
cargo test test_prune_after_manual_deletion -- --nocapture
```

## Test Execution Options

### Helper Script Options

The `run-prune-tests.sh` script provides convenient options:

```bash
# Show help
./scripts/run-prune-tests.sh -h

# Run with verbose output
./scripts/run-prune-tests.sh -v -o

# Run specific test
./scripts/run-prune-tests.sh -t test_full_prune_workflow -o

# Run all tests with output
./scripts/run-prune-tests.sh -o
```

### Direct Cargo Commands

```bash
# Basic test run
cargo test --test prune_fix_tests

# With output capture disabled
cargo test --test prune_fix_tests -- --nocapture

# Single-threaded execution
cargo test --test prune_fix_tests -- --test-threads=1

# With debug logging
RUST_LOG=debug cargo test --test prune_fix_tests -- --nocapture

# With backtrace on panic
RUST_BACKTRACE=1 cargo test --test prune_fix_tests -- --nocapture
```

## Test Scenarios

### Scenario 1: Reproducing TASK.md Issue

**Test**: `test_prune_after_manual_deletion`

**Run Command**:
```bash
./scripts/run-prune-tests.sh -t test_prune_after_manual_deletion -o
```

**What It Tests**:
- Creates a worktree
- Manually deletes the directory (simulating `rm -rf`)
- Runs `imi prune`
- Verifies Git and database cleanup

**Expected Output**:
```
🗑️  Manually deleting worktree directory: /tmp/.tmpXXX/feat-test-feature
🧹 Running prune command...
✅ Verifying cleanup...
✅ Test passed: Prune successfully cleaned up manually deleted worktree
```

---

### Scenario 2: Multiple Worktrees

**Test**: `test_prune_multiple_stale_worktrees`

**Run Command**:
```bash
./scripts/run-prune-tests.sh -t test_prune_multiple_stale_worktrees -o
```

**What It Tests**:
- Creates 3 worktrees
- Deletes 2, keeps 1 valid
- Verifies selective cleanup

**Expected Behavior**:
- Stale worktrees removed
- Valid worktree preserved
- No false positives

---

### Scenario 3: Dry Run Safety

**Test**: `test_prune_dry_run`

**Run Command**:
```bash
./scripts/run-prune-tests.sh -t test_prune_dry_run -o
```

**What It Tests**:
- Runs prune with `--dry-run` flag
- Verifies no destructive changes
- Reports what would be done

---

### Scenario 4: Orphaned Directories

**Test**: `test_orphaned_directory_cleanup`

**Run Command**:
```bash
./scripts/run-prune-tests.sh -t test_orphaned_directory_cleanup -o
```

**What It Tests**:
- Creates fake worktree directory
- Not registered in Git or database
- Verifies detection and cleanup

---

### Scenario 5: Full Workflow

**Test**: `test_full_prune_workflow`

**Run Command**:
```bash
./scripts/run-prune-tests.sh -t test_full_prune_workflow -o
```

**What It Tests**:
- Comprehensive simulation
- Multiple scenarios in one test
- Real-world usage patterns

**Expected Output**:
```
🧪 Running full prune workflow simulation...
1️⃣  Creating worktrees...
2️⃣  Simulating various deletion scenarios...
3️⃣  Running prune...
4️⃣  Verifying final state...
✅ Full workflow test passed!
```

## Test Verification

### Manual Verification Steps

After running tests, you can manually verify the fix works:

1. **Create a test repository**:
   ```bash
   mkdir /tmp/test-imi && cd /tmp/test-imi
   git init
   git commit --allow-empty -m "Initial commit"
   ```

2. **Initialize iMi**:
   ```bash
   imi init
   ```

3. **Create a worktree**:
   ```bash
   imi feat test-manual
   ```

4. **Manually delete it**:
   ```bash
   cd ..
   rm -rf feat-test-manual
   ```

5. **Run prune**:
   ```bash
   cd trunk-main
   imi prune
   ```

6. **Verify cleanup**:
   ```bash
   # Git worktrees should be clean
   git worktree list

   # iMi database should be clean
   imi list
   ```

### Automated Verification

Run the complete test suite:

```bash
./scripts/run-prune-tests.sh -v -o
```

Check for:
- ✅ All 10 tests pass
- ✅ No panics or errors
- ✅ Clean test output
- ✅ Reasonable execution time (< 30 seconds total)

## Troubleshooting

### Problem: Tests fail with "Git not found"

**Solution**:
```bash
# Install git if missing
sudo apt-get install git  # Ubuntu/Debian
brew install git          # macOS

# Verify git is in PATH
which git
```

---

### Problem: Tests hang or timeout

**Solution**:
```bash
# Run with single thread
cargo test --test prune_fix_tests -- --test-threads=1 --nocapture

# Check for processes
ps aux | grep git
```

---

### Problem: Permission denied errors

**Solution**:
```bash
# Clean up old temp directories
rm -rf /tmp/.tmp*

# Check disk space
df -h /tmp
```

---

### Problem: Database errors

**Solution**:
```bash
# Tests use temporary databases, but if issues persist:
rm -rf ~/.imi/test*.db
```

---

### Problem: Inconsistent test results

**Solution**:
```bash
# Ensure serial execution
cargo test --test prune_fix_tests -- --test-threads=1

# Clean and rebuild
cargo clean
cargo test --test prune_fix_tests
```

## Performance Benchmarks

### Expected Test Duration

| Test | Expected Time | Threshold |
|------|--------------|-----------|
| `test_prune_after_manual_deletion` | < 1s | 2s |
| `test_prune_multiple_stale_worktrees` | < 2s | 5s |
| `test_prune_dry_run` | < 1s | 2s |
| `test_git_admin_directory_cleanup` | < 1s | 2s |
| `test_orphaned_directory_cleanup` | < 1s | 2s |
| `test_prune_preserves_valid_worktrees` | < 1s | 2s |
| `test_database_cleanup_only` | < 1s | 2s |
| `test_corrupted_gitdir` | < 1s | 2s |
| `test_full_prune_workflow` | < 3s | 5s |
| `test_prune_performance` | < 5s | 10s |
| **Total** | **< 20s** | **30s** |

### Monitoring Performance

```bash
# Time the test execution
time ./scripts/run-prune-tests.sh

# Profile with flamegraph (if installed)
cargo flamegraph --test prune_fix_tests
```

## Continuous Integration

### GitHub Actions Example

Add to `.github/workflows/prune-tests.yml`:

```yaml
name: Prune Fix Tests

on:
  push:
    branches: [ main, fix/pruning ]
  pull_request:
    branches: [ main ]

jobs:
  prune-tests:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          profile: minimal
          override: true

      - name: Install Git
        run: |
          sudo apt-get update
          sudo apt-get install -y git

      - name: Run Prune Tests
        run: ./scripts/run-prune-tests.sh -o

      - name: Upload Test Results
        if: failure()
        uses: actions/upload-artifact@v3
        with:
          name: test-logs
          path: target/tmp/
```

### Local Pre-commit Hook

Create `.git/hooks/pre-commit`:

```bash
#!/bin/bash
# Run prune tests before commit

echo "Running prune tests..."
./scripts/run-prune-tests.sh

if [ $? -ne 0 ]; then
    echo "❌ Prune tests failed. Commit aborted."
    exit 1
fi

echo "✅ Prune tests passed."
exit 0
```

Make it executable:
```bash
chmod +x .git/hooks/pre-commit
```

## Test Coverage

### Current Coverage

The test suite covers:

- ✅ Manual deletion scenario (TASK.md reproduction)
- ✅ Git reference cleanup
- ✅ Database entry deactivation
- ✅ Git admin directory removal
- ✅ Orphaned directory detection
- ✅ Multiple worktree pruning
- ✅ Dry-run mode
- ✅ Valid worktree preservation
- ✅ Corrupted gitdir handling
- ✅ Performance with many worktrees

### Coverage Gaps (Future Work)

- ⬜ Remote branch cleanup
- ⬜ Submodule handling
- ⬜ LFS object cleanup
- ⬜ Concurrent prune operations
- ⬜ Network-based repositories
- ⬜ Large-scale tests (100+ worktrees)

### Measuring Coverage

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Run coverage
cargo tarpaulin --test prune_fix_tests --out Html --output-dir coverage/

# Open report
open coverage/index.html
```

## Test Maintenance

### When to Run Tests

- ✅ Before committing changes to prune functionality
- ✅ After modifying `worktree.rs`, `git.rs`, or `database.rs`
- ✅ Before releasing new version
- ✅ After Git version upgrade
- ✅ When adding new prune features

### Updating Tests

When modifying prune behavior:

1. **Update affected tests**
   ```bash
   # Find tests that need updates
   grep -r "prune_stale_worktrees" tests/
   ```

2. **Add new test cases**
   ```bash
   # Copy existing test as template
   cp tests/prune_fix_tests.rs tests/prune_fix_tests_new.rs
   # Edit and add new test
   ```

3. **Update documentation**
   ```bash
   # Update both documentation files
   vim tests/PRUNE_TEST_DOCUMENTATION.md
   vim tests/PRUNE_TEST_GUIDE.md
   ```

4. **Verify all tests pass**
   ```bash
   ./scripts/run-prune-tests.sh -v -o
   ```

## Advanced Usage

### Debug a Failing Test

```bash
# Run with full debug output
RUST_LOG=trace RUST_BACKTRACE=full \
    cargo test test_prune_after_manual_deletion -- --nocapture --test-threads=1

# Inspect temp directories (before cleanup)
# Add `std::thread::sleep(Duration::from_secs(60));` in test
# Then in another terminal:
ls -la /tmp/.tmp*/
```

### Custom Test Environment

```bash
# Use custom temp directory
export TMPDIR=/custom/temp/path
cargo test --test prune_fix_tests

# Use custom database location
export IMI_DB_PATH=/custom/db/path
cargo test --test prune_fix_tests
```

### Profiling Tests

```bash
# Install cargo-flamegraph
cargo install flamegraph

# Profile specific test
cargo flamegraph --test prune_fix_tests -- test_prune_performance --nocapture

# View flamegraph
open flamegraph.svg
```

## Integration with Development Workflow

### Development Cycle

1. **Make changes** to prune implementation
2. **Run relevant tests**:
   ```bash
   ./scripts/run-prune-tests.sh -t test_prune_after_manual_deletion -o
   ```
3. **Fix issues** if tests fail
4. **Run full suite**:
   ```bash
   ./scripts/run-prune-tests.sh -v -o
   ```
5. **Commit changes** with passing tests

### Testing Checklist

Before committing prune-related changes:

- [ ] All prune tests pass
- [ ] Manual verification completed
- [ ] Performance acceptable
- [ ] Documentation updated
- [ ] No new warnings or clippy issues
- [ ] Edge cases considered
- [ ] Backward compatibility maintained

## Resources

### Documentation
- **TASK.md**: Original problem description
- **PRUNE_TEST_DOCUMENTATION.md**: Detailed test documentation
- **PRUNE_TEST_GUIDE.md**: This guide

### Code Locations
- **Implementation**: `src/worktree.rs` (lines 1285-1458)
- **Git operations**: `src/git.rs` (lines 463-506)
- **Database operations**: `src/database.rs` (lines 432-444)
- **Tests**: `tests/prune_fix_tests.rs`

### External Resources
- [Git Worktree Documentation](https://git-scm.com/docs/git-worktree)
- [Rust Testing Guide](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [SQLx Documentation](https://docs.rs/sqlx/)

## Support

For issues or questions:

1. Check this guide first
2. Review test output carefully
3. Check GitHub issues for similar problems
4. Create new issue with:
   - Test output
   - System information
   - Steps to reproduce

## License

Same as main project (MIT)
