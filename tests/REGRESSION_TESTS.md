# iMi Regression Test Suite

Critical test cases derived from production bugs to prevent recurrence.

## PR Worktree Cross-Repository Creation

**Bug Reference:** Fixed 2025-11-03
**Root Cause:** Two layered bugs:
1. Repository resolution used `env::current_dir()` instead of database lookup
2. `gh pr checkout` had side effect of checking out branch in trunk directory

### Test Case 1: PR Worktree from Different Repository

**Scenario:** User in repo A creates PR worktree for repo B

```rust
#[tokio::test]
async fn test_pr_worktree_from_different_repo() {
    // Setup: Create two test repositories
    let repo_a = setup_test_repo("repo-a", "/tmp/test-repos/repo-a");
    let repo_b = setup_test_repo("repo-b", "/tmp/test-repos/repo-b");

    // Register both repos in iMi database
    db.register_repository("repo-a", &repo_a.path).await?;
    db.register_repository("repo-b", &repo_b.path).await?;

    // Set current directory to repo A
    env::set_current_dir(&repo_a.path)?;

    // Create PR worktree in repo B
    let result = manager.create_review_worktree(
        123,
        Some("repo-b")
    ).await?;

    // Assertions
    // 1. Worktree created in correct repo directory
    assert!(result.starts_with("/tmp/test-repos/repo-b/pr-123"));
    assert!(result.exists());

    // 2. Repo A trunk unchanged
    let repo_a_branch = git.get_current_branch(&repo_a.path)?;
    assert_eq!(repo_a_branch, "main");

    // 3. Repo B trunk unchanged
    let repo_b_branch = git.get_current_branch(&format!("{}/trunk-main", repo_b.path))?;
    assert_eq!(repo_b_branch, "main");

    // 4. PR worktree on correct branch
    let pr_branch = git.get_current_branch(&result)?;
    assert_eq!(pr_branch, "pr-123");
}
```

### Test Case 2: PR Fetch Without Trunk Corruption

**Scenario:** Fetching PR should not change trunk directory's branch

```rust
#[test]
fn test_fetch_pr_without_trunk_side_effect() {
    let repo = setup_test_repo_with_pr("test-repo", 123);
    let trunk_path = repo.path.join("trunk-main");

    // Record trunk state before operation
    let trunk_branch_before = git.get_current_branch(&trunk_path)?;
    let trunk_commit_before = git.get_head_commit(&trunk_path)?;

    // Fetch PR ref
    let remote_ref = git.fetch_pr_ref(&trunk_path, 123)?;
    git.fetch_branch(&trunk_path, &remote_ref, "pr-123")?;

    // Verify trunk unchanged
    let trunk_branch_after = git.get_current_branch(&trunk_path)?;
    let trunk_commit_after = git.get_head_commit(&trunk_path)?;

    assert_eq!(trunk_branch_before, trunk_branch_after);
    assert_eq!(trunk_commit_before, trunk_commit_after);

    // Verify branch was fetched
    assert!(git.local_branch_exists(&trunk_path, "pr-123")?);
}
```

### Test Case 3: Repository Resolution Priority

**Scenario:** Repo argument should override current directory

```rust
#[tokio::test]
async fn test_repo_resolution_priority() {
    // Setup
    let repo_a = setup_test_repo("repo-a", "/tmp/test-repos/repo-a");
    let repo_b = setup_test_repo("repo-b", "/tmp/test-repos/repo-b");

    db.register_repository("repo-a", &repo_a.path).await?;
    db.register_repository("repo-b", &repo_b.path).await?;

    // Set current directory to repo A
    env::set_current_dir(&repo_a.path)?;

    // Resolve repo B by name
    let resolved = manager.resolve_repo_name(Some("repo-b")).await?;
    assert_eq!(resolved, "repo-b");

    let db_repo = db.get_repository(&resolved).await?.unwrap();
    assert_eq!(db_repo.path, "/tmp/test-repos/repo-b/trunk-main");
}
```

### Test Case 4: GitHub Owner/Repo Format Resolution

**Scenario:** Format `owner/repo` should resolve to registered repo

