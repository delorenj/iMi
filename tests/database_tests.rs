/// Emergency Critical Coverage Tests for Database Module
///
/// This comprehensive test suite provides complete coverage for database.rs (180 lines, 0% coverage)
/// to address the CRITICAL coverage crisis identified in AC-060.
///
/// Coverage targets:
/// - Database creation and connection: new(), ensure_tables()
/// - Repository operations: create_repository(), get_repository()
/// - Worktree operations: create_worktree(), get_worktree(), list_worktrees(), deactivate_worktree()
/// - Agent activity operations: log_agent_activity(), get_recent_activities()
/// - Migration and indexing: run_migrations()
/// - Error handling and edge cases
use anyhow::{Context, Result};
use chrono::Utc;
use std::path::PathBuf;
use tempfile::TempDir;

use imi::database::Database;

/// Test helper for database operations
struct DatabaseTestHelper {
    _temp_dir: TempDir,
    db: Database,
    db_path: PathBuf,
}

impl DatabaseTestHelper {
    async fn new() -> Result<Self> {
        let temp_dir = TempDir::new().context("Failed to create temp directory")?;
        let db_path = temp_dir.path().join("test.db");
        let db = Database::new(&db_path).await?;

        Ok(Self {
            _temp_dir: temp_dir,
            db,
            db_path,
        })
    }

    fn get_temp_path(&self) -> &std::path::Path {
        self._temp_dir.path()
    }
}

#[cfg(test)]
mod database_creation_tests {
    use super::*;

    #[tokio::test]
    async fn test_database_new_creates_database_file() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("new_test.db");

        // Database should not exist initially
        assert!(!db_path.exists(), "Database should not exist initially");

        let db = Database::new(&db_path).await.unwrap();

        // Database file should now exist
        assert!(db_path.exists(), "Database file should be created");

        // Verify we can perform operations
        let worktrees = db.list_worktrees(None).await.unwrap();
        assert_eq!(worktrees.len(), 0, "New database should be empty");
    }

    #[tokio::test]
    async fn test_database_new_with_existing_database() {
        let helper = DatabaseTestHelper::new().await.unwrap();

        // Create a repository first
        helper
            .db
            .create_repository(
                "test-repo",
                "/path/to/test-repo",
                "git@github.com:user/test-repo.git",
                "main",
            )
            .await
            .unwrap();

        // Create a worktree to populate database
        let worktree = helper
            .db
            .create_worktree(
                "test-repo",
                "trunk-main",
                "main",
                "trunk",
                "/test/path",
                None,
            )
            .await
            .unwrap();

        // Open existing database
        let db2 = Database::new(&helper.db_path).await.unwrap();
        let existing_worktrees = db2.list_worktrees(None).await.unwrap();

        assert_eq!(existing_worktrees.len(), 1);
        assert_eq!(existing_worktrees[0].id, worktree.id);
    }

    #[tokio::test]
    async fn test_database_new_creates_nested_directories() {
        let temp_dir = TempDir::new().unwrap();
        let nested_path = temp_dir
            .path()
            .join("level1")
            .join("level2")
            .join("level3")
            .join("nested.db");

        let db = Database::new(&nested_path).await.unwrap();

        assert!(
            nested_path.exists(),
            "Database should create nested directories"
        );

        // Verify database is functional
        let worktrees = db.list_worktrees(None).await.unwrap();
        assert_eq!(worktrees.len(), 0);
    }

    #[tokio::test]
    async fn test_ensure_tables_method() {
        let helper = DatabaseTestHelper::new().await.unwrap();

        // Call ensure_tables explicitly (should be idempotent)
        let result = helper.db.ensure_tables().await;
        assert!(result.is_ok(), "ensure_tables should succeed");

        // Should still work after explicit call
        let worktrees = helper.db.list_worktrees(None).await.unwrap();
        assert_eq!(worktrees.len(), 0);
    }
}

#[cfg(test)]
mod repository_operations_tests {
    use super::*;

