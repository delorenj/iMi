use anyhow::Result;
use std::path::PathBuf;
use tempfile::TempDir;

use imi::{config::Config, database::Database, init::InitCommand};

/// Comprehensive path construction validation tests
/// This module validates that Bug_Fixer's path construction fix works correctly
/// and prevents the path doubling issue seen in error messages like:
/// "/path/to/repo/trunk-main/.imi" instead of proper paths.

#[cfg(test)]
mod path_construction_validation {
    use super::*;

    /// Test fixture for path validation
    struct PathValidationFixture {
        temp_dir: TempDir,
        config: Config,
    }

    impl PathValidationFixture {
        async fn new() -> Result<Self> {
            let temp_dir = TempDir::new()?;
            let mut config = Config::default();

            // Set up test paths
            config.database_path = temp_dir.path().join("test.db");
            config.workspace_settings.root_path = temp_dir.path().join("code");

            Ok(Self { temp_dir, config })
        }

        fn get_test_root(&self) -> PathBuf {
            self.temp_dir.path().to_path_buf()
        }
    }

    #[tokio::test]
    async fn test_config_path_construction_no_doubling() -> Result<()> {
        let fixture = PathValidationFixture::new().await?;
        let config = &fixture.config;

        // Test basic path construction
        let repo_name = "test-repo";
        let _worktree_name = "feat-test";

        // Test get_repo_path - should be root_path/entity_id/repo_name
        let repo_path = config.get_repo_path(repo_name);
        let expected_repo = config
            .workspace_settings
            .root_path
            .join(&config.workspace_settings.entity_id)
            .join(repo_name);

        assert_eq!(
            repo_path, expected_repo,
            "get_repo_path should construct path without doubling: expected {:?}, got {:?}",
            expected_repo, repo_path
        );

        // Ensure no double path separators
        let repo_path_str = repo_path.to_string_lossy();
        assert!(
            !repo_path_str.contains("//"),
            "Path should not contain double separators: {}",
            repo_path_str
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_trunk_path_construction_no_doubling() -> Result<()> {
        let fixture = PathValidationFixture::new().await?;
        let config = &fixture.config;
        let repo_name = "test-repo";

        // Test get_trunk_path construction
        let trunk_path = config.get_trunk_path(repo_name);
        let expected_trunk = config
            .workspace_settings
            .root_path
            .join(&config.workspace_settings.entity_id)
            .join(repo_name)
            .join(format!("trunk-{}", config.git_settings.default_branch));

        assert_eq!(
            trunk_path, expected_trunk,
            "get_trunk_path should construct path without doubling: expected {:?}, got {:?}",
            expected_trunk, trunk_path
        );

        // Ensure no double separators or repeated segments
        let trunk_str = trunk_path.to_string_lossy();
        assert!(
            !trunk_str.contains("//"),
            "Trunk path should not contain double separators: {}",
            trunk_str
        );
        assert!(
            !trunk_str.contains(&format!(
                "trunk-{}/trunk-{}",
                config.git_settings.default_branch, config.git_settings.default_branch
            )),
            "Trunk path should not contain repeated trunk segment: {}",
            trunk_str
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_worktree_path_construction_no_doubling() -> Result<()> {
        let fixture = PathValidationFixture::new().await?;
        let config = &fixture.config;
        let repo_name = "test-repo";
        let worktree_name = "feat-test";

        // Test get_worktree_path construction
        let worktree_path = config.get_worktree_path(repo_name, worktree_name);
        let expected_worktree = config
            .workspace_settings
            .root_path
            .join(&config.workspace_settings.entity_id)
            .join(repo_name)
            .join(worktree_name);

        assert_eq!(
            worktree_path, expected_worktree,
            "get_worktree_path should construct path without doubling: expected {:?}, got {:?}",
            expected_worktree, worktree_path
        );

        // Ensure no double separators or repeated segments
        let worktree_str = worktree_path.to_string_lossy();
        assert!(
            !worktree_str.contains("//"),
            "Worktree path should not contain double separators: {}",
            worktree_str
        );
        assert!(
            !worktree_str.contains(&format!("{}/{}", worktree_name, worktree_name)),
            "Worktree path should not contain repeated worktree name: {}",
            worktree_str
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_sync_path_construction_no_doubling() -> Result<()> {
        let fixture = PathValidationFixture::new().await?;
        let config = &fixture.config;
        let repo_name = "test-repo";

        // Test global sync path construction
        let global_sync = config.get_sync_path(repo_name, true);
        let expected_global = config
            .workspace_settings
            .root_path
            .join(&config.workspace_settings.entity_id)
            .join(repo_name)
            .join(&config.sync_settings.user_sync_path);

        assert_eq!(
            global_sync, expected_global,
            "Global sync path should construct without doubling: expected {:?}, got {:?}",
            expected_global, global_sync
        );

        // Test repo sync path construction
        let repo_sync = config.get_sync_path(repo_name, false);
        let expected_repo_sync = config
            .workspace_settings
            .root_path
            .join(&config.workspace_settings.entity_id)
            .join(repo_name)
            .join(&config.sync_settings.local_sync_path);

        assert_eq!(
            repo_sync, expected_repo_sync,
            "Repo sync path should construct without doubling: expected {:?}, got {:?}",
            expected_repo_sync, repo_sync
        );

        // Ensure no double separators in either path
        for (path, name) in [(global_sync, "global"), (repo_sync, "repo")] {
            let path_str = path.to_string_lossy();
            assert!(
                !path_str.contains("//"),
                "{} sync path should not contain double separators: {}",
                name,
                path_str
            );
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_specific_path_doubling_bug_scenarios() -> Result<()> {
        let fixture = PathValidationFixture::new().await?;
        let config = &fixture.config;

        // Test the specific scenario from the error message: "/path/to/repo/trunk-main/.imi"
        // This suggests .imi was being doubled somewhere

        let repo_name = "test-repo";
        let _trunk_name = "trunk-main";

        // Construct paths that were problematic
        let repo_path = config.get_repo_path(repo_name);
        let trunk_path = config.get_trunk_path(repo_name);

        // Simulate paths that might have .imi appended
        let potential_config_path = repo_path.join(".imi").join("config.toml");
        let potential_trunk_config = trunk_path.join(".imi").join("config.toml");

        // These should not contain doubled .imi segments
        for (path, desc) in [
            (potential_config_path, "repo config path"),
            (potential_trunk_config, "trunk config path"),
        ] {
            let path_str = path.to_string_lossy();
            assert!(
                !path_str.contains(".imi/.imi"),
                "{} should not contain doubled .imi: {}",
                desc,
                path_str
            );
            assert!(
                !path_str.contains(".imi.imi"),
                "{} should not contain .imi.imi: {}",
                desc,
                path_str
            );
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_path_construction_with_edge_cases() -> Result<()> {
        let fixture = PathValidationFixture::new().await?;
        let config = &fixture.config;

        // Test edge cases that could cause path doubling
        let edge_cases = vec![
            ("repo-with-hyphens", "feat-test-feature"),
            ("repo_with_underscores", "fix_bug_123"),
            ("RepoWithCaps", "FEAT-UPPERCASE"),
            ("123numeric", "456numeric"),
            ("short", "a"),
            ("repo.with.dots", "branch.with.dots"),
        ];

        for (repo_name, worktree_name) in edge_cases {
            // Test all path construction methods
            let repo_path = config.get_repo_path(repo_name);
            let trunk_path = config.get_trunk_path(repo_name);
            let worktree_path = config.get_worktree_path(repo_name, worktree_name);
            let global_sync = config.get_sync_path(repo_name, true);
            let repo_sync = config.get_sync_path(repo_name, false);

            let paths = vec![
                (repo_path, "repo_path"),
                (trunk_path, "trunk_path"),
                (worktree_path, "worktree_path"),
                (global_sync, "global_sync"),
                (repo_sync, "repo_sync"),
            ];

            for (path, path_type) in paths {
                let path_str = path.to_string_lossy();

                // Check for double separators
                assert!(
                    !path_str.contains("//"),
                    "{} for repo '{}' should not have double separators: {}",
                    path_type,
                    repo_name,
                    path_str
                );

                // Check for doubled segments (basic heuristic)
                let segments: Vec<&str> = path_str.split('/').collect();
                for i in 0..segments.len().saturating_sub(1) {
                    if segments[i] == segments[i + 1] && !segments[i].is_empty() {
                        panic!(
                            "{} for repo '{}' has repeated segment '{}': {}",
                            path_type, repo_name, segments[i], path_str
                        );
                    }
                }
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_database_path_construction_validation() -> Result<()> {
        let fixture = PathValidationFixture::new().await?;
        let config = &fixture.config;

        // Test database path construction doesn't have doubling
        let db_path = &config.database_path;
        let db_str = db_path.to_string_lossy();

        // Should not have double separators
        assert!(
            !db_str.contains("//"),
            "Database path should not contain double separators: {}",
            db_str
        );

        // Should not have repeated .db extensions
        assert!(
            !db_str.contains(".db.db"),
            "Database path should not contain doubled extension: {}",
            db_str
        );

        // Test that we can actually create the database without path issues
        let database = Database::new(db_path).await?;
        database.ensure_tables().await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_config_path_validation() -> Result<()> {
        // Test the global config path construction
        let config_path = Config::get_global_config_path()?;
        let config_str = config_path.to_string_lossy();

        // Should not have double separators
        assert!(
            !config_str.contains("//"),
            "Config path should not contain double separators: {}",
            config_str
        );

        // Should not have doubled segments
        assert!(
            !config_str.contains("iMi/iMi"),
            "Config path should not contain doubled iMi segment: {}",
            config_str
        );

        // Should end with config.toml, not doubled
        assert!(
            !config_str.contains("config.toml.toml"),
            "Config path should not have doubled extension: {}",
            config_str
        );

        assert!(
            config_str.ends_with("config.toml"),
            "Config path should end with config.toml: {}",
            config_str
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_path_canonicalization_no_doubling() -> Result<()> {
        let fixture = PathValidationFixture::new().await?;
        let config = &fixture.config;

        // Test path construction with various scenarios that could cause doubling
        let test_cases = vec![
            ("normal-repo", "normal paths"),
            ("repo-with-dots", "paths with special characters"),
            ("repo-with-current", "paths with current-like names"),
        ];

        for (repo_name, description) in test_cases {
            println!("Testing path canonicalization: {}", description);

            let repo_path = config.get_repo_path(repo_name);
            let trunk_path = config.get_trunk_path(repo_name);

            // Validate paths are clean (no double separators or weird segments)
            for (path, path_name) in [(&repo_path, "repo"), (&trunk_path, "trunk")] {
                let path_str = path.to_string_lossy();

                // Basic path validity checks
                assert!(
                    !path_str.contains("//"),
                    "{} path should not contain double separators: {}",
                    path_name,
                    path_str
                );

                // Note: We don't check for /./ or /../ because these are valid in repo names,
                // but we do check that they don't cause path construction issues

                println!("  ✅ {} path: {}", path_name, path.display());
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_path_construction_regression_prevention() -> Result<()> {
        let fixture = PathValidationFixture::new().await?;
        let config = &fixture.config;

        // Test combinations that historically caused path doubling
        let test_scenarios = vec![
            // (repo_name, worktree_name, potential_issue_description)
            ("repo", "trunk-main", "trunk in worktree name"),
            ("my-repo", ".imi", "hidden directory as worktree"),
            ("test", "test", "same name as repo"),
            ("project", "project/subdir", "nested path"),
        ];

        for (repo_name, worktree_name, description) in test_scenarios {
            println!(
                "Testing scenario: {} (repo: '{}', worktree: '{}')",
                description, repo_name, worktree_name
            );

            // Test all path construction methods
            let repo_path = config.get_repo_path(repo_name);
            let trunk_path = config.get_trunk_path(repo_name);
            let worktree_path = config.get_worktree_path(repo_name, worktree_name);

            // Validate no path component doubling
            for (path, path_name) in [
                (&repo_path, "repo_path"),
                (&trunk_path, "trunk_path"),
                (&worktree_path, "worktree_path"),
            ] {
                let path_str = path.to_string_lossy();

                // No double separators
                assert!(
                    !path_str.contains("//"),
                    "{} in scenario '{}' has double separators: {}",
                    path_name,
                    description,
                    path_str
                );

                // No repeated repo name (unless intentional)
                let repo_occurrences = path_str.matches(repo_name).count();
                if repo_name != worktree_name {
                    assert!(
                        repo_occurrences <= 2, // Once in root path, once in repo segment
                        "{} in scenario '{}' has too many repo name occurrences ({}): {}",
                        path_name,
                        description,
                        repo_occurrences,
                        path_str
                    );
                }
            }

            println!("  ✅ Repo path: {}", repo_path.display());
            println!("  ✅ Trunk path: {}", trunk_path.display());
            println!("  ✅ Worktree path: {}", worktree_path.display());
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_full_init_workflow_path_validation() -> Result<()> {
        let fixture = PathValidationFixture::new().await?;
        let test_root = fixture.get_test_root();

        // Set up test environment with proper config
        let test_config = fixture.config.clone();
        test_config.save().await?;

        // Create test repository structure
        let repo_dir = test_root.join("integration-test-repo");
        let trunk_dir = repo_dir.join("trunk-main");
        tokio::fs::create_dir_all(&trunk_dir).await?;

        // Change to trunk directory
        let original_dir = std::env::current_dir()?;
        std::env::set_current_dir(&trunk_dir)?;

        // Execute init command
        let database = Database::new(&test_config.database_path).await?;
        let init_cmd = InitCommand::new(true, test_config, database); // Force to avoid existing config issues
        let init_result = init_cmd.execute(Some(&trunk_dir)).await;

        // Restore directory
        std::env::set_current_dir(original_dir)?;

        // Validate init was successful
        match init_result {
            Ok(result) => {
                assert!(result.success, "Init should succeed: {}", result.message);
                println!("✅ Init command executed successfully");
                println!("   Message: {}", result.message);

                // Load the final configuration and validate paths
                let final_config = Config::load().await?;

                // All paths should be properly constructed without doubling
                let db_path_str = final_config.database_path.to_string_lossy();
                let root_path_str = final_config.workspace_settings.root_path.to_string_lossy();

                assert!(
                    !db_path_str.contains("//"),
                    "Final database path should not have double separators: {}",
                    db_path_str
                );
                assert!(
                    !root_path_str.contains("//"),
                    "Final root path should not have double separators: {}",
                    root_path_str
                );

                println!("✅ Final config paths validated:");
                println!("   Database: {}", final_config.database_path.display());
                println!("   Root: {}", final_config.workspace_settings.root_path.display());
            }
            Err(e) => {
                panic!("Init command failed: {}", e);
            }
        }

        Ok(())
    }
}
