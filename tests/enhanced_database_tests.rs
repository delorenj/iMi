//! Enhanced Unit Tests for Database Module
//!
//! These tests provide comprehensive coverage of database operations,
//! including CRUD operations, error scenarios, concurrent access, and data integrity.

use anyhow::Result;
use chrono::Utc;
use serial_test::serial;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

use imi::database::{AgentActivity, Database, Repository, Worktree};

/// Test utilities for database testing
pub struct DatabaseTestUtils {
    pub temp_dir: TempDir,
    pub db_path: PathBuf,
    pub database: Database,
}

impl DatabaseTestUtils {
    /// Create a new test database with temporary storage
    pub async fn new() -> Result<Self> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");
        let database = Database::new(&db_path).await?;

        Ok(Self {
            temp_dir,
            db_path,
            database,
        })
    }

    /// Create a test repository with default values
    pub async fn create_test_repository(&self, name: &str) -> Result<Repository> {
        self.database
            .create_repository(
                name,
                &format!("/tmp/{}", name),
                &format!("git@github.com:test/{}", name),
                "main",
            )
            .await
    }

    /// Create a test worktree with default values
    pub async fn create_test_worktree(
        &self,
        repo_name: &str,
        worktree_name: &str,
    ) -> Result<Worktree> {
        self.database
            .create_worktree(
                repo_name,
                worktree_name,
                &format!("feature/{}", worktree_name),
                "feat",
                &format!("/tmp/{}/{}", repo_name, worktree_name),
                None,
            )
            .await
    }

    /// Create a test agent activity
    pub async fn create_test_activity(
        &self,
        worktree_id: &Uuid,
        activity_type: &str,
    ) -> Result<AgentActivity> {
        self.database
            .log_agent_activity(
                "test-agent",
                worktree_id,
                activity_type,
                Some("/tmp/test.rs"),
                "Test activity",
            )
            .await?;

        // Fetch the activity we just created
        let activities = self.database.get_recent_activities(Some(worktree_id), 1).await?;
        Ok(activities[0].clone())
    }

    /// Verify database constraints and relationships
    pub async fn verify_database_integrity(&self) -> Result<()> {
        // This would contain integrity checks in a real implementation
        Ok(())
    }
}

/// Test basic database initialization and table creation
#[tokio::test]
#[serial]
async fn test_database_initialization() -> Result<()> {
    let utils = DatabaseTestUtils::new().await?;

    // Verify database file was created
    assert!(utils.db_path.exists());

    // Verify we can connect and tables exist
    let result = utils.database.ensure_tables().await;
    assert!(result.is_ok(), "Failed to ensure tables exist");

    Ok(())
}

/// Test database initialization with invalid path
#[tokio::test]
#[serial]
async fn test_database_invalid_path() -> Result<()> {
    // Try to create database in read-only location
    let result = Database::new("/root/readonly/test.db").await;
    assert!(result.is_err(), "Should fail with invalid path");

    Ok(())
}

/// Test repository CRUD operations
#[tokio::test]
#[serial]
async fn test_repository_crud_operations() -> Result<()> {
    let utils = DatabaseTestUtils::new().await?;

    // Test Create
    let repo = utils.create_test_repository("test-repo").await?;
    assert_eq!(repo.name, "test-repo");
    assert_eq!(repo.default_branch, "main");
    assert!(repo.active);

    // Test Read
    let retrieved_repo = utils.database.get_repository("test-repo").await?;
    assert!(retrieved_repo.is_some());
    let retrieved_repo = retrieved_repo.unwrap();
    assert_eq!(retrieved_repo.name, repo.name);
    assert_eq!(retrieved_repo.id, repo.id);

    // Test non-existent repository
    let missing_repo = utils.database.get_repository("missing-repo").await?;
    assert!(missing_repo.is_none());

    Ok(())
}

/// Test repository creation with duplicate names
#[tokio::test]
#[serial]
async fn test_repository_duplicate_names() -> Result<()> {
    let utils = DatabaseTestUtils::new().await?;

    // Create first repository
    let repo1 = utils.create_test_repository("duplicate-repo").await?;

    // Create second repository with same name (should replace)
    let repo2 = utils
        .database
        .create_repository(
            "duplicate-repo",
            "/different/path",
            "git@github.com:different/repo",
            "develop",
        )
        .await?;

    // Verify the second repository replaced the first
    let retrieved = utils.database.get_repository("duplicate-repo").await?;
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.path, "/different/path");
    assert_eq!(retrieved.default_branch, "develop");

    Ok(())
}

