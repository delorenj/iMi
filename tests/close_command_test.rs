/// Integration tests for the `iMi close` command
///
/// This test file validates that the close command:
/// 1. Removes the worktree directory
/// 2. Removes the git worktree reference
/// 3. Updates the database to reflect the worktree is no longer active
/// 4. Preserves the branch (does not delete it)
/// 5. Handles edge cases gracefully
use anyhow::Result;
use std::env;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio;

// Import the necessary modules from the main crate
use imi::cli::{Cli, Commands};
use imi::config::Config;
use imi::database::Database;
use imi::git::GitManager;
use imi::worktree::WorktreeManager;

#[tokio::test]
async fn test_close_command_basic() -> Result<()> {
    // Setup test environment
    let temp_dir = TempDir::new()?;
    let test_repo_path = temp_dir.path().join("test-repo");

    // Initialize a git repository
    let git_manager = GitManager::new();
    std::fs::create_dir_all(&test_repo_path)?;
    env::set_current_dir(&test_repo_path)?;

    // Initialize git repo
    let output = std::process::Command::new("git").args(&["init"]).output()?;
    assert!(output.status.success());

    // Create initial commit
    std::fs::write(test_repo_path.join("README.md"), "# Test Repo")?;
    std::process::Command::new("git")
        .args(&["add", "."])
        .output()?;
    std::process::Command::new("git")
        .args(&["commit", "-m", "Initial commit"])
        .output()?;

    // Setup config and database
    let config = Config {
        database_path: temp_dir.path().join("imi.db"),
        ..Default::default()
    };

    let db = Database::new(&config.database_path).await?;
    let worktree_manager = WorktreeManager::new(git_manager, db.clone(), config.clone(), None);

    // Register the repository
    db.create_repository("test-repo", test_repo_path.to_str().unwrap(), "", "main")
        .await?;

    // Create a feature worktree
    let worktree_path = worktree_manager
        .create_feature_worktree("test-feature", Some("test-repo"))
        .await?;

    // Verify worktree was created
    assert!(worktree_path.exists());
    assert!(db
        .get_worktree("test-repo", "feat-test-feature")
        .await?
        .is_some());

    // Get the git repository
    let repo = git2::Repository::open(&test_repo_path)?;

    // Verify branch exists before closing
    let branch_exists_before = repo
        .find_branch("feat/test-feature", git2::BranchType::Local)
        .is_ok();
    assert!(branch_exists_before, "Branch should exist before closing");

    // Now close the worktree
    worktree_manager
        .close_worktree("feat-test-feature", Some("test-repo"))
        .await?;

    // Verify worktree directory was removed
    assert!(
        !worktree_path.exists(),
        "Worktree directory should be removed"
    );

    // Verify database was updated
    let db_entry = db.get_worktree("test-repo", "feat-test-feature").await?;
    assert!(db_entry.is_none(), "Database entry should be deactivated");

    // Verify branch still exists (not deleted)
    let branch_exists_after = repo
        .find_branch("feat/test-feature", git2::BranchType::Local)
        .is_ok();
    assert!(
        branch_exists_after,
        "Branch should still exist after closing"
    );

    // Verify git worktree reference was removed
    let worktrees = repo.worktrees()?;
    let worktree_exists = worktrees.iter().flatten().any(|w| w == "feat-test-feature");
    assert!(!worktree_exists, "Git worktree reference should be removed");

    Ok(())
}

#[tokio::test]
async fn test_close_command_with_name_variations() -> Result<()> {
    // Test that close command works with various name formats
    let temp_dir = TempDir::new()?;
    let test_repo_path = temp_dir.path().join("test-repo");

    // Initialize a git repository
    let git_manager = GitManager::new();
    std::fs::create_dir_all(&test_repo_path)?;
    env::set_current_dir(&test_repo_path)?;

    // Initialize git repo
    std::process::Command::new("git").args(&["init"]).output()?;
    std::fs::write(test_repo_path.join("README.md"), "# Test Repo")?;
    std::process::Command::new("git")
        .args(&["add", "."])
        .output()?;
    std::process::Command::new("git")
        .args(&["commit", "-m", "Initial commit"])
        .output()?;

    // Setup config and database
    let config = Config {
        database_path: temp_dir.path().join("imi.db"),
        ..Default::default()
    };

    let db = Database::new(&config.database_path).await?;
    let worktree_manager = WorktreeManager::new(git_manager, db.clone(), config.clone(), None);

    // Register the repository
    db.create_repository("test-repo", test_repo_path.to_str().unwrap(), "", "main")
        .await?;

    // Create a feature worktree
    let worktree_path = worktree_manager
        .create_feature_worktree("my-feature", Some("test-repo"))
        .await?;

    assert!(worktree_path.exists());

    // Test closing with just the feature name (without prefix)
    let result = worktree_manager
        .close_worktree("my-feature", Some("test-repo"))
        .await;
    assert!(
        result.is_ok(),
        "Should be able to close with just feature name"
    );
    assert!(!worktree_path.exists(), "Worktree should be removed");

    // Create another worktree
    let worktree_path2 = worktree_manager
        .create_fix_worktree("my-fix", Some("test-repo"))
        .await?;

    // Test closing with full prefixed name
    let result = worktree_manager
        .close_worktree("fix-my-fix", Some("test-repo"))
        .await;
    assert!(result.is_ok(), "Should be able to close with prefixed name");
    assert!(!worktree_path2.exists(), "Worktree should be removed");

    Ok(())
}

