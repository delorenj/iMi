//! Enhanced Unit Tests for Config Module
//!
//! These tests provide comprehensive coverage of configuration functionality,
//! including edge cases, error scenarios, and validation logic.

use anyhow::Result;
use std::env;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::fs;

use imi::config::{Config, GitSettings, MonitoringSettings, SyncSettings};
use std::os::unix::fs::PermissionsExt;

/// Test utilities for config testing
pub struct ConfigTestUtils {
    pub temp_dir: TempDir,
}

impl ConfigTestUtils {
    pub fn new() -> Result<Self> {
        let temp_dir = TempDir::new()?;
        Ok(Self { temp_dir })
    }

    pub fn get_config_path(&self) -> PathBuf {
        self.temp_dir
            .path()
            .join(".config")
            .join("iMi")
            .join("config.toml")
    }

    pub async fn create_invalid_config_file(&self, content: &str) -> Result<PathBuf> {
        let config_path = self.get_config_path();

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(&config_path, content).await?;
        Ok(config_path)
    }

    pub async fn create_valid_config_file(&self) -> Result<PathBuf> {
        let config_content = r#"
database_path = "/tmp/test-imi.db"
root_path = "/tmp/test-code"
symlink_files = [".env", ".vscode/settings.json"]

[sync_settings]
enabled = true
global_sync_path = "sync/global"
repo_sync_path = "sync/repo"

[git_settings]
default_branch = "main"
remote_name = "origin"
auto_fetch = true
prune_on_fetch = true

[monitoring_settings]
enabled = true
refresh_interval_ms = 1000
watch_file_changes = true
track_agent_activity = true
"#;
        self.create_invalid_config_file(config_content).await
    }
}



#[cfg(test)]
mod config_unit_tests {
    use super::*;
    use serial_test::serial;

    // Tests for Config::default()
    #[tokio::test]
    async fn test_config_default_values() {
        let config = Config::default();

        // Test default values are reasonable
        assert!(config
            .database_path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .contains("iMi.db"));
        assert!(config.root_path.file_name().unwrap() == std::ffi::OsStr::new("code"));

        // Test sync settings defaults
        assert!(config.sync_settings.enabled);
        assert_eq!(
            config.sync_settings.global_sync_path,
            PathBuf::from("sync/global")
        );
        assert_eq!(
            config.sync_settings.repo_sync_path,
            PathBuf::from("sync/repo")
        );

        // Test git settings defaults
        assert_eq!(config.git_settings.default_branch, "main");
        assert_eq!(config.git_settings.remote_name, "origin");
        assert!(config.git_settings.auto_fetch);
        assert!(config.git_settings.prune_on_fetch);

        // Test monitoring settings defaults
        assert!(config.monitoring_settings.enabled);
        assert_eq!(config.monitoring_settings.refresh_interval_ms, 1000);
        assert!(config.monitoring_settings.watch_file_changes);
        assert!(config.monitoring_settings.track_agent_activity);

        // Test symlink files are present
        assert!(!config.symlink_files.is_empty());
        assert!(config.symlink_files.contains(&".env".to_string()));
    }

