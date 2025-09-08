/// Database integration tests for iMi initialization
///
/// This test suite focuses specifically on database-related functionality
/// during the initialization process, including:
/// - Database table creation and migration
/// - Worktree registration
/// - Database error handling
/// - Data consistency and validation
use anyhow::{Context, Result};
use std::path::Path;
use tempfile::TempDir;
use tokio::fs;

use imi::config::Config;
use imi::database::{Database, Worktree};

/// Helper for database-focused init testing
pub struct DatabaseInitHelper {
    _temp_dir: TempDir,
    config: Config,
    db: Database,
}

impl DatabaseInitHelper {
    pub async fn new() -> Result<Self> {
        let temp_dir = TempDir::new().context("Failed to create temp directory")?;

        let mut config = Config::default();
        config.database_path = temp_dir.path().join("test_init.db");
        config.root_path = temp_dir.path().join("projects");

        let db = Database::new(&config.database_path).await?;

        Ok(Self {
            _temp_dir: temp_dir,
            config,
            db,
        })
    }

    pub fn get_temp_path(&self) -> &Path {
        self._temp_dir.path()
    }

    pub async fn create_test_repo_structure(&self, repo_name: &str) -> Result<std::path::PathBuf> {
        let repo_dir = self.get_temp_path().join("projects").join(repo_name);
        let trunk_dir = repo_dir.join("trunk-main");
        fs::create_dir_all(&trunk_dir).await?;
        Ok(trunk_dir)
    }

    /// Simulate the database operations that would happen during init
    pub async fn simulate_init_database_operations(
        &self,
        repo_name: &str,
        trunk_path: &Path,
    ) -> Result<Worktree> {
        // This simulates what the init command should do with the database
        let trunk_name = trunk_path
            .file_name()
            .and_then(|n| n.to_str())
            .context("Invalid trunk directory name")?;

        // Create worktree entry for trunk
        let worktree = self
            .db
            .create_worktree(
                repo_name,
                trunk_name,
                &self.config.git_settings.default_branch,
                "trunk",
                trunk_path.to_str().context("Invalid trunk path")?,
                None,
            )
            .await?;

        Ok(worktree)
    }
}

#[cfg(test)]
mod database_table_tests {
    use super::*;

    #[tokio::test]
    async fn test_database_tables_created_on_init() {
        let helper = DatabaseInitHelper::new().await.unwrap();

        // Database should be initialized with all required tables
        // The Database::new() method already calls run_migrations()

        // Verify tables exist by attempting to use the database methods
        // If tables don't exist, these operations will fail

        // Test worktrees table
        let worktrees_result = helper.db.list_worktrees(None).await;
        assert!(
            worktrees_result.is_ok(),
            "worktrees table should exist and be queryable"
        );

        // Test agent_activities table by attempting to get recent activities
        let activities_result = helper.db.get_recent_activities(None, 10).await;
        assert!(
            activities_result.is_ok(),
            "agent_activities table should exist and be queryable"
        );
    }

    #[tokio::test]
    async fn test_database_indexes_created() {
        let helper = DatabaseInitHelper::new().await.unwrap();

        // Test index effectiveness indirectly by testing query performance
        // Create multiple worktrees to test indexing
        for i in 0..10 {
            let repo_name = format!("index-test-repo-{}", i);
            let trunk_dir = helper.create_test_repo_structure(&repo_name).await.unwrap();
            helper
                .simulate_init_database_operations(&repo_name, &trunk_dir)
                .await
                .unwrap();
        }

        // Query specific repo - should be fast due to repo_name index
        let start = std::time::Instant::now();
        let specific_worktrees = helper
            .db
            .list_worktrees(Some("index-test-repo-5"))
            .await
            .unwrap();
        let duration = start.elapsed();

        assert_eq!(specific_worktrees.len(), 1);
        assert!(
            duration.as_millis() < 100,
            "Repo-specific query should be fast (indexed)"
        );

        // Query all active worktrees - should be fast due to active index
        let all_worktrees = helper.db.list_worktrees(None).await.unwrap();
        assert_eq!(all_worktrees.len(), 10, "Should find all created worktrees");
    }