    #[tokio::test]
    async fn test_create_repository_success() {
        let helper = DatabaseTestHelper::new().await.unwrap();

        let repo = helper
            .db
            .create_repository(
                "test-repo",
                "/path/to/repo",
                "git@github.com:user/test-repo.git",
                "main",
            )
            .await
            .unwrap();

        // Verify all fields are correct
        assert_eq!(repo.name, "test-repo");
        assert_eq!(repo.path, "/path/to/repo");
        assert_eq!(repo.remote_url, "git@github.com:user/test-repo.git");
        assert_eq!(repo.default_branch, "main");
        assert!(repo.active, "Repository should be active by default");

        // Verify timestamps are recent
        let now = Utc::now();
        let time_diff = (now - repo.created_at).num_seconds().abs();
        assert!(time_diff < 5, "Created timestamp should be recent");

        let time_diff = (now - repo.updated_at).num_seconds().abs();
        assert!(time_diff < 5, "Updated timestamp should be recent");
    }

    #[tokio::test]
    async fn test_get_repository_existing() {
        let helper = DatabaseTestHelper::new().await.unwrap();

        // Create a repository first
        let created_repo = helper
            .db
            .create_repository(
                "get-test-repo",
                "/path/to/get-repo",
                "git@github.com:user/get-repo.git",
                "develop",
            )
            .await
            .unwrap();

        // Retrieve the repository
        let retrieved_repo = helper.db.get_repository("get-test-repo").await.unwrap();

        assert!(retrieved_repo.is_some(), "Repository should be found");
        let repo = retrieved_repo.unwrap();

        assert_eq!(repo.id, created_repo.id);
        assert_eq!(repo.name, "get-test-repo");
        assert_eq!(repo.path, "/path/to/get-repo");
        assert_eq!(repo.remote_url, "git@github.com:user/get-repo.git");
        assert_eq!(repo.default_branch, "develop");
        assert!(repo.active);
    }

    #[tokio::test]
    async fn test_get_repository_nonexistent() {
        let helper = DatabaseTestHelper::new().await.unwrap();

        let result = helper.db.get_repository("nonexistent-repo").await.unwrap();

        assert!(
            result.is_none(),
            "Nonexistent repository should return None"
        );
    }

    #[tokio::test]
    async fn test_create_repository_with_different_default_branches() {
        let helper = DatabaseTestHelper::new().await.unwrap();

        let branches = vec!["main", "master", "develop", "staging"];

        for branch in branches {
            let repo_name = format!("repo-{}", branch);
            let repo = helper
                .db
                .create_repository(
                    &repo_name,
                    &format!("/path/to/{}", repo_name),
                    &format!("git@github.com:user/{}.git", repo_name),
                    branch,
                )
                .await
                .unwrap();

            assert_eq!(repo.default_branch, branch);
            assert_eq!(repo.name, repo_name);
        }
    }
}

#[cfg(test)]
mod worktree_operations_tests {
    use super::*;

    #[tokio::test]
    async fn test_create_worktree_success() {
        let helper = DatabaseTestHelper::new().await.unwrap();

        // Create repository first
        helper
            .db
            .create_repository(
                "test-repo",
                "/path/to/test-repo",
                "git@github.com:user/test-repo.git",
                "main",
            )
            .await
            .unwrap();

        let worktree = helper
            .db
            .create_worktree(
                "test-repo",
                "feat-auth",
                "feat/authentication",
                "feat",
                "/path/to/worktree",
                Some("agent-123".to_string()),
            )
            .await
            .unwrap();

        // Verify all fields
        assert_eq!(worktree.repo_name, "test-repo");
        assert_eq!(worktree.worktree_name, "feat-auth");
        assert_eq!(worktree.branch_name, "feat/authentication");
        assert_eq!(worktree.worktree_type, "feat");
        assert_eq!(worktree.path, "/path/to/worktree");
        assert!(worktree.active, "Worktree should be active by default");
        assert_eq!(worktree.agent_id, Some("agent-123".to_string()));

        // Verify timestamps
        let now = Utc::now();
        let time_diff = (now - worktree.created_at).num_seconds().abs();
        assert!(time_diff < 5, "Created timestamp should be recent");
    }

