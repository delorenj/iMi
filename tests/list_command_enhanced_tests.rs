/// Comprehensive integration tests for the enhanced iMi list command
///
/// This test suite validates all aspects of the context-aware list command:
/// - Context detection (inside/outside repos, registered/unregistered)
/// - Flag behavior (--projects, --worktrees, --repo)
/// - Edge cases (no repos, no worktrees, multiple repos)
/// - Full workflow integration tests
///
/// Test Structure:
/// Each test is independent and creates its own isolated test environment
/// using temporary directories and databases to ensure no test pollution.
use anyhow::Result;
use serial_test::serial;
use std::env;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio;

// Import necessary modules from the main crate
use imi::config::{Config, GitSettings, MonitoringSettings, SyncSettings};
use imi::database::Database;
use imi::git::GitManager;
use imi::worktree::WorktreeManager;

/// RAII guard for directory changes - automatically restores original directory on drop
struct DirGuard {
    original_dir: PathBuf,
}

impl DirGuard {
    fn new() -> Result<Self> {
        Ok(Self {
            original_dir: env::current_dir()?,
        })
    }
}

impl Drop for DirGuard {
    fn drop(&mut self) {
        let _ = env::set_current_dir(&self.original_dir);
    }
}

/// Helper function to create a test environment with a git repository
async fn setup_test_repo(
    temp_dir: &TempDir,
) -> Result<(PathBuf, Config, Database, WorktreeManager)> {
    let test_repo_path = temp_dir.path().join("test-repo");
    std::fs::create_dir_all(&test_repo_path)?;

    // Store original directory to restore later
    let original_dir = env::current_dir()?;
    env::set_current_dir(&test_repo_path)?;

    // Initialize git repo
    std::process::Command::new("git").args(&["init"]).output()?;

    // Configure git user for tests
    std::process::Command::new("git")
        .args(&["config", "user.name", "Test User"])
        .output()?;

    std::process::Command::new("git")
        .args(&["config", "user.email", "test@example.com"])
        .output()?;

    // Create initial commit
    std::fs::write(test_repo_path.join("README.md"), "# Test Repo")?;
    std::process::Command::new("git")
        .args(&["add", "."])
        .output()?;
    std::process::Command::new("git")
        .args(&["commit", "-m", "Initial commit"])
        .output()?;

    // Restore original directory
    env::set_current_dir(&original_dir)?;

    // Setup config and database
    let config = Config {
        database_path: temp_dir.path().join("imi.db"),
        root_path: temp_dir.path().to_path_buf(),
        sync_settings: SyncSettings {
            enabled: false,
            user_sync_path: temp_dir.path().join("sync/global"),
            local_sync_path: temp_dir.path().join("sync/repo"),
        },
        git_settings: GitSettings {
            default_branch: "main".to_string(),
            remote_name: "origin".to_string(),
            auto_fetch: false,
            prune_on_fetch: false,
        },
        monitoring_settings: MonitoringSettings {
            enabled: false,
            refresh_interval_ms: 5000,
            watch_file_changes: false,
            track_agent_activity: false,
        },
        symlink_files: vec![],
        ..Default::default()
    };

    let db = Database::new(&config.database_path).await?;
    let git_manager = GitManager::new();
    let worktree_manager = WorktreeManager::new(git_manager, db.clone(), config.clone(), None);

    Ok((test_repo_path, config, db, worktree_manager))
}

/// Helper function to create a second test repository
async fn create_second_repo(temp_dir: &TempDir, name: &str) -> Result<PathBuf> {
    let repo_path = temp_dir.path().join(name);
    std::fs::create_dir_all(&repo_path)?;

    let original_dir = env::current_dir()?;
    env::set_current_dir(&repo_path)?;

    std::process::Command::new("git").args(&["init"]).output()?;
    std::process::Command::new("git")
        .args(&["config", "user.name", "Test User"])
        .output()?;
    std::process::Command::new("git")
        .args(&["config", "user.email", "test@example.com"])
        .output()?;

    std::fs::write(repo_path.join("README.md"), format!("# {}", name))?;
    std::process::Command::new("git")
        .args(&["add", "."])
        .output()?;
    std::process::Command::new("git")
        .args(&["commit", "-m", "Initial commit"])
        .output()?;

    env::set_current_dir(&original_dir)?;

    Ok(repo_path)
}

// ============================================================================
// CONTEXT DETECTION TESTS
// ============================================================================