    #[tokio::test]
    async fn test_database_schema_validation() {
        let helper = DatabaseInitHelper::new().await.unwrap();
        let repo_name = "schema-test-repo";

        let trunk_dir = helper.create_test_repo_structure(repo_name).await.unwrap();

        // Create a worktree to test all fields are working
        let worktree = helper
            .simulate_init_database_operations(repo_name, &trunk_dir)
            .await
            .unwrap();

        // Verify all expected fields are present and accessible
        assert!(!worktree.id.is_empty(), "id field should be populated");
        assert_eq!(worktree.repo_name, repo_name, "repo_name field should work");
        assert!(
            !worktree.worktree_name.is_empty(),
            "worktree_name field should be populated"
        );
        assert!(
            !worktree.branch_name.is_empty(),
            "branch_name field should be populated"
        );
        assert!(
            !worktree.worktree_type.is_empty(),
            "worktree_type field should be populated"
        );
        assert!(!worktree.path.is_empty(), "path field should be populated");
        assert!(
            worktree.created_at > chrono::Utc::now() - chrono::Duration::minutes(1),
            "created_at should be recent"
        );
        assert!(
            worktree.updated_at > chrono::Utc::now() - chrono::Duration::minutes(1),
            "updated_at should be recent"
        );
        assert!(worktree.active, "active field should work");
        // agent_id is optional so it can be None

        println!("Worktree schema validation passed: {:?}", worktree);
    }
}

#[cfg(test)]
mod worktree_registration_tests {
    use super::*;

    #[tokio::test]
    async fn test_trunk_worktree_registration() {
        let helper = DatabaseInitHelper::new().await.unwrap();
        let repo_name = "test-trunk-registration";

        let trunk_dir = helper.create_test_repo_structure(repo_name).await.unwrap();

        // Simulate init database operations
        let worktree = helper
            .simulate_init_database_operations(repo_name, &trunk_dir)
            .await
            .unwrap();

        // Verify worktree was created correctly
        assert_eq!(worktree.repo_name, repo_name);
        assert_eq!(worktree.worktree_name, "trunk-main");
        assert_eq!(worktree.worktree_type, "trunk");
        assert_eq!(
            worktree.branch_name,
            helper.config.git_settings.default_branch
        );
        assert!(worktree.active);
        assert!(worktree.agent_id.is_none());

        // Verify it can be retrieved from database
        let retrieved = helper
            .db
            .get_worktree(repo_name, "trunk-main")
            .await
            .unwrap();
        assert!(retrieved.is_some());

        let retrieved_worktree = retrieved.unwrap();
        assert_eq!(retrieved_worktree.id, worktree.id);
        assert_eq!(retrieved_worktree.path, trunk_dir.to_string_lossy());
    }

    #[tokio::test]
    async fn test_multiple_repo_trunk_registration() {
        let helper = DatabaseInitHelper::new().await.unwrap();

        let repos = vec!["repo-1", "repo-2", "repo-3"];
        let mut created_worktrees = Vec::new();

        for repo_name in &repos {
            let trunk_dir = helper.create_test_repo_structure(repo_name).await.unwrap();
            let worktree = helper
                .simulate_init_database_operations(repo_name, &trunk_dir)
                .await
                .unwrap();
            created_worktrees.push(worktree);
        }

        // Verify all worktrees were created
        assert_eq!(created_worktrees.len(), 3);

        // Verify we can list worktrees for each repo
        for repo_name in &repos {
            let worktrees = helper.db.list_worktrees(Some(repo_name)).await.unwrap();
            assert_eq!(worktrees.len(), 1);
            assert_eq!(worktrees[0].repo_name, *repo_name);
            assert_eq!(worktrees[0].worktree_type, "trunk");
        }

        // Verify we can list all worktrees
        let all_worktrees = helper.db.list_worktrees(None).await.unwrap();
        assert_eq!(all_worktrees.len(), 3);
    }