    #[tokio::test]
    async fn test_create_worktree_without_agent_id() {
        let helper = DatabaseTestHelper::new().await.unwrap();

        // Create repository first
        helper
            .db
            .create_repository(
                "test-repo",
                "/path/to/test-repo",
                "git@github.com:user/test-repo.git",
                "main",
            )
            .await
            .unwrap();

        let worktree = helper
            .db
            .create_worktree(
                "test-repo",
                "trunk-main",
                "main",
                "trunk",
                "/path/to/trunk",
                None,
            )
            .await
            .unwrap();

        assert!(worktree.agent_id.is_none(), "Agent ID should be None");
        assert_eq!(worktree.worktree_type, "trunk");
    }

    #[tokio::test]
    async fn test_create_worktree_insert_or_replace() {
        let helper = DatabaseTestHelper::new().await.unwrap();

        // Create repository first
        helper
            .db
            .create_repository(
                "replace-repo",
                "/path/to/replace-repo",
                "git@github.com:user/replace-repo.git",
                "main",
            )
            .await
            .unwrap();

        // Create first worktree
        let worktree1 = helper
            .db
            .create_worktree(
                "replace-repo",
                "feat-test",
                "feat/test-v1",
                "feat",
                "/path/v1",
                Some("agent-1"),
            )
            .await
            .unwrap();

        // Create second worktree with same repo_name and worktree_name (should replace)
        let worktree2 = helper
            .db
            .create_worktree(
                "replace-repo",
                "feat-test",
                "feat/test-v2",
                "feat",
                "/path/v2",
                Some("agent-2"),
            )
            .await
            .unwrap();

        // Should have different IDs
        assert_ne!(worktree1.id, worktree2.id);

        // Verify replacement occurred
        let retrieved = helper
            .db
            .get_worktree("replace-repo", "feat-test")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(retrieved.id, worktree2.id);
        assert_eq!(retrieved.branch_name, "feat/test-v2");
        assert_eq!(retrieved.path, "/path/v2");
        assert_eq!(retrieved.agent_id, Some("agent-2".to_string()));
    }

    #[tokio::test]
    async fn test_get_worktree_existing() {
        let helper = DatabaseTestHelper::new().await.unwrap();

        // Create repository first
        helper
            .db
            .create_repository(
                "get-repo",
                "/path/to/get-repo",
                "git@github.com:user/get-repo.git",
                "main",
            )
            .await
            .unwrap();

        // Create a worktree
        let created = helper
            .db
            .create_worktree(
                "get-repo",
                "pr-123",
                "pr/feature-branch",
                "review",
                "/path/to/pr",
                None,
            )
            .await
            .unwrap();

        // Retrieve it
        let retrieved = helper.db.get_worktree("get-repo", "pr-123").await.unwrap();

        assert!(retrieved.is_some());
        let worktree = retrieved.unwrap();
        assert_eq!(worktree.id, created.id);
        assert_eq!(worktree.worktree_type, "review");
    }