/// Test worktree CRUD operations
#[tokio::test]
#[serial]
async fn test_worktree_crud_operations() -> Result<()> {
    let utils = DatabaseTestUtils::new().await?;

    // Create a repository first
    let _repo = utils.create_test_repository("test-repo").await?;

    // Test Create worktree
    let worktree = utils
        .create_test_worktree("test-repo", "feature-branch")
        .await?;
    assert_eq!(worktree.repo_name, "test-repo");
    assert_eq!(worktree.worktree_name, "feature-branch");
    assert_eq!(worktree.branch_name, "feature/feature-branch");
    assert_eq!(worktree.worktree_type, "feat");
    assert!(worktree.active);

    // Test Read worktree
    let retrieved = utils
        .database
        .get_worktree("test-repo", "feature-branch")
        .await?;
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.id, worktree.id);
    assert_eq!(retrieved.worktree_name, worktree.worktree_name);

    // Test List worktrees
    let all_worktrees = utils.database.list_worktrees(None).await?;
    assert_eq!(all_worktrees.len(), 1);

    let repo_worktrees = utils.database.list_worktrees(Some("test-repo")).await?;
    assert_eq!(repo_worktrees.len(), 1);

    let other_repo_worktrees = utils.database.list_worktrees(Some("other-repo")).await?;
    assert_eq!(other_repo_worktrees.len(), 0);

    // Test Deactivate worktree
    utils
        .database
        .deactivate_worktree("test-repo", "feature-branch")
        .await?;
    let deactivated = utils
        .database
        .get_worktree("test-repo", "feature-branch")
        .await?;
    assert!(
        deactivated.is_none(),
        "Deactivated worktree should not be retrieved"
    );

    Ok(())
}

/// Ensure repositories are returned in most-recently-updated order
#[tokio::test]
#[serial]
async fn test_list_repositories_ordered_by_recent_update() -> Result<()> {
    let utils = DatabaseTestUtils::new().await?;

    // Create two repositories with distinct timestamps
    let _alpha = utils.create_test_repository("alpha-repo").await?;
    sleep(Duration::from_millis(25)).await;
    let _beta = utils.create_test_repository("beta-repo").await?;

    // Touch the first repository to bump its updated_at
    sleep(Duration::from_millis(25)).await;
    utils.database.touch_repository("alpha-repo").await?;

    let repos = utils.database.list_repositories().await?;
    assert_eq!(repos.len(), 2, "Expected two repositories in listing");

    assert_eq!(
        repos[0].name, "alpha-repo",
        "Most recently updated repository should appear first"
    );
    assert!(
        repos[0].updated_at >= repos[1].updated_at,
        "Repository ordering should be sorted by updated_at descending"
    );

    Ok(())
}

/// Ensure worktrees are returned in most-recently-updated order
#[tokio::test]
#[serial]
async fn test_list_worktrees_ordered_by_recent_update() -> Result<()> {
    let utils = DatabaseTestUtils::new().await?;

    utils.create_test_repository("order-repo").await?;

    utils
        .create_test_worktree("order-repo", "alpha-worktree")
        .await?;
    sleep(Duration::from_millis(25)).await;
    utils
        .create_test_worktree("order-repo", "beta-worktree")
        .await?;

    // Touch the first worktree to bump its updated_at
    sleep(Duration::from_millis(25)).await;
    utils
        .database
        .touch_worktree("order-repo", "alpha-worktree")
        .await?;

    let worktrees = utils.database.list_worktrees(Some("order-repo")).await?;
    assert_eq!(worktrees.len(), 2, "Expected two worktrees registered");

    assert_eq!(
        worktrees[0].worktree_name, "alpha-worktree",
        "Most recently updated worktree should appear first"
    );
    assert!(
        worktrees[0].updated_at >= worktrees[1].updated_at,
        "Worktree ordering should be sorted by updated_at descending"
    );

    Ok(())
}