```rust
#[tokio::test]
async fn test_github_format_repo_resolution() {
    // Setup: Register repo with GitHub remote
    let repo = setup_test_repo("trinote2.0", "/tmp/test-repos/trinote2.0");
    db.register_repository_with_remote(
        "trinote2.0",
        &repo.path,
        "git@github.com:YIC-Triumph/trinote2.0.git"
    ).await?;

    // Resolve using GitHub format
    let resolved = manager.resolve_repo_name(
        Some("YIC-Triumph/trinote2.0")
    ).await?;

    assert_eq!(resolved, "trinote2.0");

    let db_repo = db.get_repository(&resolved).await?.unwrap();
    assert!(db_repo.remote_url.contains("YIC-Triumph/trinote2.0"));
}
```

## Git State Recovery Tests

### Test Case 5: Corrupted HEAD Recovery

**Scenario:** Detect and recover from corrupted HEAD reference

```rust
#[test]
fn test_detect_corrupted_head() {
    let repo = setup_test_repo("test-repo", "/tmp/test-repo");

    // Corrupt HEAD
    std::fs::write(
        repo.path.join(".git/HEAD"),
        "ref: refs/heads/non-existent-branch"
    )?;

    // Attempt operation - should detect corruption
    let result = manager.create_feature_worktree("test", None).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("corrupted"));
}

#[test]
fn test_recover_corrupted_head() {
    let repo = setup_test_repo("test-repo", "/tmp/test-repo");

    // Corrupt HEAD
    std::fs::write(
        repo.path.join(".git/HEAD"),
        "ref: refs/heads/non-existent-branch"
    )?;

    // Run recovery
    git.recover_corrupted_head(&repo.path, "main")?;

    // Verify recovery
    let current_branch = git.get_current_branch(&repo.path)?;
    assert_eq!(current_branch, "main");
}
```

### Test Case 6: Orphaned Worktree Detection

**Scenario:** Detect worktrees that exist in Git but not filesystem

```rust
#[tokio::test]
async fn test_detect_orphaned_worktrees() {
    let repo = setup_test_repo("test-repo", "/tmp/test-repo");

    // Create worktree
    let wt_path = repo.path.join("feat-test");
    git.create_worktree(&repo, "feat-test", &wt_path, "feat/test", None)?;

    // Manually delete directory (simulating user deletion)
    std::fs::remove_dir_all(&wt_path)?;

    // Detect orphaned worktrees
    let orphaned = manager.detect_orphaned_worktrees(&repo).await?;

    assert_eq!(orphaned.len(), 1);
    assert_eq!(orphaned[0].name, "feat-test");
}
```

## Worktree State Consistency Tests

### Test Case 7: Database-Git Synchronization

**Scenario:** Database entries should match actual Git worktree state

```rust
#[tokio::test]
async fn test_database_git_sync() {
    let repo = setup_test_repo("test-repo", "/tmp/test-repo");

    // Create worktree via iMi
    let wt_path = manager.create_feature_worktree("test", None).await?;

    // Verify database entry
    let db_worktree = db.get_worktree("test-repo", "feat-test").await?;
    assert!(db_worktree.is_some());

    // Verify Git worktree
    let git_worktrees = git.list_worktrees(&repo)?;
    assert!(git_worktrees.contains(&"feat-test".to_string()));

    // Verify filesystem
    assert!(wt_path.exists());
}
```

### Test Case 8: Prune Cleanup Completeness

**Scenario:** Prune should clean all three layers (Git, DB, filesystem)

```rust
#[tokio::test]
async fn test_prune_cleanup_completeness() {
    let repo = setup_test_repo("test-repo", "/tmp/test-repo");

    // Create and then manually delete worktree directory
    let wt_path = manager.create_feature_worktree("test", None).await?;
    std::fs::remove_dir_all(&wt_path)?;

    // Run prune
    manager.prune_stale_worktrees(Some("test-repo"), false, false).await?;

    // Verify all layers cleaned
    // 1. Git worktree reference removed
    let git_worktrees = git.list_worktrees(&repo)?;
    assert!(!git_worktrees.contains(&"feat-test".to_string()));

    // 2. Database entry deactivated
    let db_worktree = db.get_worktree("test-repo", "feat-test").await?;
    assert!(db_worktree.is_none() || !db_worktree.unwrap().active);

    // 3. Filesystem cleaned (already done in test setup)
    assert!(!wt_path.exists());
}
```