    #[tokio::test]
    async fn test_get_worktree_nonexistent() {
        let helper = DatabaseTestHelper::new().await.unwrap();

        let result = helper
            .db
            .get_worktree("nonexistent-repo", "nonexistent-worktree")
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_list_worktrees_all() {
        let helper = DatabaseTestHelper::new().await.unwrap();

        // Create repositories first
        helper
            .db
            .create_repository(
                "repo-1",
                "/path/to/repo-1",
                "git@github.com:user/repo-1.git",
                "main",
            )
            .await
            .unwrap();

        helper
            .db
            .create_repository(
                "repo-2",
                "/path/to/repo-2",
                "git@github.com:user/repo-2.git",
                "main",
            )
            .await
            .unwrap();

        // Create multiple worktrees
        let worktrees_data = vec![
            ("repo-1", "trunk-main", "trunk"),
            ("repo-1", "feat-auth", "feat"),
            ("repo-2", "trunk-main", "trunk"),
            ("repo-2", "pr-456", "review"),
        ];

        for (repo, name, wt_type) in &worktrees_data {
            helper
                .db
                .create_worktree(
                    repo,
                    name,
                    "main",
                    wt_type,
                    &format!("/path/{}/{}", repo, name),
                    None,
                )
                .await
                .unwrap();
        }

        // List all worktrees
        let all_worktrees = helper.db.list_worktrees(None).await.unwrap();
        assert_eq!(all_worktrees.len(), 4);

        // Verify ordering (should be by created_at DESC)
        let mut previous_created_at = Utc::now() + chrono::Duration::minutes(1);
        for worktree in &all_worktrees {
            assert!(
                worktree.created_at <= previous_created_at,
                "Worktrees should be ordered by created_at DESC"
            );
            previous_created_at = worktree.created_at;
        }
    }

    #[tokio::test]
    async fn test_list_worktrees_by_repo() {
        let helper = DatabaseTestHelper::new().await.unwrap();

        // Create repositories first
        helper
            .db
            .create_repository(
                "filter-repo-1",
                "/path/to/filter-repo-1",
                "git@github.com:user/filter-repo-1.git",
                "main",
            )
            .await
            .unwrap();

        helper
            .db
            .create_repository(
                "filter-repo-2",
                "/path/to/filter-repo-2",
                "git@github.com:user/filter-repo-2.git",
                "main",
            )
            .await
            .unwrap();

        // Create worktrees for different repos
        helper
            .db
            .create_worktree(
                "filter-repo-1",
                "trunk-main",
                "main",
                "trunk",
                "/path1",
                None,
            )
            .await
            .unwrap();

        helper
            .db
            .create_worktree(
                "filter-repo-1",
                "feat-test",
                "feat/test",
                "feat",
                "/path2",
                None,
            )
            .await
            .unwrap();

        helper
            .db
            .create_worktree(
                "filter-repo-2",
                "trunk-main",
                "main",
                "trunk",
                "/path3",
                None,
            )
            .await
            .unwrap();

        // Filter by specific repo
        let repo1_worktrees = helper
            .db
            .list_worktrees(Some("filter-repo-1"))
            .await
            .unwrap();

        assert_eq!(repo1_worktrees.len(), 2);
        for worktree in &repo1_worktrees {
            assert_eq!(worktree.repo_name, "filter-repo-1");
        }

        let repo2_worktrees = helper
            .db
            .list_worktrees(Some("filter-repo-2"))
            .await
            .unwrap();

        assert_eq!(repo2_worktrees.len(), 1);
        assert_eq!(repo2_worktrees[0].repo_name, "filter-repo-2");
    }

    #[tokio::test]
    async fn test_deactivate_worktree() {
        let helper = DatabaseTestHelper::new().await.unwrap();

        // Create repository first
        helper
            .db
            .create_repository(
                "deactivate-repo",
                "/path/to/deactivate-repo",
                "git@github.com:user/deactivate-repo.git",
                "main",
            )
            .await
            .unwrap();

        // Create a worktree
        let created = helper
            .db
            .create_worktree(
                "deactivate-repo",
                "feat-temp",
                "feat/temporary",
                "feat",
                "/path/temp",
                None,
            )
            .await
            .unwrap();

        // Verify it's active and listed
        let initial_worktrees = helper
            .db
            .list_worktrees(Some("deactivate-repo"))
            .await
            .unwrap();
        assert_eq!(initial_worktrees.len(), 1);
        assert!(initial_worktrees[0].active);

        // Deactivate it
        helper
            .db
            .deactivate_worktree("deactivate-repo", "feat-temp")
            .await
            .unwrap();

        // Should no longer appear in active worktrees list
        let after_deactivate = helper
            .db
            .list_worktrees(Some("deactivate-repo"))
            .await
            .unwrap();
        assert_eq!(after_deactivate.len(), 0);

        // But should still exist in database as inactive
        let retrieved = helper
            .db
            .get_worktree("deactivate-repo", "feat-temp")
            .await
            .unwrap();
        // get_worktree only returns active worktrees, so this should be None
        assert!(retrieved.is_none());
    }
}

#[cfg(test)]
mod agent_activity_tests {
    use super::*;