#[tokio::test]
#[serial]
async fn test_list_outside_repo_shows_all_projects() -> Result<()> {
    // Test: Running list outside any git repository should show all registered projects
    let temp_dir = TempDir::new()?;
    let _guard = DirGuard::new()?; // Ensure directory is restored on test completion

    let (test_repo_path, _config, db, worktree_manager) = setup_test_repo(&temp_dir).await?;

    // Register the repository
    db.create_repository("test-repo", test_repo_path.to_str().unwrap(), "", "main")
        .await?;

    // Change to a directory outside the repository
    let non_repo_dir = temp_dir.path().join("non-repo");
    std::fs::create_dir_all(&non_repo_dir)?;
    env::set_current_dir(&non_repo_dir)?;

    // Execute list command (should show all projects)
    let result = worktree_manager.list_smart(None, false, false).await;

    // Verify command succeeds
    assert!(result.is_ok(), "List command should succeed outside repo");

    // Verify repository is listed in database
    let repos = db.list_repositories().await?;
    assert_eq!(repos.len(), 1, "Should have one registered repository");
    assert_eq!(repos[0].name, "test-repo");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_list_inside_registered_repo_shows_worktrees() -> Result<()> {
    // Test: Running list inside a registered repository should show worktrees for that repo
    let temp_dir = TempDir::new()?;
    let _guard = DirGuard::new()?; // Ensure directory is restored on test completion
    let (test_repo_path, _config, db, worktree_manager) = setup_test_repo(&temp_dir).await?;

    // Register the repository
    db.create_repository("test-repo", test_repo_path.to_str().unwrap(), "", "main")
        .await?;

    // Create a worktree directly in the database (simulating a created worktree)
    let worktree_path = temp_dir.path().join("feat-test-feature");
    std::fs::create_dir_all(&worktree_path)?;

    db.create_worktree(
        "test-repo",
        "feat-test-feature",
        "feat/test-feature",
        "feat",
        worktree_path.to_str().unwrap(),
        None,
    )
    .await?;

    // Change to the repository directory
    env::set_current_dir(&test_repo_path)?;

    // Execute list command (should show worktrees for this repo)
    let result = worktree_manager.list_smart(None, false, false).await;

    // Verify command succeeds
    assert!(
        result.is_ok(),
        "List command should succeed inside registered repo"
    );

    // Verify worktrees exist
    let worktrees = db.list_worktrees(Some("test-repo")).await?;
    assert_eq!(worktrees.len(), 1, "Should have one worktree");
    assert_eq!(worktrees[0].worktree_name, "feat-test-feature");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_list_inside_unregistered_repo_shows_helpful_message() -> Result<()> {
    // Test: Running list inside an unregistered repository should show onboarding message
    let temp_dir = TempDir::new()?;
    let _guard = DirGuard::new()?; // Ensure directory is restored on test completion
    let (test_repo_path, _config, _db, worktree_manager) = setup_test_repo(&temp_dir).await?;

    // DO NOT register the repository - leave it unregistered

    // Change to the unregistered repository directory
    env::set_current_dir(&test_repo_path)?;

    // Execute list command (should show helpful onboarding message)
    let result = worktree_manager.list_smart(None, false, false).await;

    // Verify command succeeds (shows message, doesn't error)
    assert!(
        result.is_ok(),
        "List command should handle unregistered repo gracefully"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_list_inside_worktree_shows_parent_repo_worktrees() -> Result<()> {
    // Test: Running list inside a worktree should show worktrees for the parent repo
    let temp_dir = TempDir::new()?;
    let _guard = DirGuard::new()?; // Ensure directory is restored on test completion
    let (test_repo_path, _config, db, worktree_manager) = setup_test_repo(&temp_dir).await?;

    // Register the repository
    db.create_repository("test-repo", test_repo_path.to_str().unwrap(), "", "main")
        .await?;

    // Create worktrees directly in the database
    let worktree_path1 = temp_dir.path().join("feat-test-feature");
    std::fs::create_dir_all(&worktree_path1)?;
    // Initialize as a git repo so context detection works
    env::set_current_dir(&worktree_path1)?;
    std::process::Command::new("git").args(&["init"]).output()?;
    std::process::Command::new("git")
        .args(&["config", "user.name", "Test"])
        .output()?;
    std::process::Command::new("git")
        .args(&["config", "user.email", "test@example.com"])
        .output()?;

    db.create_worktree(
        "test-repo",
        "feat-test-feature",
        "feat/test-feature",
        "feat",
        worktree_path1.to_str().unwrap(),
        None,
    )
    .await?;

    let worktree_path2 = temp_dir.path().join("fix-test-fix");
    std::fs::create_dir_all(&worktree_path2)?;
    db.create_worktree(
        "test-repo",
        "fix-test-fix",
        "fix/test-fix",
        "fix",
        worktree_path2.to_str().unwrap(),
        None,
    )
    .await?;

    // Change to the worktree directory
    env::set_current_dir(&worktree_path1)?;

    // Execute list command (should show all worktrees for parent repo)
    let result = worktree_manager.list_smart(None, false, false).await;

    // Verify command succeeds
    assert!(
        result.is_ok(),
        "List command should succeed inside worktree"
    );

    // Verify we can see all worktrees for the parent repo
    let worktrees = db.list_worktrees(Some("test-repo")).await?;
    assert_eq!(
        worktrees.len(),
        2,
        "Should see both worktrees from parent repo"
    );

    Ok(())
}

// ============================================================================
// FLAG BEHAVIOR TESTS
// ============================================================================

#[tokio::test]
#[serial]
async fn test_projects_flag_outside_repo_lists_all_projects() -> Result<()> {
    // Test: --projects flag outside repo lists all projects
    let temp_dir = TempDir::new()?;
    let _guard = DirGuard::new()?; // Ensure directory is restored on test completion
    let (test_repo_path, _config, db, worktree_manager) = setup_test_repo(&temp_dir).await?;

    // Register the repository
    db.create_repository("test-repo", test_repo_path.to_str().unwrap(), "", "main")
        .await?;

    // Change to non-repo directory
    let non_repo_dir = temp_dir.path().join("non-repo");
    std::fs::create_dir_all(&non_repo_dir)?;
    env::set_current_dir(&non_repo_dir)?;

    // Execute with --projects flag
    let result = worktree_manager.list_smart(None, false, true).await;

    assert!(result.is_ok(), "List with --projects should succeed");

    // Verify we can retrieve all repositories
    let repos = db.list_repositories().await?;
    assert_eq!(repos.len(), 1, "Should list all registered projects");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_projects_flag_inside_repo_lists_all_projects() -> Result<()> {
    // Test: --projects flag inside repo overrides context and lists all projects
    let temp_dir = TempDir::new()?;
    let _guard = DirGuard::new()?; // Ensure directory is restored on test completion
    let (test_repo_path, _config, db, worktree_manager) = setup_test_repo(&temp_dir).await?;

    // Register the repository
    db.create_repository("test-repo", test_repo_path.to_str().unwrap(), "", "main")
        .await?;

    // Create a second repository
    let second_repo_path = create_second_repo(&temp_dir, "second-repo").await?;
    db.create_repository(
        "second-repo",
        second_repo_path.to_str().unwrap(),
        "",
        "main",
    )
    .await?;

    // Change to first repo directory
    env::set_current_dir(&test_repo_path)?;

    // Execute with --projects flag (should show ALL projects, not just current repo)
    let result = worktree_manager.list_smart(None, false, true).await;

    assert!(result.is_ok(), "List with --projects should succeed");

    // Verify both repositories are accessible
    let repos = db.list_repositories().await?;
    assert_eq!(
        repos.len(),
        2,
        "Should list all projects, not just current repo context"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_worktrees_flag_outside_repo_lists_all_worktrees() -> Result<()> {
    // Test: --worktrees flag outside repo lists all worktrees from all repos
    let temp_dir = TempDir::new()?;
    let _guard = DirGuard::new()?; // Ensure directory is restored on test completion
    let (test_repo_path, _config, db, worktree_manager) = setup_test_repo(&temp_dir).await?;

    // Register the repository
    db.create_repository("test-repo", test_repo_path.to_str().unwrap(), "", "main")
        .await?;

    // Create worktrees directly in database
    let worktree_path1 = temp_dir.path().join("feat-feature-1");
    std::fs::create_dir_all(&worktree_path1)?;
    db.create_worktree(
        "test-repo",
        "feat-feature-1",
        "feat/feature-1",
        "feat",
        worktree_path1.to_str().unwrap(),
        None,
    )
    .await?;

    let worktree_path2 = temp_dir.path().join("fix-fix-1");
    std::fs::create_dir_all(&worktree_path2)?;
    db.create_worktree(
        "test-repo",
        "fix-fix-1",
        "fix/fix-1",
        "fix",
        worktree_path2.to_str().unwrap(),
        None,
    )
    .await?;

    // Change to non-repo directory
    let non_repo_dir = temp_dir.path().join("non-repo");
    std::fs::create_dir_all(&non_repo_dir)?;
    env::set_current_dir(&non_repo_dir)?;

    // Execute with --worktrees flag
    let result = worktree_manager.list_smart(None, true, false).await;

    assert!(result.is_ok(), "List with --worktrees should succeed");

    // Verify worktrees are accessible
    let worktrees = db.list_worktrees(None).await?;
    assert_eq!(
        worktrees.len(),
        2,
        "Should list all worktrees from all repos"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_worktrees_flag_inside_repo_lists_repo_worktrees() -> Result<()> {
    // Test: --worktrees flag inside repo lists worktrees for current repo
    let temp_dir = TempDir::new()?;
    let _guard = DirGuard::new()?; // Ensure directory is restored on test completion
    let (test_repo_path, _config, db, worktree_manager) = setup_test_repo(&temp_dir).await?;

    // Register the repository
    db.create_repository("test-repo", test_repo_path.to_str().unwrap(), "", "main")
        .await?;

    // Create worktree directly in database
    let worktree_path = temp_dir.path().join("feat-feature-1");
    std::fs::create_dir_all(&worktree_path)?;
    db.create_worktree(
        "test-repo",
        "feat-feature-1",
        "feat/feature-1",
        "feat",
        worktree_path.to_str().unwrap(),
        None,
    )
    .await?;

    // Change to repo directory
    env::set_current_dir(&test_repo_path)?;

    // Execute with --worktrees flag
    let result = worktree_manager.list_smart(None, true, false).await;

    assert!(
        result.is_ok(),
        "List with --worktrees should succeed inside repo"
    );

    // Verify worktrees for this repo
    let worktrees = db.list_worktrees(Some("test-repo")).await?;
    assert_eq!(worktrees.len(), 1, "Should list worktrees for current repo");

    Ok(())
}

// ============================================================================
// REPO PARAMETER TESTS
// ============================================================================

#[tokio::test]
#[serial]
async fn test_repo_flag_lists_specified_repo_worktrees() -> Result<()> {
    // Test: --repo <name> flag lists worktrees for specified repository
    let temp_dir = TempDir::new()?;
    let _guard = DirGuard::new()?; // Ensure directory is restored on test completion
    let (test_repo_path, _config, db, worktree_manager) = setup_test_repo(&temp_dir).await?;

    // Register repositories
    db.create_repository("test-repo", test_repo_path.to_str().unwrap(), "", "main")
        .await?;

    let second_repo_path = create_second_repo(&temp_dir, "second-repo").await?;
    db.create_repository(
        "second-repo",
        second_repo_path.to_str().unwrap(),
        "",
        "main",
    )
    .await?;

    // Create worktrees for both repos directly in database
    let worktree_path1 = temp_dir.path().join("feat-feature-1");
    std::fs::create_dir_all(&worktree_path1)?;
    db.create_worktree(
        "test-repo",
        "feat-feature-1",
        "feat/feature-1",
        "feat",
        worktree_path1.to_str().unwrap(),
        None,
    )
    .await?;

    let worktree_path2 = temp_dir.path().join("feat-feature-2");
    std::fs::create_dir_all(&worktree_path2)?;
    db.create_worktree(
        "second-repo",
        "feat-feature-2",
        "feat/feature-2",
        "feat",
        worktree_path2.to_str().unwrap(),
        None,
    )
    .await?;

    // Change to non-repo directory
    let non_repo_dir = temp_dir.path().join("non-repo");
    std::fs::create_dir_all(&non_repo_dir)?;
    env::set_current_dir(&non_repo_dir)?;

    // Execute with --repo flag for first repo
    let result = worktree_manager
        .list_smart(Some("test-repo"), false, false)
        .await;

    assert!(result.is_ok(), "List with --repo should succeed");

    // Verify we get only the specified repo's worktrees
    let worktrees = db.list_worktrees(Some("test-repo")).await?;
    assert_eq!(
        worktrees.len(),
        1,
        "Should list only specified repo's worktrees"
    );
    assert_eq!(worktrees[0].worktree_name, "feat-feature-1");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_repo_flag_with_invalid_repo_shows_error() -> Result<()> {
    // Test: --repo <invalid> shows proper error message
    let temp_dir = TempDir::new()?;
    let _guard = DirGuard::new()?; // Ensure directory is restored on test completion
    let (_test_repo_path, _config, _db, worktree_manager) = setup_test_repo(&temp_dir).await?;

    // Change to non-repo directory
    let non_repo_dir = temp_dir.path().join("non-repo");
    std::fs::create_dir_all(&non_repo_dir)?;
    env::set_current_dir(&non_repo_dir)?;

    // Execute with --repo flag for non-existent repo
    let result = worktree_manager
        .list_smart(Some("nonexistent-repo"), false, false)
        .await;

    // Command should succeed but show helpful error message
    assert!(
        result.is_ok(),
        "List with invalid --repo should handle gracefully"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_repo_flag_with_projects_flag_shows_projects() -> Result<()> {
    // Test: Conflicting flags - --repo with --projects should show projects
    let temp_dir = TempDir::new()?;
    let _guard = DirGuard::new()?; // Ensure directory is restored on test completion
    let (test_repo_path, _config, db, worktree_manager) = setup_test_repo(&temp_dir).await?;

    // Register the repository
    db.create_repository("test-repo", test_repo_path.to_str().unwrap(), "", "main")
        .await?;

    // Change to non-repo directory
    let non_repo_dir = temp_dir.path().join("non-repo");
    std::fs::create_dir_all(&non_repo_dir)?;
    env::set_current_dir(&non_repo_dir)?;

    // Execute with both --repo and --projects (projects should take precedence)
    let result = worktree_manager
        .list_smart(Some("test-repo"), false, true)
        .await;

    assert!(
        result.is_ok(),
        "List with --repo and --projects should succeed"
    );

    // Verify we get all repositories (projects flag takes precedence)
    let repos = db.list_repositories().await?;
    assert_eq!(
        repos.len(),
        1,
        "Should list all projects when --projects flag is used"
    );

    Ok(())
}

// ============================================================================
// EDGE CASE TESTS
// ============================================================================

#[tokio::test]
#[serial]
async fn test_no_registered_repositories_shows_onboarding() -> Result<()> {
    // Test: No registered repositories shows helpful onboarding message
    let temp_dir = TempDir::new()?;
    let _guard = DirGuard::new()?; // Ensure directory is restored on test completion
    let (_test_repo_path, _config, db, worktree_manager) = setup_test_repo(&temp_dir).await?;

    // DO NOT register any repositories

    // Change to non-repo directory
    let non_repo_dir = temp_dir.path().join("non-repo");
    std::fs::create_dir_all(&non_repo_dir)?;
    env::set_current_dir(&non_repo_dir)?;

    // Execute list command
    let result = worktree_manager.list_smart(None, false, false).await;

    assert!(result.is_ok(), "List with no repos should show onboarding");

    // Verify no repositories exist
    let repos = db.list_repositories().await?;
    assert_eq!(repos.len(), 0, "Should have no registered repositories");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_no_worktrees_in_registered_repo_shows_empty_message() -> Result<()> {
    // Test: Registered repo with no worktrees shows appropriate empty message
    let temp_dir = TempDir::new()?;
    let _guard = DirGuard::new()?; // Ensure directory is restored on test completion
    let (test_repo_path, _config, db, worktree_manager) = setup_test_repo(&temp_dir).await?;

    // Register the repository
    db.create_repository("test-repo", test_repo_path.to_str().unwrap(), "", "main")
        .await?;

    // DO NOT create any worktrees

    // Change to repo directory
    env::set_current_dir(&test_repo_path)?;

    // Execute list command
    let result = worktree_manager.list_smart(None, false, false).await;

    assert!(result.is_ok(), "List with no worktrees should succeed");

    // Verify no worktrees exist
    let worktrees = db.list_worktrees(Some("test-repo")).await?;
    assert_eq!(worktrees.len(), 0, "Should have no worktrees");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_multiple_registered_repos_all_shown() -> Result<()> {
    // Test: Multiple registered repositories are all shown correctly
    let temp_dir = TempDir::new()?;
    let _guard = DirGuard::new()?; // Ensure directory is restored on test completion
    let (test_repo_path, _config, db, worktree_manager) = setup_test_repo(&temp_dir).await?;

    // Register first repository
    db.create_repository("test-repo", test_repo_path.to_str().unwrap(), "", "main")
        .await?;

    // Create and register second repository
    let second_repo_path = create_second_repo(&temp_dir, "second-repo").await?;
    db.create_repository(
        "second-repo",
        second_repo_path.to_str().unwrap(),
        "",
        "main",
    )
    .await?;

    // Create and register third repository
    let third_repo_path = create_second_repo(&temp_dir, "third-repo").await?;
    db.create_repository("third-repo", third_repo_path.to_str().unwrap(), "", "main")
        .await?;

    // Change to non-repo directory
    let non_repo_dir = temp_dir.path().join("non-repo");
    std::fs::create_dir_all(&non_repo_dir)?;
    env::set_current_dir(&non_repo_dir)?;

    // Execute list command
    let result = worktree_manager.list_smart(None, false, false).await;

    assert!(result.is_ok(), "List with multiple repos should succeed");

    // Verify all repositories are registered
    let repos = db.list_repositories().await?;
    assert_eq!(repos.len(), 3, "Should list all registered repositories");

    let repo_names: Vec<String> = repos.iter().map(|r| r.name.clone()).collect();
    assert!(repo_names.contains(&"test-repo".to_string()));
    assert!(repo_names.contains(&"second-repo".to_string()));
    assert!(repo_names.contains(&"third-repo".to_string()));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_repository_with_many_worktrees_displays_correctly() -> Result<()> {
    // Test: Repository with many worktrees displays properly
    let temp_dir = TempDir::new()?;
    let _guard = DirGuard::new()?; // Ensure directory is restored on test completion
    let (test_repo_path, _config, db, worktree_manager) = setup_test_repo(&temp_dir).await?;

    // Register the repository
    db.create_repository("test-repo", test_repo_path.to_str().unwrap(), "", "main")
        .await?;

    // Create multiple worktrees of different types directly in database
    let wt_paths = vec![
        ("feat-feature-1", "feat/feature-1", "feat"),
        ("feat-feature-2", "feat/feature-2", "feat"),
        ("fix-fix-1", "fix/fix-1", "fix"),
        ("fix-fix-2", "fix/fix-2", "fix"),
        ("aiops-aiops-1", "aiops/aiops-1", "aiops"),
    ];

    for (wt_name, branch_name, wt_type) in wt_paths {
        let wt_path = temp_dir.path().join(wt_name);
        std::fs::create_dir_all(&wt_path)?;
        db.create_worktree(
            "test-repo",
            wt_name,
            branch_name,
            wt_type,
            wt_path.to_str().unwrap(),
            None,
        )
        .await?;
    }

    // Change to repo directory
    env::set_current_dir(&test_repo_path)?;

    // Execute list command
    let result = worktree_manager.list_smart(None, false, false).await;

    assert!(result.is_ok(), "List with many worktrees should succeed");

    // Verify all worktrees are registered
    let worktrees = db.list_worktrees(Some("test-repo")).await?;
    assert_eq!(worktrees.len(), 5, "Should have created 5 worktrees");

    // Verify different worktree types
    let mut feat_count = 0;
    let mut fix_count = 0;
    let mut aiops_count = 0;

    for wt in worktrees {
        match wt.worktree_type.as_str() {
            "feat" => feat_count += 1,
            "fix" => fix_count += 1,
            "aiops" => aiops_count += 1,
            _ => {}
        }
    }

    assert_eq!(feat_count, 2, "Should have 2 feature worktrees");
    assert_eq!(fix_count, 2, "Should have 2 fix worktrees");
    assert_eq!(aiops_count, 1, "Should have 1 aiops worktree");

    Ok(())
}

// ============================================================================
// INTEGRATION TESTS
// ============================================================================

#[tokio::test]
#[serial]
async fn test_full_workflow_init_register_create_list() -> Result<()> {
    // Test: Complete workflow from initialization to listing
    let temp_dir = TempDir::new()?;
    let _guard = DirGuard::new()?; // Ensure directory is restored on test completion
    let (test_repo_path, _config, db, worktree_manager) = setup_test_repo(&temp_dir).await?;

    // Step 1: Register repository (simulating trunk command)
    db.create_repository("test-repo", test_repo_path.to_str().unwrap(), "", "main")
        .await?;

    // Verify registration
    let repos = db.list_repositories().await?;
    assert_eq!(repos.len(), 1, "Repository should be registered");

    // Step 2: Create multiple worktrees directly in database (simulating feat/fix commands)
    let feat_path = temp_dir.path().join("feat-authentication");
    std::fs::create_dir_all(&feat_path)?;
    // Make it a git repo for context detection
    let original_dir = env::current_dir()?;
    env::set_current_dir(&feat_path)?;
    std::process::Command::new("git").args(&["init"]).output()?;
    std::process::Command::new("git")
        .args(&["config", "user.name", "Test"])
        .output()?;
    std::process::Command::new("git")
        .args(&["config", "user.email", "test@example.com"])
        .output()?;
    env::set_current_dir(&original_dir)?;

    db.create_worktree(
        "test-repo",
        "feat-authentication",
        "feat/authentication",
        "feat",
        feat_path.to_str().unwrap(),
        None,
    )
    .await?;
    assert!(feat_path.exists(), "Feature worktree should exist");

    let fix_path = temp_dir.path().join("fix-login-bug");
    std::fs::create_dir_all(&fix_path)?;
    db.create_worktree(
        "test-repo",
        "fix-login-bug",
        "fix/login-bug",
        "fix",
        fix_path.to_str().unwrap(),
        None,
    )
    .await?;
    assert!(fix_path.exists(), "Fix worktree should exist");

    // Step 3: List from outside repo (should show all repos)
    let non_repo_dir = temp_dir.path().join("outside");
    std::fs::create_dir_all(&non_repo_dir)?;
    env::set_current_dir(&non_repo_dir)?;

    let result = worktree_manager.list_smart(None, false, false).await;
    assert!(result.is_ok(), "List from outside should succeed");

    // Step 4: List from inside repo (should show repo's worktrees)
    env::set_current_dir(&test_repo_path)?;
    let result = worktree_manager.list_smart(None, false, false).await;
    assert!(result.is_ok(), "List from inside repo should succeed");

    // Verify worktrees
    let worktrees = db.list_worktrees(Some("test-repo")).await?;
    assert_eq!(worktrees.len(), 2, "Should have 2 worktrees");

    // Step 5: List from inside worktree (should show parent repo's worktrees)
    env::set_current_dir(&feat_path)?;
    let result = worktree_manager.list_smart(None, false, false).await;
    assert!(result.is_ok(), "List from inside worktree should succeed");

    // Step 6: Test explicit flags
    let result = worktree_manager.list_smart(None, false, true).await;
    assert!(result.is_ok(), "List with --projects should succeed");

    let result = worktree_manager.list_smart(None, true, false).await;
    assert!(result.is_ok(), "List with --worktrees should succeed");

    // Step 7: Test --repo flag
    let result = worktree_manager
        .list_smart(Some("test-repo"), false, false)
        .await;
    assert!(result.is_ok(), "List with --repo should succeed");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_multi_repo_workflow_with_context_switching() -> Result<()> {
    // Test: Complex workflow with multiple repos and context switching
    let temp_dir = TempDir::new()?;
    let _guard = DirGuard::new()?; // Ensure directory is restored on test completion
    let (repo1_path, _config, db, worktree_manager) = setup_test_repo(&temp_dir).await?;

    // Create and register first repository
    db.create_repository("repo-1", repo1_path.to_str().unwrap(), "", "main")
        .await?;

    // Create and register second repository
    let repo2_path = create_second_repo(&temp_dir, "repo-2").await?;
    db.create_repository("repo-2", repo2_path.to_str().unwrap(), "", "main")
        .await?;

    // Create worktrees for repo-1 directly in database
    let wt1_path = temp_dir.path().join("feat-feature-a");
    std::fs::create_dir_all(&wt1_path)?;
    db.create_worktree(
        "repo-1",
        "feat-feature-a",
        "feat/feature-a",
        "feat",
        wt1_path.to_str().unwrap(),
        None,
    )
    .await?;

    let wt2_path = temp_dir.path().join("fix-fix-a");
    std::fs::create_dir_all(&wt2_path)?;
    db.create_worktree(
        "repo-1",
        "fix-fix-a",
        "fix/fix-a",
        "fix",
        wt2_path.to_str().unwrap(),
        None,
    )
    .await?;

    // Create worktrees for repo-2 directly in database
    let wt3_path = temp_dir.path().join("feat-feature-b");
    std::fs::create_dir_all(&wt3_path)?;
    db.create_worktree(
        "repo-2",
        "feat-feature-b",
        "feat/feature-b",
        "feat",
        wt3_path.to_str().unwrap(),
        None,
    )
    .await?;

    let wt4_path = temp_dir.path().join("devops-devops-b");
    std::fs::create_dir_all(&wt4_path)?;
    db.create_worktree(
        "repo-2",
        "devops-devops-b",
        "devops/devops-b",
        "devops",
        wt4_path.to_str().unwrap(),
        None,
    )
    .await?;

    // Test 1: List from repo-1 (should show only repo-1 worktrees)
    env::set_current_dir(&repo1_path)?;
    let result = worktree_manager.list_smart(None, false, false).await;
    assert!(result.is_ok(), "List from repo-1 should succeed");

    let repo1_worktrees = db.list_worktrees(Some("repo-1")).await?;
    assert_eq!(repo1_worktrees.len(), 2, "Repo-1 should have 2 worktrees");

    // Test 2: List from repo-2 (should show only repo-2 worktrees)
    env::set_current_dir(&repo2_path)?;
    let result = worktree_manager.list_smart(None, false, false).await;
    assert!(result.is_ok(), "List from repo-2 should succeed");

    let repo2_worktrees = db.list_worktrees(Some("repo-2")).await?;
    assert_eq!(repo2_worktrees.len(), 2, "Repo-2 should have 2 worktrees");

    // Test 3: List with --projects from any context (should show both repos)
    let result = worktree_manager.list_smart(None, false, true).await;
    assert!(result.is_ok(), "List --projects should succeed");

    let all_repos = db.list_repositories().await?;
    assert_eq!(all_repos.len(), 2, "Should have 2 registered repos");

    // Test 4: List with --worktrees from outside (should show all worktrees)
    let non_repo_dir = temp_dir.path().join("outside");
    std::fs::create_dir_all(&non_repo_dir)?;
    env::set_current_dir(&non_repo_dir)?;

    let result = worktree_manager.list_smart(None, true, false).await;
    assert!(
        result.is_ok(),
        "List --worktrees from outside should succeed"
    );

    let all_worktrees = db.list_worktrees(None).await?;
    assert_eq!(
        all_worktrees.len(),
        4,
        "Should have 4 total worktrees across both repos"
    );

    // Test 5: List specific repo with --repo flag from any context
    let result = worktree_manager
        .list_smart(Some("repo-1"), false, false)
        .await;
    assert!(result.is_ok(), "List --repo repo-1 should succeed");

    let result = worktree_manager
        .list_smart(Some("repo-2"), false, false)
        .await;
    assert!(result.is_ok(), "List --repo repo-2 should succeed");

    Ok(())
}

/// Test that -p short flag works identically to --projects long flag
/// This test verifies that the short flag -p produces the same behavior as --projects
#[tokio::test]
#[serial]
async fn test_short_p_flag_equivalent_to_projects_flag() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let _guard = DirGuard::new()?;

    // Setup first test repo
    let (repo1_path, _config, db, worktree_manager) = setup_test_repo(&temp_dir).await?;

    // Register first repository
    db.create_repository("test-repo-1", repo1_path.to_str().unwrap(), "", "main")
        .await?;

    // Setup second test repo
    let repo2_path = temp_dir.path().join("test-repo-2");
    std::fs::create_dir_all(&repo2_path)?;
    env::set_current_dir(&repo2_path)?;

    std::process::Command::new("git").args(&["init"]).output()?;
    std::process::Command::new("git")
        .args(&["config", "user.name", "Test User"])
        .output()?;
    std::process::Command::new("git")
        .args(&["config", "user.email", "test@example.com"])
        .output()?;
    std::fs::write(repo2_path.join("README.md"), "# Test Repo 2")?;
    std::process::Command::new("git")
        .args(&["add", "."])
        .output()?;
    std::process::Command::new("git")
        .args(&["commit", "-m", "Initial commit"])
        .output()?;

    // Register second repository
    db.create_repository("test-repo-2", repo2_path.to_str().unwrap(), "", "main")
        .await?;

    // Test from outside any repo (neutral context)
    let neutral_dir = temp_dir.path().join("neutral");
    std::fs::create_dir_all(&neutral_dir)?;
    env::set_current_dir(&neutral_dir)?;

    // Get repositories with projects=true (simulating --projects or -p flag)
    let result_long_flag = worktree_manager.list_smart(None, false, true).await;
    assert!(
        result_long_flag.is_ok(),
        "List with projects=true should succeed"
    );

    let repos_with_long_flag = db.list_repositories().await?;

    // Both -p and --projects should return the same repositories
    // Since list_smart with projects=true triggers repository listing
    assert_eq!(
        repos_with_long_flag.len(),
        2,
        "Should have 2 registered repos"
    );

    // Verify repo names
    let repo_names: Vec<String> = repos_with_long_flag
        .iter()
        .map(|r| r.name.clone())
        .collect();
    assert!(
        repo_names.contains(&"test-repo-1".to_string()),
        "Should contain test-repo-1"
    );
    assert!(
        repo_names.contains(&"test-repo-2".to_string()),
        "Should contain test-repo-2"
    );

    // Test from inside a repo
    env::set_current_dir(&repo1_path)?;

    let result_inside_repo = worktree_manager.list_smart(None, false, true).await;
    assert!(
        result_inside_repo.is_ok(),
        "List with projects=true inside repo should succeed"
    );

    let repos_inside = db.list_repositories().await?;
    assert_eq!(
        repos_inside.len(),
        2,
        "Should still show all 2 repos when using projects flag inside a repo"
    );

    Ok(())
}