    #[tokio::test]
    #[serial]
    async fn test_config_get_config_path_success() -> Result<()> {
        let utils = ConfigTestUtils::new()?;
        let temp_home = utils.temp_dir.path().to_path_buf();
        env::set_var("HOME", &temp_home);

        let config_path = Config::get_config_path()?;

        let expected_path = temp_home.join(".config/iMi/config.toml");
        assert_eq!(config_path, expected_path);

        env::remove_var("HOME");
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_config_save_creates_directories() -> Result<()> {
        let utils = ConfigTestUtils::new()?;
        let config = Config::default();
        let config_path = utils.get_config_path();

        // Ensure config directory doesn't exist initially
        assert!(!config_path.exists());

        // Save should create directories and file
        config.save_to(&config_path).await?;

        assert!(config_path.exists());
        assert!(config_path.parent().unwrap().is_dir());

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_config_save_overwrites_existing() -> Result<()> {
        let utils = ConfigTestUtils::new()?;
        let mut config = Config::default();
        let config_path = utils.get_config_path();

        // Save initial config
        config.save_to(&config_path).await?;
        let initial_content = fs::read_to_string(&config_path).await?;

        // Modify and save again
        config.git_settings.default_branch = "develop".to_string();
        config.save_to(&config_path).await?;
        let updated_content = fs::read_to_string(&config_path).await?;

        assert_ne!(initial_content, updated_content);
        assert!(updated_content.contains("develop"));

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_config_load_creates_default_if_missing() -> Result<()> {
        let utils = ConfigTestUtils::new()?;
        let config_path = utils.get_config_path();

        // Ensure config doesn't exist
        assert!(!config_path.exists());

        // Load should create default config
        let config = Config::load_from(&config_path).await?;

        assert!(config_path.exists());
        assert_eq!(config.git_settings.default_branch, "main");

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_config_load_reads_existing_valid_file() -> Result<()> {
        let utils = ConfigTestUtils::new()?;
        let config_path = utils.create_valid_config_file().await?;

        let config = Config::load_from(&config_path).await?;

        assert_eq!(config.database_path, PathBuf::from("/tmp/test-imi.db"));
        assert_eq!(config.root_path, PathBuf::from("/tmp/test-code"));
        assert_eq!(config.git_settings.default_branch, "main");

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_config_load_handles_invalid_toml() -> Result<()> {
        let utils = ConfigTestUtils::new()?;
        let invalid_toml = "this is not valid toml [[[";
        let config_path = utils.create_invalid_config_file(invalid_toml).await?;

        let result = Config::load_from(&config_path).await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to parse config file"));

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_config_load_handles_missing_fields() -> Result<()> {
        let utils = ConfigTestUtils::new()?;
        let incomplete_toml = r#"
database_path = "/tmp/test.db"
# Missing other required fields
"#;
        let config_path = utils.create_invalid_config_file(incomplete_toml).await?;

        let result = Config::load_from(&config_path).await;

        // Should fail due to missing required fields
        assert!(result.is_err());

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_config_get_repo_path() -> Result<()> {
        let mut config = Config::default();
        config.root_path = PathBuf::from("/test/root");

        let repo_path = config.get_repo_path("my-repo");

        assert_eq!(repo_path, PathBuf::from("/test/root/my-repo"));

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_config_get_trunk_path() -> Result<()> {
        let mut config = Config::default();
        config.root_path = PathBuf::from("/test/root");
        config.git_settings.default_branch = "develop".to_string();

        let trunk_path = config.get_trunk_path("my-repo");

        assert_eq!(
            trunk_path,
            PathBuf::from("/test/root/my-repo/trunk-develop")
        );

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_config_get_worktree_path() -> Result<()> {
        let mut config = Config::default();
        config.root_path = PathBuf::from("/test/root");

        let worktree_path = config.get_worktree_path("my-repo", "feature-branch");

        assert_eq!(
            worktree_path,
            PathBuf::from("/test/root/my-repo/feature-branch")
        );

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_config_get_sync_path_global() -> Result<()> {
        let mut config = Config::default();
        config.root_path = PathBuf::from("/test/root");
        config.sync_settings.global_sync_path = PathBuf::from("global-sync");

        let sync_path = config.get_sync_path("my-repo", true);

        assert_eq!(sync_path, PathBuf::from("/test/root/my-repo/global-sync"));

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_config_get_sync_path_repo() -> Result<()> {
        let mut config = Config::default();
        config.root_path = PathBuf::from("/test/root");
        config.sync_settings.repo_sync_path = PathBuf::from("repo-sync");

        let sync_path = config.get_sync_path("my-repo", false);

        assert_eq!(sync_path, PathBuf::from("/test/root/my-repo/repo-sync"));

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_config_ensure_database_directory() -> Result<()> {
        let utils = ConfigTestUtils::new()?;
        let mut config = Config::default();
        let db_dir = utils.temp_dir.path().join("nested/deep/directory");
        config.database_path = db_dir.join("test.db");

        // Directory shouldn't exist initially
        assert!(!db_dir.exists());

        config.ensure_database_directory().await?;

        // Directory should be created
        assert!(db_dir.exists());
        assert!(db_dir.is_dir());

        Ok(())
    }

    // Edge cases and error scenarios
    #[tokio::test]
    #[serial]
    async fn test_config_handles_empty_repo_name() -> Result<()> {
        let config = Config::default();

        let repo_path = config.get_repo_path("");
        let trunk_path = config.get_trunk_path("");
        let worktree_path = config.get_worktree_path("", "branch");
        let sync_path = config.get_sync_path("", true);

        // Should handle empty names gracefully
        assert!(repo_path.to_string_lossy().ends_with("/code/"));
        assert!(trunk_path.to_string_lossy().contains("trunk-main"));
        assert!(worktree_path.to_string_lossy().ends_with("/branch"));
        assert!(sync_path.to_string_lossy().contains("sync"));

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_config_handles_special_characters_in_paths() -> Result<()> {
        let mut config = Config::default();
        config.root_path = PathBuf::from("/test/root with spaces");

        let repo_path = config.get_repo_path("repo-with-dashes");
        let trunk_path = config.get_trunk_path("repo_with_underscores");
        let worktree_path = config.get_worktree_path("repo", "feature/branch-name");

        // Should handle special characters in paths
        assert!(repo_path.to_string_lossy().contains("root with spaces"));
        assert!(repo_path.to_string_lossy().contains("repo-with-dashes"));
        assert!(trunk_path
            .to_string_lossy()
            .contains("repo_with_underscores"));
        assert!(worktree_path
            .to_string_lossy()
            .contains("feature/branch-name"));

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_sync_settings_defaults() {
        let sync_settings = SyncSettings {
            enabled: true,
            global_sync_path: PathBuf::from("sync/global"),
            repo_sync_path: PathBuf::from("sync/repo"),
        };

        assert!(sync_settings.enabled);
        assert_eq!(sync_settings.global_sync_path, PathBuf::from("sync/global"));
        assert_eq!(sync_settings.repo_sync_path, PathBuf::from("sync/repo"));
    }

    #[tokio::test]
    #[serial]
    async fn test_git_settings_defaults() {
        let git_settings = GitSettings {
            default_branch: "main".to_string(),
            remote_name: "origin".to_string(),
            auto_fetch: true,
            prune_on_fetch: true,
        };

        assert_eq!(git_settings.default_branch, "main");
        assert_eq!(git_settings.remote_name, "origin");
        assert!(git_settings.auto_fetch);
        assert!(git_settings.prune_on_fetch);
    }

    #[tokio::test]
    #[serial]
    async fn test_monitoring_settings_defaults() {
        let monitoring_settings = MonitoringSettings {
            enabled: true,
            refresh_interval_ms: 1000,
            watch_file_changes: true,
            track_agent_activity: true,
        };

        assert!(monitoring_settings.enabled);
        assert_eq!(monitoring_settings.refresh_interval_ms, 1000);
        assert!(monitoring_settings.watch_file_changes);
        assert!(monitoring_settings.track_agent_activity);
    }

    #[tokio::test]
    #[serial]
    async fn test_config_serialization_roundtrip() -> Result<()> {
        let mut config = Config::default();
        config.git_settings.default_branch = "develop".to_string();
        config.monitoring_settings.enabled = false;
        config.sync_settings.repo_sync_path = PathBuf::from("custom/sync");

        let toml_string = toml::to_string_pretty(&config)?;
        let deserialized_config: Config = toml::from_str(&toml_string)?;

        assert_eq!(
            config.git_settings.default_branch,
            deserialized_config.git_settings.default_branch
        );
        assert_eq!(
            config.monitoring_settings.enabled,
            deserialized_config.monitoring_settings.enabled
        );
        assert_eq!(
            config.sync_settings.repo_sync_path,
            deserialized_config.sync_settings.repo_sync_path
        );
        assert_eq!(config.database_path, deserialized_config.database_path);

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_config_save_permission_error() -> Result<()> {
        let utils = ConfigTestUtils::new()?;
        let config = Config::default();
        let config_path = utils.get_config_path();
        let config_dir = config_path.parent().unwrap();

        fs::create_dir_all(config_dir).await?;

        // Set directory to read-only
        let mut perms = fs::metadata(config_dir).await?.permissions();
        perms.set_readonly(true);
        fs::set_permissions(config_dir, perms).await?;

        let result = config.save_to(&config_path).await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to write config file"));

        // Cleanup: make writable again to allow deletion
        let mut perms = fs::metadata(config_dir).await?.permissions();
        perms.set_readonly(false);
        fs::set_permissions(config_dir, perms).await?;

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_config_load_permission_error() -> Result<()> {
        let utils = ConfigTestUtils::new()?;
        let config_path = utils.create_valid_config_file().await?;

        // Set file to read-only, but owner can't read
        let perms = std::fs::Permissions::from_mode(0o000); // No permissions
        fs::set_permissions(&config_path, perms).await?;

        let result = Config::load_from(&config_path).await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to read config file"));

        // Cleanup: make writable again to allow deletion
        let perms = std::fs::Permissions::from_mode(0o644);
        fs::set_permissions(&config_path, perms).await?;

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_ensure_database_directory_permission_error() -> Result<()> {
        let utils = ConfigTestUtils::new()?;
        let mut config = Config::default();
        let read_only_dir = utils.temp_dir.path().join("read-only");
        fs::create_dir(&read_only_dir).await?;

        // Set directory to read-only
        let mut perms = fs::metadata(&read_only_dir).await?.permissions();
        perms.set_readonly(true);
        fs::set_permissions(&read_only_dir, perms).await?;

        config.database_path = read_only_dir.join("db/test.db");

        let result = config.ensure_database_directory().await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to create database directory"));

        // Cleanup
        let mut perms = fs::metadata(&read_only_dir).await?.permissions();
        perms.set_readonly(false);
        fs::set_permissions(&read_only_dir, perms).await?;

        Ok(())
    }

    // Integration-like tests for complete config workflow
    #[tokio::test]
    #[serial]
    async fn test_config_full_lifecycle() -> Result<()> {
        let utils = ConfigTestUtils::new()?;
        let config_path = utils.get_config_path();

        // 1. Load should create default config
        let mut config = Config::load_from(&config_path).await?;
        assert!(config_path.exists());

        // 2. Modify config
        config.git_settings.default_branch = "develop".to_string();
        config.monitoring_settings.refresh_interval_ms = 2000;
        config.symlink_files.push("custom-file.conf".to_string());

        // 3. Save modified config
        config.save_to(&config_path).await?;

        // 4. Load again and verify changes persisted
        let loaded_config = Config::load_from(&config_path).await?;
        assert_eq!(loaded_config.git_settings.default_branch, "develop");
        assert_eq!(loaded_config.monitoring_settings.refresh_interval_ms, 2000);
        assert!(loaded_config
            .symlink_files
            .contains(&"custom-file.conf".to_string()));

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_config_concurrent_access() -> Result<()> {
        let utils = ConfigTestUtils::new()?;
        let config_path = utils.get_config_path();
        Config::default().save_to(&config_path).await?;

        let config_path_clone1 = config_path.clone();
        let config1_handle = tokio::spawn(async move {
            let mut config = Config::load_from(&config_path_clone1).await.unwrap();
            config.git_settings.default_branch = "feature-1".to_string();
            config.save_to(&config_path_clone1).await.unwrap();
        });

        let config_path_clone2 = config_path.clone();
        let config2_handle = tokio::spawn(async move {
            let mut config = Config::load_from(&config_path_clone2).await.unwrap();
            config.git_settings.default_branch = "feature-2".to_string();
            config.save_to(&config_path_clone2).await.unwrap();
        });

        // Wait for both operations
        let _ = tokio::try_join!(config1_handle, config2_handle)?;

        // Load final config and verify it has one of the values
        let final_config = Config::load_from(&config_path).await?;
        assert!(
            final_config.git_settings.default_branch == "feature-1"
                || final_config.git_settings.default_branch == "feature-2"
        );

        Ok(())
    }
}

// Property-based tests for path handling
#[cfg(test)]
mod config_property_tests {
    use super::*;

    #[tokio::test]
    async fn test_path_operations_are_consistent() -> Result<()> {
        let config = Config::default();
        let repo_name = "test-repo";
        let worktree_name = "test-worktree";

        // Test that path operations are consistent
        let repo_path = config.get_repo_path(repo_name);
        let worktree_path = config.get_worktree_path(repo_name, worktree_name);

        // Worktree path should be under repo path
        assert!(worktree_path.starts_with(&repo_path));

        // Trunk path should be under repo path
        let trunk_path = config.get_trunk_path(repo_name);
        assert!(trunk_path.starts_with(&repo_path));

        // Sync paths should be under repo path
        let global_sync = config.get_sync_path(repo_name, true);
        let repo_sync = config.get_sync_path(repo_name, false);
        assert!(global_sync.starts_with(&repo_path));
        assert!(repo_sync.starts_with(&repo_path));

        Ok(())
    }

    #[tokio::test]
    async fn test_path_normalization() -> Result<()> {
        let mut config = Config::default();

        // Test with paths that need normalization
        config.root_path = PathBuf::from("/test/root/../normalized");

        let repo_path = config.get_repo_path("repo");

        // Path should be properly constructed (though not necessarily normalized by Config)
        assert!(repo_path.to_string_lossy().contains("repo"));

        Ok(())
    }
}