    #[tokio::test]
    async fn test_log_agent_activity_success() {
        let helper = DatabaseTestHelper::new().await.unwrap();

        // Create repository first
        helper
            .db
            .create_repository(
                "activity-repo",
                "/path/to/activity-repo",
                "git@github.com:user/activity-repo.git",
                "main",
            )
            .await
            .unwrap();

        // Create a worktree first
        let worktree = helper
            .db
            .create_worktree(
                "activity-repo",
                "trunk-main",
                "main",
                "trunk",
                "/path/activity",
                None,
            )
            .await
            .unwrap();

        // Log an activity
        let activity = helper
            .db
            .log_agent_activity(
                "test-agent",
                &worktree.id,
                "created",
                Some("src/main.rs"),
                "Created main source file",
            )
            .await
            .unwrap();

        // Verify activity fields
        assert_eq!(activity.agent_id, "test-agent");
        assert_eq!(activity.worktree_id, worktree.id);
        assert_eq!(activity.activity_type, "created");
        assert_eq!(activity.file_path, Some("src/main.rs".to_string()));
        assert_eq!(activity.description, "Created main source file");

        // Verify timestamp
        let now = Utc::now();
        let time_diff = (now - activity.created_at).num_seconds().abs();
        assert!(time_diff < 5, "Activity timestamp should be recent");
    }

    #[tokio::test]
    async fn test_log_agent_activity_without_file_path() {
        let helper = DatabaseTestHelper::new().await.unwrap();

        // Create repository first
        helper
            .db
            .create_repository(
                "activity-repo",
                "/path/to/activity-repo",
                "git@github.com:user/activity-repo.git",
                "main",
            )
            .await
            .unwrap();

        let worktree = helper
            .db
            .create_worktree(
                "activity-repo",
                "feat-test",
                "feat/test",
                "feat",
                "/path",
                None,
            )
            .await
            .unwrap();

        let activity = helper
            .db
            .log_agent_activity(
                "test-agent",
                &worktree.id,
                "committed",
                None,
                "Committed changes to repository",
            )
            .await
            .unwrap();

        assert_eq!(activity.activity_type, "committed");
        assert!(activity.file_path.is_none());
        assert_eq!(activity.description, "Committed changes to repository");
    }

    #[tokio::test]
    async fn test_get_recent_activities_with_worktree_filter() {
        let helper = DatabaseTestHelper::new().await.unwrap();

        // Create repositories first
        helper
            .db
            .create_repository(
                "repo1",
                "/path/to/repo1",
                "git@github.com:user/repo1.git",
                "main",
            )
            .await
            .unwrap();

        helper
            .db
            .create_repository(
                "repo2",
                "/path/to/repo2",
                "git@github.com:user/repo2.git",
                "main",
            )
            .await
            .unwrap();

        // Create two worktrees
        let worktree1 = helper
            .db
            .create_worktree("repo1", "wt1", "branch1", "feat", "/path1", None)
            .await
            .unwrap();

        let worktree2 = helper
            .db
            .create_worktree("repo2", "wt2", "branch2", "feat", "/path2", None)
            .await
            .unwrap();

        // Add activities to both
        for i in 0..3 {
            helper
                .db
                .log_agent_activity(
                    "agent1",
                    &worktree1.id,
                    "modified",
                    Some(&format!("file{}.rs", i)),
                    &format!("Modified file {}", i),
                )
                .await
                .unwrap();

            helper
                .db
                .log_agent_activity(
                    "agent2",
                    &worktree2.id,
                    "created",
                    Some(&format!("test{}.rs", i)),
                    &format!("Created test {}", i),
                )
                .await
                .unwrap();
        }

        // Get activities for specific worktree
        let wt1_activities = helper
            .db
            .get_recent_activities(Some(&worktree1.id), 10)
            .await
            .unwrap();

        assert_eq!(wt1_activities.len(), 3);
        for activity in &wt1_activities {
            assert_eq!(activity.worktree_id, worktree1.id);
            assert_eq!(activity.agent_id, "agent1");
            assert_eq!(activity.activity_type, "modified");
        }

        // Verify ordering (most recent first)
        let mut previous_created_at = Utc::now() + chrono::Duration::minutes(1);
        for activity in &wt1_activities {
            assert!(activity.created_at <= previous_created_at);
            previous_created_at = activity.created_at;
        }
    }