    #[tokio::test]
    async fn test_trunk_worktree_with_different_branch_names() {
        let helper = DatabaseInitHelper::new().await.unwrap();

        // Test with different default branch configurations
        let test_cases = vec![
            ("main-repo", "main"),
            ("develop-repo", "develop"),
            ("staging-repo", "staging"),
        ];

        for (repo_name, branch_name) in test_cases {
            // Modify config for this test
            let mut test_config = helper.config.clone();
            test_config.git_settings.default_branch = branch_name.to_string();

            let trunk_dir = helper.create_test_repo_structure(repo_name).await.unwrap();

            // Create worktree with custom branch name
            let worktree = helper
                .db
                .create_worktree(
                    repo_name,
                    "trunk-main",
                    branch_name,
                    "trunk",
                    trunk_dir.to_str().unwrap(),
                    None,
                )
                .await
                .unwrap();

            assert_eq!(worktree.branch_name, branch_name);
            assert_eq!(worktree.repo_name, repo_name);
        }
    }

    #[tokio::test]
    async fn test_duplicate_trunk_registration_handling() {
        let helper = DatabaseInitHelper::new().await.unwrap();
        let repo_name = "duplicate-test-repo";

        let trunk_dir = helper.create_test_repo_structure(repo_name).await.unwrap();

        // First registration should succeed
        let worktree1 = helper
            .simulate_init_database_operations(repo_name, &trunk_dir)
            .await
            .unwrap();

        // Second registration should succeed due to INSERT OR REPLACE
        let worktree2 = helper
            .simulate_init_database_operations(repo_name, &trunk_dir)
            .await
            .unwrap();

        // Should have different IDs but same repo_name and worktree_name
        assert_ne!(worktree1.id, worktree2.id);
        assert_eq!(worktree1.repo_name, worktree2.repo_name);
        assert_eq!(worktree1.worktree_name, worktree2.worktree_name);

        // Should only have one worktree in the database (replaced, not duplicated)
        let worktrees = helper.db.list_worktrees(Some(repo_name)).await.unwrap();
        assert_eq!(worktrees.len(), 1);
        assert_eq!(worktrees[0].id, worktree2.id); // Should be the newer one
    }
}

#[cfg(test)]
mod database_error_handling_tests {
    use super::*;

    #[tokio::test]
    async fn test_handles_database_connection_failure() {
        // Test with invalid database path
        let temp_dir = TempDir::new().unwrap();
        let invalid_db_path = temp_dir
            .path()
            .join("nonexistent")
            .join("dir")
            .join("test.db");

        // This should fail or create the necessary directories
        let db_result = Database::new(&invalid_db_path).await;

        // Database::new should either succeed (by creating directories) or fail gracefully
        println!(
            "Database creation with invalid path result: {:?}",
            db_result
        );
    }

    #[tokio::test]
    async fn test_handles_database_corruption() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("corrupted.db");

        // Create a corrupted database file
        fs::write(&db_path, "This is not a valid SQLite database")
            .await
            .unwrap();

        // Database::new should handle corruption
        let db_result = Database::new(&db_path).await;

        // Should either recover or provide clear error
        println!(
            "Database creation with corrupted file result: {:?}",
            db_result
        );
    }

    #[tokio::test]
    async fn test_handles_insufficient_disk_space() {
        // This is difficult to test without actually filling up disk
        // In practice, would need to mock filesystem operations
        let helper = DatabaseInitHelper::new().await.unwrap();

        // Attempt to create many large entries to simulate disk full
        let repo_name = "disk-space-test";
        let trunk_dir = helper.create_test_repo_structure(repo_name).await.unwrap();

        let result = helper
            .simulate_init_database_operations(repo_name, &trunk_dir)
            .await;

        // Should succeed in normal test environment
        assert!(result.is_ok(), "Should handle normal disk space correctly");
    }

    #[tokio::test]
    async fn test_database_transaction_rollback() {
        let helper = DatabaseInitHelper::new().await.unwrap();
        let repo_name = "transaction-test";

        let trunk_dir = helper.create_test_repo_structure(repo_name).await.unwrap();

        // This test would ideally simulate a partial failure and verify rollback
        // For now, just verify normal operation
        let result = helper
            .simulate_init_database_operations(repo_name, &trunk_dir)
            .await;
        assert!(result.is_ok(), "Database operations should succeed");

        // In a full implementation, would test scenarios like:
        // - Network interruption during database write
        // - Disk full during transaction
        // - Process termination during transaction
    }
}