## Integration Test Suite

### Test Case 9: End-to-End PR Workflow

**Scenario:** Complete PR review workflow from creation to cleanup

```rust
#[tokio::test]
async fn test_complete_pr_workflow() {
    // Setup
    let repo = setup_test_repo_with_gh("test-repo", "/tmp/test-repo");
    db.register_repository("test-repo", &repo.path).await?;

    // 1. Create PR worktree
    let pr_path = manager.create_review_worktree(123, None).await?;
    assert!(pr_path.exists());
    assert_eq!(git.get_current_branch(&pr_path)?, "pr-123");

    // 2. Verify trunk unchanged
    let trunk_path = repo.path.join("trunk-main");
    assert_eq!(git.get_current_branch(&trunk_path)?, "main");

    // 3. Make changes in PR worktree
    std::fs::write(pr_path.join("test.txt"), "changes")?;
    git.commit_all(&pr_path, "test changes")?;

    // 4. Close PR worktree
    manager.close_worktree("pr-123", None).await?;

    // 5. Verify cleanup
    assert!(!pr_path.exists());
    let db_worktree = db.get_worktree("test-repo", "pr-123").await?;
    assert!(db_worktree.is_none() || !db_worktree.unwrap().active);
}
```

## Test Helpers

```rust
// Test repository setup
fn setup_test_repo(name: &str, path: &str) -> TestRepo {
    let path = PathBuf::from(path);
    std::fs::create_dir_all(&path).unwrap();

    // Initialize git repo
    Command::new("git")
        .current_dir(&path)
        .args(&["init"])
        .output()
        .unwrap();

    // Create initial commit
    Command::new("git")
        .current_dir(&path)
        .args(&["commit", "--allow-empty", "-m", "Initial commit"])
        .output()
        .unwrap();

    TestRepo { name: name.to_string(), path }
}

// Setup repo with mock PR
fn setup_test_repo_with_pr(name: &str, pr_number: u32) -> TestRepo {
    let repo = setup_test_repo(name, &format!("/tmp/test-repos/{}", name));

    // Create PR branch
    Command::new("git")
        .current_dir(&repo.path)
        .args(&["checkout", "-b", &format!("pr-{}", pr_number)])
        .output()
        .unwrap();

    // Add commit
    Command::new("git")
        .current_dir(&repo.path)
        .args(&["commit", "--allow-empty", "-m", "PR changes"])
        .output()
        .unwrap();

    // Return to main
    Command::new("git")
        .current_dir(&repo.path)
        .args(&["checkout", "main"])
        .output()
        .unwrap();

    repo
}

struct TestRepo {
    name: String,
    path: PathBuf,
}

impl Drop for TestRepo {
    fn drop(&mut self) {
        // Cleanup test repo
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
```

## Running Tests

```bash
# Run all regression tests
cargo test --test regression

# Run specific test category
cargo test --test regression -- pr_worktree
cargo test --test regression -- git_state
cargo test --test regression -- integration

# Run with output
cargo test --test regression -- --nocapture

# Run specific test
cargo test --test regression test_pr_worktree_from_different_repo -- --nocapture
```

## Continuous Integration

Add to `.github/workflows/test.yml`:

```yaml
name: Regression Tests

on: [push, pull_request]

jobs:
  regression:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Install gh CLI
        run: |
          curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg | sudo dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg
          echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" | sudo tee /etc/apt/sources.list.d/github-cli.list > /dev/null
          sudo apt update
          sudo apt install gh

      - name: Run Regression Tests
        run: cargo test --test regression

      - name: Check Test Coverage
        run: |
          cargo install cargo-tarpaulin
          cargo tarpaulin --test regression --out Xml

      - name: Upload Coverage
        uses: codecov/codecov-action@v3
```

## Related Documentation

- `/home/delorenj/.claude/skills/git-state-recovery.md` - Recovery procedures
- `/home/delorenj/.claude/skills/layered-bug-diagnosis.md` - Debugging methodology
- `/home/delorenj/.claude/skills/ecosystem-patterns/SKILL.md` - Composable git operations pattern