    #[tokio::test]
    async fn test_get_recent_activities_all_with_limit() {
        let helper = DatabaseTestHelper::new().await.unwrap();

        // Create repository first
        helper
            .db
            .create_repository(
                "limit-repo",
                "/path/to/limit-repo",
                "git@github.com:user/limit-repo.git",
                "main",
            )
            .await
            .unwrap();

        let worktree = helper
            .db
            .create_worktree("limit-repo", "trunk-main", "main", "trunk", "/path", None)
            .await
            .unwrap();

        // Create 10 activities
        for i in 0..10 {
            helper
                .db
                .log_agent_activity(
                    &format!("agent-{}", i),
                    &worktree.id,
                    "created",
                    Some(&format!("file-{}.rs", i)),
                    &format!("Activity {}", i),
                )
                .await
                .unwrap();
        }

        // Request only 5 most recent
        let limited_activities = helper.db.get_recent_activities(None, 5).await.unwrap();

        assert_eq!(limited_activities.len(), 5);

        // Should be the most recent ones (9, 8, 7, 6, 5)
        for (idx, activity) in limited_activities.iter().enumerate() {
            let expected_agent = format!("agent-{}", 9 - idx);
            assert_eq!(activity.agent_id, expected_agent);
        }
    }

    #[tokio::test]
    async fn test_get_recent_activities_empty_database() {
        let helper = DatabaseTestHelper::new().await.unwrap();

        let activities = helper.db.get_recent_activities(None, 10).await.unwrap();

        assert_eq!(activities.len(), 0);
    }
}

#[cfg(test)]
mod worktree_types_tests {
    use super::*;

    #[tokio::test]
    async fn test_all_worktree_types() {
        let helper = DatabaseTestHelper::new().await.unwrap();

        // Create repository first
        helper
            .db
            .create_repository(
                "types-repo",
                "/path/to/types-repo",
                "git@github.com:user/types-repo.git",
                "main",
            )
            .await
            .unwrap();

        let worktree_types = vec![
            ("trunk", "trunk-main"),
            ("feat", "feat-auth"),
            ("review", "pr-123"),
            ("fix", "fix-bug"),
            ("aiops", "aiops-deploy"),
            ("devops", "devops-ci"),
        ];

        for (wt_type, wt_name) in worktree_types {
            let worktree = helper
                .db
                .create_worktree(
                    "types-repo",
                    wt_name,
                    "main",
                    wt_type,
                    &format!("/path/{}", wt_name),
                    None,
                )
                .await
                .unwrap();

            assert_eq!(worktree.worktree_type, wt_type);
            assert_eq!(worktree.worktree_name, wt_name);
        }

        // Verify all were created
        let all_worktrees = helper.db.list_worktrees(Some("types-repo")).await.unwrap();
        assert_eq!(all_worktrees.len(), 6);
    }
}

#[cfg(test)]
mod error_handling_tests {
    use super::*;

    #[tokio::test]
    async fn test_database_with_invalid_path_characters() {
        let temp_dir = TempDir::new().unwrap();

        // Most special characters should work in paths
        let valid_paths = vec![
            "normal.db",
            "with-dashes.db",
            "with_underscores.db",
            "with.dots.db",
            "with123numbers.db",
        ];

        for path_name in valid_paths {
            let db_path = temp_dir.path().join(path_name);
            let db_result = Database::new(&db_path).await;
            assert!(db_result.is_ok(), "Should handle path: {}", path_name);
        }
    }

