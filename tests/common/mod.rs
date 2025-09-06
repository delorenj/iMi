use anyhow::Result;
use std::path::PathBuf;
use tempfile::TempDir;
use imi::config::Config;
use imi::database::Database;
use imi::git::GitManager;

/// Create a test environment with temporary directory and default configuration
pub async fn setup_test_env() -> Result<(TempDir, Config, Database, GitManager)> {
    let temp_dir = TempDir::new()?;

    // Create test config with temp paths
    let mut config = Config::default();
    config.database_path = temp_dir.path().join("test.db");
    config.root_path = temp_dir.path().to_path_buf();

    let db = Database::new(&config.database_path).await?;
    let git = GitManager::new();

    Ok((temp_dir, config, db, git))
}

/// Create a mock repository structure for testing
pub async fn create_mock_repo_structure(
    base_path: &PathBuf,
    repo_name: &str,
    trunk_branch: &str,
) -> Result<(PathBuf, PathBuf)> {
    let repo_dir = base_path.join(repo_name);
    let trunk_dir = repo_dir.join(format!("trunk-{}", trunk_branch));

    tokio::fs::create_dir_all(&trunk_dir).await?;

    Ok((repo_dir, trunk_dir))
}