#[tokio::test]
async fn test_close_nonexistent_worktree() -> Result<()> {
    // Test that closing a non-existent worktree doesn't crash
    let temp_dir = TempDir::new()?;
    let test_repo_path = temp_dir.path().join("test-repo");

    // Initialize a git repository
    let git_manager = GitManager::new();
    std::fs::create_dir_all(&test_repo_path)?;
    env::set_current_dir(&test_repo_path)?;

    std::process::Command::new("git").args(&["init"]).output()?;
    std::fs::write(test_repo_path.join("README.md"), "# Test Repo")?;
    std::process::Command::new("git")
        .args(&["add", "."])
        .output()?;
    std::process::Command::new("git")
        .args(&["commit", "-m", "Initial commit"])
        .output()?;

    // Setup config and database
    let config = Config {
        database_path: temp_dir.path().join("imi.db"),
        ..Default::default()
    };

    let db = Database::new(&config.database_path).await?;
    let worktree_manager = WorktreeManager::new(git_manager, db.clone(), config.clone(), None);

    // Register the repository
    db.create_repository("test-repo", test_repo_path.to_str().unwrap(), "", "main")
        .await?;

    // Try to close a non-existent worktree
    let result = worktree_manager
        .close_worktree("nonexistent", Some("test-repo"))
        .await;

    // Should complete without error (gracefully handle non-existent worktrees)
    assert!(
        result.is_ok(),
        "Should handle non-existent worktree gracefully"
    );

    Ok(())
}

#[tokio::test]
async fn test_close_vs_remove_difference() -> Result<()> {
    // Verify that close preserves branches while remove deletes them
    let temp_dir = TempDir::new()?;
    let test_repo_path = temp_dir.path().join("test-repo");

    // Initialize a git repository
    let git_manager = GitManager::new();
    std::fs::create_dir_all(&test_repo_path)?;
    env::set_current_dir(&test_repo_path)?;

    std::process::Command::new("git").args(&["init"]).output()?;
    std::fs::write(test_repo_path.join("README.md"), "# Test Repo")?;
    std::process::Command::new("git")
        .args(&["add", "."])
        .output()?;
    std::process::Command::new("git")
        .args(&["commit", "-m", "Initial commit"])
        .output()?;

    // Setup config and database
    let config = Config {
        database_path: temp_dir.path().join("imi.db"),
        ..Default::default()
    };

    let db = Database::new(&config.database_path).await?;
    let worktree_manager = WorktreeManager::new(git_manager, db.clone(), config.clone(), None);

    // Register the repository
    db.create_repository("test-repo", test_repo_path.to_str().unwrap(), "", "main")
        .await?;

    // Create two worktrees - one to close, one to remove
    let close_worktree_path = worktree_manager
        .create_feature_worktree("to-close", Some("test-repo"))
        .await?;

    let remove_worktree_path = worktree_manager
        .create_feature_worktree("to-remove", Some("test-repo"))
        .await?;

    let repo = git2::Repository::open(&test_repo_path)?;

    // Close the first worktree
    worktree_manager
        .close_worktree("to-close", Some("test-repo"))
        .await?;

    // Remove the second worktree (without keeping branch)
    worktree_manager
        .remove_worktree("to-remove", Some("test-repo"), false, false)
        .await?;

    // Verify close preserved the branch
    let close_branch_exists = repo
        .find_branch("feat/to-close", git2::BranchType::Local)
        .is_ok();
    assert!(close_branch_exists, "Close should preserve the branch");

    // Verify remove deleted the branch
    let remove_branch_exists = repo
        .find_branch("feat/to-remove", git2::BranchType::Local)
        .is_ok();
    assert!(
        !remove_branch_exists,
        "Remove should delete the branch when keep_branch is false"
    );

    // Both worktree directories should be removed
    assert!(
        !close_worktree_path.exists(),
        "Close should remove worktree directory"
    );
    assert!(
        !remove_worktree_path.exists(),
        "Remove should remove worktree directory"
    );

    Ok(())
}
