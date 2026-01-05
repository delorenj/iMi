/// Test for the get_worktree_status fix
/// This verifies that uncommitted changes are correctly detected
use anyhow::Result;
use imi::git::GitManager;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_worktree_status_detects_uncommitted_changes() -> Result<()> {
    // Create a temporary directory for testing
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();

    // Initialize a git repository
    let repo = git2::Repository::init(repo_path)?;

    // Configure git user for commits
    let mut config = repo.config()?;
    config.set_str("user.name", "Test User")?;
    config.set_str("user.email", "test@example.com")?;

    // Create and commit an initial file
    let test_file = repo_path.join("test.txt");
    fs::write(&test_file, "initial content")?;

    let mut index = repo.index()?;
    index.add_path(std::path::Path::new("test.txt"))?;
    index.write()?;

    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    let sig = repo.signature()?;
    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])?;

    // Test 1: Clean repository should report clean status
    let git_manager = GitManager::new();
    let status = git_manager.get_worktree_status(repo_path)?;

    assert!(
        status.clean,
        "Expected clean status for unmodified repository, but got: \
         modified={}, new={}, deleted={}",
        status.modified_files.len(),
        status.new_files.len(),
        status.deleted_files.len()
    );
    assert_eq!(status.modified_files.len(), 0);
    assert_eq!(status.new_files.len(), 0);
    assert_eq!(status.deleted_files.len(), 0);

    // Test 2: Modified file should report dirty status
    fs::write(&test_file, "modified content")?;
    let status = git_manager.get_worktree_status(repo_path)?;

    assert!(
        !status.clean,
        "Expected dirty status for modified file, but got clean status"
    );
    assert_eq!(
        status.modified_files.len(),
        1,
        "Expected 1 modified file, got {}",
        status.modified_files.len()
    );
    assert_eq!(status.modified_files[0], "test.txt");

    // Restore file to clean state
    fs::write(&test_file, "initial content")?;
    repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))?;

    // Test 3: New untracked file should report dirty status
    let new_file = repo_path.join("new.txt");
    fs::write(&new_file, "new file content")?;
    let status = git_manager.get_worktree_status(repo_path)?;

    assert!(
        !status.clean,
        "Expected dirty status for new file, but got clean status"
    );
    assert_eq!(
        status.new_files.len(),
        1,
        "Expected 1 new file, got {}",
        status.new_files.len()
    );
    assert_eq!(status.new_files[0], "new.txt");

    // Clean up new file
    fs::remove_file(&new_file)?;

    // Test 4: Staged changes should report dirty status
    fs::write(&test_file, "staged changes")?;
    let mut index = repo.index()?;
    index.add_path(std::path::Path::new("test.txt"))?;
    index.write()?;

    let status = git_manager.get_worktree_status(repo_path)?;
    assert!(
        !status.clean,
        "Expected dirty status for staged changes, but got clean status"
    );
    assert_eq!(
        status.modified_files.len(),
        1,
        "Expected 1 modified file (staged), got {}",
        status.modified_files.len()
    );

    Ok(())
}

#[test]
fn test_worktree_status_multiple_changes() -> Result<()> {
    // Create a temporary directory for testing
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();

    // Initialize a git repository
    let repo = git2::Repository::init(repo_path)?;

    // Configure git user
    let mut config = repo.config()?;
    config.set_str("user.name", "Test User")?;
    config.set_str("user.email", "test@example.com")?;

    // Create and commit initial files
    let file1 = repo_path.join("file1.txt");
    let file2 = repo_path.join("file2.txt");
    fs::write(&file1, "content 1")?;
    fs::write(&file2, "content 2")?;

    let mut index = repo.index()?;
    index.add_path(std::path::Path::new("file1.txt"))?;
    index.add_path(std::path::Path::new("file2.txt"))?;
    index.write()?;

    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    let sig = repo.signature()?;
    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])?;

    // Create multiple types of changes
    fs::write(&file1, "modified content")?; // Modified file
    let file3 = repo_path.join("file3.txt");
    fs::write(&file3, "new content")?; // New file
    fs::remove_file(&file2)?; // Deleted file

    let git_manager = GitManager::new();
    let status = git_manager.get_worktree_status(repo_path)?;

    // Verify all changes are detected
    assert!(!status.clean, "Expected dirty status with multiple changes");
    assert_eq!(status.modified_files.len(), 1, "Expected 1 modified file");
    assert_eq!(status.new_files.len(), 1, "Expected 1 new file");
    assert_eq!(status.deleted_files.len(), 1, "Expected 1 deleted file");

    assert_eq!(status.modified_files[0], "file1.txt");
    assert_eq!(status.new_files[0], "file3.txt");
    assert_eq!(status.deleted_files[0], "file2.txt");

    Ok(())
}
