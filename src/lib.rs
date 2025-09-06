//! iMi - Git Worktree Management Tool
//! 
//! A sophisticated worktree management tool designed for asynchronous,
//! parallel multi-agent workflows with opinionated defaults and real-time visibility.

pub mod cli;
pub mod config;
pub mod database;
pub mod error;
pub mod git;
pub mod init;
pub mod monitor;
pub mod worktree;

// Re-export commonly used types
pub use config::Config;
pub use database::{Database, Repository, Worktree, AgentActivity};
pub use error::ImiError;
pub use git::{GitManager, WorktreeStatus};
pub use init::{InitCommand, InitResult};
pub use worktree::WorktreeManager;

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default configuration values
pub mod defaults {
    /// Default root directory for iMi repositories
    pub const DEFAULT_ROOT: &str = "~/code";
    
    /// Default database filename
    pub const DEFAULT_DB_NAME: &str = "iMi.db";
    
    /// Default config filename
    pub const DEFAULT_CONFIG_NAME: &str = "config.toml";
    
    /// Default branch name
    pub const DEFAULT_BRANCH: &str = "main";
    
    /// Default remote name
    pub const DEFAULT_REMOTE: &str = "origin";
}

/// Test utilities for integration testing
#[cfg(any(test, feature = "testing"))]
pub mod test_utils {
    use super::*;
    use anyhow::Result;
    use std::path::PathBuf;

    #[cfg(any(test, feature = "testing"))]
    pub use tempfile::TempDir;

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
}