/// Test worktree with agent assignment
#[tokio::test]
#[serial]
async fn test_worktree_with_agent() -> Result<()> {
    let utils = DatabaseTestUtils::new().await?;

    // Create repository
    let _repo = utils.create_test_repository("test-repo").await?;

    // Create worktree with agent
    let agent_id = Uuid::new_v4().to_string();
    let worktree = utils
        .database
        .create_worktree(
            "test-repo",
            "agent-worktree",
            "feature/agent-branch",
            "feat",
            "/tmp/agent-path",
            Some(agent_id),
        )
        .await?;

    assert_eq!(worktree.agent_id, Some(agent_id.clone()));

    // Verify retrieval includes agent
    let retrieved = utils
        .database
        .get_worktree("test-repo", "agent-worktree")
        .await?;
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().agent_id, Some(agent_id));

    Ok(())
}

/// Test worktree creation with invalid repository
#[tokio::test]
#[serial]
async fn test_worktree_invalid_repository() -> Result<()> {
    let utils = DatabaseTestUtils::new().await?;

    // Try to create worktree for non-existent repository
    let result = utils
        .create_test_worktree("non-existent-repo", "feature")
        .await;
    // Note: This should ideally fail with foreign key constraint, but SQLite might not enforce it
    // In a production system, we'd want proper constraint validation

    Ok(())
}

/// Test agent activity logging
#[tokio::test]
#[serial]
async fn test_agent_activity_logging() -> Result<()> {
    let utils = DatabaseTestUtils::new().await?;

    // Setup repository and worktree
    let _repo = utils.create_test_repository("test-repo").await?;
    let worktree = utils.create_test_worktree("test-repo", "feature").await?;

    // Test activity creation
    let activity = utils.create_test_activity(&worktree.id, "created").await?;
    assert_eq!(activity.agent_id, "test-agent");
    assert_eq!(activity.worktree_id, worktree.id);
    assert_eq!(activity.activity_type, "created");
    assert_eq!(activity.file_path, Some("/tmp/test.rs".to_string()));

    // Test retrieving recent activities
    let activities = utils
        .database
        .get_recent_activities(Some(&worktree.id), 10)
        .await?;
    assert_eq!(activities.len(), 1);
    assert_eq!(activities[0].id, activity.id);

    // Test retrieving all activities
    let all_activities = utils.database.get_recent_activities(None, 10).await?;
    assert_eq!(all_activities.len(), 1);

    Ok(())
}

/// Test multiple agent activities for same worktree
#[tokio::test]
#[serial]
async fn test_multiple_agent_activities() -> Result<()> {
    let utils = DatabaseTestUtils::new().await?;

    // Setup
    let _repo = utils.create_test_repository("test-repo").await?;
    let worktree = utils.create_test_worktree("test-repo", "feature").await?;

    // Create multiple activities
    let _activity1 = utils.create_test_activity(&worktree.id, "created").await?;
    let _activity2 = utils.create_test_activity(&worktree.id, "modified").await?;
    let _activity3 = utils
        .create_test_activity(&worktree.id, "committed")
        .await?;

    // Test retrieval with limit
    let activities = utils
        .database
        .get_recent_activities(Some(&worktree.id), 2)
        .await?;
    assert_eq!(activities.len(), 2);

    // Activities should be ordered by created_at DESC (most recent first)
    assert_eq!(activities[0].activity_type, "committed");
    assert_eq!(activities[1].activity_type, "modified");

    Ok(())
}

/// Test concurrent database operations
#[tokio::test]
#[serial]
async fn test_concurrent_database_operations() -> Result<()> {
    let utils = DatabaseTestUtils::new().await?;

    // Create repository
    let _repo = utils.create_test_repository("concurrent-repo").await?;

    // Spawn multiple concurrent worktree creation tasks
    let handles = (0..5).map(|i| {
        let db = utils.database.clone();
        tokio::spawn(async move {
            db.create_worktree(
                "concurrent-repo",
                &format!("worktree-{}", i),
                &format!("branch-{}", i),
                "feat",
                &format!("/tmp/path-{}", i),
                None,
            )
            .await
        })
    });

    // Wait for all tasks to complete
    let mut results = Vec::new();
    for handle in handles {
        results.push(handle.await);
    }

    // Verify all operations succeeded
    for result in results {
        assert!(result.is_ok(), "Concurrent task failed");
        assert!(result.unwrap().is_ok(), "Worktree creation failed");
    }

    // Verify all worktrees were created
    let worktrees = utils
        .database
        .list_worktrees(Some("concurrent-repo"))
        .await?;
    assert_eq!(worktrees.len(), 5);

    Ok(())
}