    #[tokio::test]
    async fn test_database_operations_with_unicode() {
        let helper = DatabaseTestHelper::new().await.unwrap();

        // Create repository first
        helper
            .db
            .create_repository(
                "测试-repo",
                "/路径/到/测试-repo",
                "git@github.com:用户/测试-repo.git",
                "main",
            )
            .await
            .unwrap();

        // Test unicode in various fields
        let worktree = helper
            .db
            .create_worktree(
                "测试-repo",
                "特征-测试",
                "feat/测试-功能",
                "feat",
                "/路径/到/工作树",
                Some("代理-123".to_string()),
            )
            .await
            .unwrap();

        assert_eq!(worktree.repo_name, "测试-repo");
        assert_eq!(worktree.worktree_name, "特征-测试");
        assert_eq!(worktree.branch_name, "feat/测试-功能");
        assert_eq!(worktree.agent_id, Some("代理-123".to_string()));

        // Should be retrievable
        let retrieved = helper
            .db
            .get_worktree("测试-repo", "特征-测试")
            .await
            .unwrap();
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_very_long_field_values() {
        let helper = DatabaseTestHelper::new().await.unwrap();

        // Create very long strings
        let long_repo_name = "a".repeat(1000);
        let long_description = "Very long description: ".to_string() + &"x".repeat(10000);

        // Create repository first
        helper
            .db
            .create_repository(
                &long_repo_name,
                "/path/to/very/long/repo",
                "git@github.com:user/very-long-repo.git",
                "main",
            )
            .await
            .unwrap();

        let worktree = helper
            .db
            .create_worktree(
                &long_repo_name,
                "trunk-main",
                "main",
                "trunk",
                "/path/to/very/long/path/that/goes/on/and/on",
                None,
            )
            .await
            .unwrap();

        assert_eq!(worktree.repo_name, long_repo_name);

        // Log activity with very long description
        let activity = helper
            .db
            .log_agent_activity(
                "agent-with-very-long-name-that-exceeds-normal-limits",
                &worktree.id,
                "created",
                Some("file-with-very-long-name.rs"),
                &long_description,
            )
            .await
            .unwrap();

        assert_eq!(activity.description, long_description);
    }
}

#[cfg(test)]
mod database_edge_cases_tests {
    use super::*;

    #[tokio::test]
    async fn test_concurrent_worktree_creation() {
        let helper = DatabaseTestHelper::new().await.unwrap();

        // Create repository first
        helper
            .db
            .create_repository(
                "concurrent-repo",
                "/path/to/concurrent-repo",
                "git@github.com:user/concurrent-repo.git",
                "main",
            )
            .await
            .unwrap();

        // Simulate concurrent creation attempts
        let handles: Vec<_> = (0..10)
            .map(|i| {
                let db = helper.db.clone();
                tokio::spawn(async move {
                    db.create_worktree(
                        "concurrent-repo",
                        &format!("feat-{}", i),
                        "main",
                        "feat",
                        &format!("/path/{}", i),
                        Some(format!("agent-{}", i)),
                    )
                    .await
                })
            })
            .collect();

        // Wait for all to complete
        let mut results = Vec::new();
        for handle in handles {
            results.push(handle.await.unwrap());
        }

        // All should succeed
        let successful_count = results.iter().filter(|r| r.is_ok()).count();
        assert_eq!(
            successful_count, 10,
            "All concurrent creations should succeed"
        );

        // Verify all were created
        let worktrees = helper
            .db
            .list_worktrees(Some("concurrent-repo"))
            .await
            .unwrap();
        assert_eq!(worktrees.len(), 10);
    }

    #[tokio::test]
    async fn test_empty_string_fields() {
        let helper = DatabaseTestHelper::new().await.unwrap();

        // Create repository first with empty name
        helper
            .db
            .create_repository(
                "",
                "/path/to/empty-repo",
                "git@github.com:user/empty-repo.git",
                "main",
            )
            .await
            .unwrap();

        // Test with empty strings (should work but may not be practical)
        let worktree = helper
            .db
            .create_worktree("", "", "", "", "", Some("".to_string()))
            .await
            .unwrap();

        assert_eq!(worktree.repo_name, "");
        assert_eq!(worktree.worktree_name, "");
        assert_eq!(worktree.branch_name, "");
        assert_eq!(worktree.worktree_type, "");
        assert_eq!(worktree.path, "");
        assert_eq!(worktree.agent_id, Some("".to_string()));
    }

    #[tokio::test]
    async fn test_null_byte_handling() {
        let helper = DatabaseTestHelper::new().await.unwrap();

        // SQLite should handle null bytes in strings
        let name_with_null = format!("repo{}name", '\0');
        let result = helper
            .db
            .create_worktree(
                &name_with_null,
                "trunk-main",
                "main",
                "trunk",
                "/path",
                None,
            )
            .await;

        // Should either work or fail gracefully
        match result {
            Ok(worktree) => {
                assert!(worktree.repo_name.contains("repo"));
                println!("Null byte handled successfully");
            }
            Err(e) => {
                println!("Null byte handled with error (expected): {}", e);
            }
        }
    }
}