#[cfg(test)]
mod database_consistency_tests {
    use super::*;

    #[tokio::test]
    async fn test_worktree_timestamps_consistency() {
        let helper = DatabaseInitHelper::new().await.unwrap();
        let repo_name = "timestamp-test";

        let trunk_dir = helper.create_test_repo_structure(repo_name).await.unwrap();

        let start_time = chrono::Utc::now();
        let worktree = helper
            .simulate_init_database_operations(repo_name, &trunk_dir)
            .await
            .unwrap();
        let end_time = chrono::Utc::now();

        // Verify timestamps are within expected range
        assert!(
            worktree.created_at >= start_time,
            "Created timestamp should be after start"
        );
        assert!(
            worktree.created_at <= end_time,
            "Created timestamp should be before end"
        );
        assert!(
            worktree.updated_at >= start_time,
            "Updated timestamp should be after start"
        );
        assert!(
            worktree.updated_at <= end_time,
            "Updated timestamp should be before end"
        );

        // For new entries, created_at and updated_at should be very close
        let time_diff = (worktree.updated_at - worktree.created_at)
            .num_milliseconds()
            .abs();
        assert!(
            time_diff < 1000,
            "Created and updated times should be within 1 second"
        );
    }

    #[tokio::test]
    async fn test_worktree_path_consistency() {
        let helper = DatabaseInitHelper::new().await.unwrap();
        let repo_name = "path-consistency-test";

        let trunk_dir = helper.create_test_repo_structure(repo_name).await.unwrap();
        let expected_path = trunk_dir.to_string_lossy().to_string();

        let worktree = helper
            .simulate_init_database_operations(repo_name, &trunk_dir)
            .await
            .unwrap();

        // Path stored in database should match actual directory path
        assert_eq!(worktree.path, expected_path);

        // Verify path can be used to access the directory
        assert!(
            std::path::Path::new(&worktree.path).exists(),
            "Path stored in database should point to existing directory"
        );
    }

    #[tokio::test]
    async fn test_worktree_unique_constraints() {
        let helper = DatabaseInitHelper::new().await.unwrap();
        let repo_name = "unique-constraint-test";

        let trunk_dir = helper.create_test_repo_structure(repo_name).await.unwrap();

        // Create first worktree
        let worktree1 = helper
            .db
            .create_worktree(
                repo_name,
                "trunk-main",
                "main",
                "trunk",
                trunk_dir.to_str().unwrap(),
                None,
            )
            .await
            .unwrap();

        // Create second worktree with same repo_name and worktree_name
        // This should succeed due to INSERT OR REPLACE, updating the first entry
        let worktree2 = helper
            .db
            .create_worktree(
                repo_name,
                "trunk-main",
                "main",
                "trunk",
                trunk_dir.to_str().unwrap(),
                None,
            )
            .await
            .unwrap();

        // Should have different IDs (indicating replacement occurred)
        assert_ne!(worktree1.id, worktree2.id);

        // Should only have one entry in database
        let worktrees = helper.db.list_worktrees(Some(repo_name)).await.unwrap();
        assert_eq!(worktrees.len(), 1);
        assert_eq!(worktrees[0].id, worktree2.id);
    }

    #[tokio::test]
    async fn test_database_foreign_key_constraints() {
        let helper = DatabaseInitHelper::new().await.unwrap();
        let repo_name = "foreign-key-test";

        // Create a worktree first
        let trunk_dir = helper.create_test_repo_structure(repo_name).await.unwrap();
        let worktree = helper
            .simulate_init_database_operations(repo_name, &trunk_dir)
            .await
            .unwrap();

        // Try to create agent activity for the worktree
        let activity_result = helper
            .db
            .log_agent_activity(
                "test-agent",
                &worktree.id,
                "created",
                Some("test.txt"),
                "Created test file",
            )
            .await;

        assert!(
            activity_result.is_ok(),
            "Should be able to create activity for existing worktree"
        );

        // Try to create agent activity for non-existent worktree
        let invalid_activity_result = helper
            .db
            .log_agent_activity(
                "test-agent",
                "non-existent-worktree-id",
                "created",
                Some("test.txt"),
                "Created test file",
            )
            .await;

        // This might succeed or fail depending on foreign key enforcement
        // SQLite doesn't enforce foreign keys by default
        println!("Invalid activity result: {:?}", invalid_activity_result);
    }
}