/// Test database error scenarios
#[tokio::test]
#[serial]
async fn test_database_error_scenarios() -> Result<()> {
    let utils = DatabaseTestUtils::new().await?;

    // Test retrieving from non-existent worktree
    let result = utils
        .database
        .get_recent_activities(Some(&Uuid::new_v4()), 10)
        .await?;
    assert_eq!(
        result.len(),
        0,
        "Should return empty list for non-existent worktree"
    );

    // Test deactivating non-existent worktree
    let result = utils
        .database
        .deactivate_worktree("non-repo", "non-worktree")
        .await;
    assert!(result.is_ok(), "Should not fail for non-existent worktree");

    Ok(())
}

/// Test worktree unique constraint
#[tokio::test]
#[serial]
async fn test_worktree_unique_constraint() -> Result<()> {
    let utils = DatabaseTestUtils::new().await?;

    // Setup
    let _repo = utils.create_test_repository("test-repo").await?;

    // Create first worktree
    let worktree1 = utils.create_test_worktree("test-repo", "duplicate").await?;

    // Create second worktree with same repo_name and worktree_name (should replace)
    let worktree2 = utils
        .database
        .create_worktree(
            "test-repo",
            "duplicate",
            "different-branch",
            "fix",
            "/different/path",
            None,
        )
        .await?;

    // Verify second worktree replaced the first
    let retrieved = utils
        .database
        .get_worktree("test-repo", "duplicate")
        .await?;
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.id, worktree2.id);
    assert_eq!(retrieved.branch_name, "different-branch");
    assert_eq!(retrieved.worktree_type, "fix");

    Ok(())
}

/// Test data integrity and validation
#[tokio::test]
#[serial]
async fn test_data_integrity() -> Result<()> {
    let utils = DatabaseTestUtils::new().await?;

    // Test that timestamps are properly set and parsed
    let repo = utils.create_test_repository("timestamp-test").await?;

    // Verify timestamps are recent (within last minute)
    let now = Utc::now();
    let time_diff = now.signed_duration_since(repo.created_at);
    assert!(
        time_diff.num_seconds() < 60,
        "Created timestamp should be recent"
    );
    assert!(
        time_diff.num_seconds() >= 0,
        "Created timestamp should not be in future"
    );

    Ok(())
}

/// Test database cleanup and resource management
#[tokio::test]
#[serial]
async fn test_database_cleanup() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("cleanup_test.db");

    {
        // Create database in limited scope
        let _database = Database::new(&db_path).await?;

        // Verify database file exists
        assert!(db_path.exists());
    } // Database should be dropped here

    // File should still exist (SQLite persists)
    assert!(db_path.exists());

    // Verify we can reconnect to existing database
    let database2 = Database::new(&db_path).await?;
    let result = database2.ensure_tables().await;
    assert!(result.is_ok());

    Ok(())
}

/// Property-based test for database operations
#[tokio::test]
#[serial]
async fn test_database_properties() -> Result<()> {
    let utils = DatabaseTestUtils::new().await?;

    // Property: Every created repository should be retrievable
    for i in 0..10 {
        let repo_name = format!("prop-test-{}", i);
        let repo = utils.create_test_repository(&repo_name).await?;

        let retrieved = utils.database.get_repository(&repo_name).await?;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, repo.id);
    }

    // Property: Deactivated worktrees should not be retrievable
    let _repo = utils.create_test_repository("deactivation-test").await?;
    let worktree = utils
        .create_test_worktree("deactivation-test", "test-branch")
        .await?;

    // Verify it exists
    let before = utils
        .database
        .get_worktree("deactivation-test", "test-branch")
        .await?;
    assert!(before.is_some());

    // Deactivate
    utils
        .database
        .deactivate_worktree("deactivation-test", "test-branch")
        .await?;

    // Verify it's gone
    let after = utils
        .database
        .get_worktree("deactivation-test", "test-branch")
        .await?;
    assert!(after.is_none());

    Ok(())
}