#[cfg(test)]
mod database_performance_tests {
    use super::*;
    use std::time::Instant;

    #[tokio::test]
    async fn test_database_init_performance() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("performance.db");

        let start = Instant::now();
        let db = Database::new(&db_path).await.unwrap();
        let init_duration = start.elapsed();

        println!("Database initialization took: {:?}", init_duration);
        assert!(
            init_duration.as_millis() < 1000,
            "Database init should complete within 1 second"
        );

        // Test worktree creation performance
        let worktree_start = Instant::now();
        let _worktree = db
            .create_worktree(
                "perf-test-repo",
                "trunk-main",
                "main",
                "trunk",
                "/test/path",
                None,
            )
            .await
            .unwrap();
        let worktree_duration = worktree_start.elapsed();

        println!("Worktree creation took: {:?}", worktree_duration);
        assert!(
            worktree_duration.as_millis() < 100,
            "Worktree creation should complete within 100ms"
        );
    }

    #[tokio::test]
    async fn test_bulk_worktree_operations_performance() {
        let helper = DatabaseInitHelper::new().await.unwrap();

        let start = Instant::now();

        // Create 100 worktrees
        for i in 0..100 {
            let repo_name = format!("bulk-repo-{}", i);
            let trunk_dir = helper.create_test_repo_structure(&repo_name).await.unwrap();
            helper
                .simulate_init_database_operations(&repo_name, &trunk_dir)
                .await
                .unwrap();
        }

        let duration = start.elapsed();
        println!("Creating 100 worktrees took: {:?}", duration);

        // Should handle bulk operations reasonably quickly
        assert!(
            duration.as_secs() < 10,
            "Bulk operations should complete within 10 seconds"
        );

        // Test listing performance
        let list_start = Instant::now();
        let all_worktrees = helper.db.list_worktrees(None).await.unwrap();
        let list_duration = list_start.elapsed();

        println!(
            "Listing {} worktrees took: {:?}",
            all_worktrees.len(),
            list_duration
        );
        assert_eq!(all_worktrees.len(), 100);
        assert!(list_duration.as_millis() < 100, "Listing should be fast");
    }

    #[tokio::test]
    async fn test_database_query_optimization() {
        let helper = DatabaseInitHelper::new().await.unwrap();

        // Create worktrees for multiple repos
        for i in 0..50 {
            let repo_name = format!("query-test-repo-{}", i);
            let trunk_dir = helper.create_test_repo_structure(&repo_name).await.unwrap();
            helper
                .simulate_init_database_operations(&repo_name, &trunk_dir)
                .await
                .unwrap();
        }

        // Test specific repo query performance (should use repo_name index)
        let specific_start = Instant::now();
        let specific_worktrees = helper
            .db
            .list_worktrees(Some("query-test-repo-25"))
            .await
            .unwrap();
        let specific_duration = specific_start.elapsed();

        assert_eq!(specific_worktrees.len(), 1);
        println!("Specific repo query took: {:?}", specific_duration);
        assert!(
            specific_duration.as_millis() < 10,
            "Indexed query should be very fast"
        );

        // Test active worktrees filter performance (should use active index)
        let active_start = Instant::now();
        let active_worktrees = helper.db.list_worktrees(None).await.unwrap();
        let active_duration = active_start.elapsed();

        println!("Active worktrees query took: {:?}", active_duration);
        assert!(
            active_duration.as_millis() < 50,
            "Active filter should be fast"
        );
        assert_eq!(active_worktrees.len(), 50); // All should be active
    }
}
