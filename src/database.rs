use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPool;
use sqlx::Row;
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Database {
    pool: PgPool,
}

// ============================================================================
// Models matching new PostgreSQL schema
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    #[sqlx(rename = "remote_origin")]
    pub remote_url: String,  // Keep remote_url for API compatibility
    pub default_branch: String,
    #[sqlx(rename = "trunk_path")]
    pub path: String,  // Renamed from trunk_path for backwards compatibility
    pub description: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub active: bool,
}

// Alias for backwards compatibility
pub type Repository = Project;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Worktree {
    pub id: Uuid,
    pub project_id: Uuid,
    pub type_id: i32,
    pub name: String,
    pub branch_name: String,
    pub path: String,
    pub agent_id: Option<String>,

    // In-flight work tracking
    pub has_uncommitted_changes: Option<bool>,
    pub uncommitted_files_count: Option<i32>,
    pub ahead_of_trunk: Option<i32>,
    pub behind_trunk: Option<i32>,
    pub last_commit_hash: Option<String>,
    pub last_commit_message: Option<String>,
    pub last_sync_at: Option<DateTime<Utc>>,

    // Merge tracking
    pub merged_at: Option<DateTime<Utc>>,
    pub merged_by: Option<String>,
    pub merge_commit_hash: Option<String>,

    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub active: bool,

    // Backwards compatibility - these are computed/loaded separately
    #[sqlx(default)]
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub repo_name: String,

    #[sqlx(default)]
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub worktree_name: String,

    #[sqlx(default)]
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub worktree_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AgentActivity {
    pub id: Uuid,
    pub agent_id: String,
    pub worktree_id: Uuid,
    pub activity_type: String,
    pub file_path: Option<String>,
    pub description: String,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WorktreeType {
    pub id: i32,
    pub name: String,
    pub branch_prefix: String,
    pub worktree_prefix: String,
    pub description: Option<String>,
    pub is_builtin: bool,
    pub color: Option<String>,
    pub icon: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// Database implementation
// ============================================================================

impl Database {
    /// Connect to PostgreSQL database
    pub async fn new<P: AsRef<Path>>(_database_path: P) -> Result<Self> {
        // Get connection string from environment or use default
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| {
                "postgresql://imi:imi_dev_password_2026@192.168.1.12:5432/imi".to_string()
            });

        let pool = PgPool::connect(&database_url)
            .await
            .context("Failed to connect to PostgreSQL database")?;

        Ok(Self { pool })
    }

    /// Ensure database tables exist - no-op for PostgreSQL (migrations are external)
    pub async fn ensure_tables(&self) -> Result<()> {
        // Migrations are handled externally via SQL files
        // Just verify connection works
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .context("Failed to verify database connection")?;
        Ok(())
    }

    // ========================================================================
    // Project (Repository) operations
    // ========================================================================

    pub async fn create_repository(
        &self,
        name: &str,
        path: &str,
        remote_url: &str,
        default_branch: &str,
    ) -> Result<Project> {
        // Use register_project() helper function
        let row = sqlx::query(
            r#"
            SELECT register_project($1, $2, $3, $4, '{}'::jsonb) as project_id
            "#
        )
        .bind(name)
        .bind(remote_url)  // remote_origin in new schema
        .bind(default_branch)
        .bind(path)  // This becomes trunk_path
        .fetch_one(&self.pool)
        .await
        .context("Failed to register project")?;

        let project_id: Uuid = row.get("project_id");

        // Fetch the created project
        self.get_repository_by_id(&project_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Project not found after creation"))
    }

    pub async fn get_repository(&self, name: &str) -> Result<Option<Project>> {
        let project = sqlx::query_as::<_, Project>(
            r#"
            SELECT id, name, remote_origin, default_branch, trunk_path,
                   description, metadata, created_at, updated_at, active
            FROM projects
            WHERE name = $1 AND active = TRUE
            "#
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch project")?;

        Ok(project)
    }

    pub async fn get_repository_by_id(&self, id: &Uuid) -> Result<Option<Project>> {
        let project = sqlx::query_as::<_, Project>(
            r#"
            SELECT id, name, remote_origin, default_branch, trunk_path,
                   description, metadata, created_at, updated_at, active
            FROM projects
            WHERE id = $1 AND active = TRUE
            "#
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch project by ID")?;

        Ok(project)
    }

    pub async fn update_repository_path(&self, name: &str, new_path: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE projects
            SET trunk_path = $1, updated_at = NOW()
            WHERE name = $2 AND active = TRUE
            "#
        )
        .bind(new_path)
        .bind(name)
        .execute(&self.pool)
        .await
        .context("Failed to update project path")?;

        Ok(())
    }

    pub async fn list_repositories(&self) -> Result<Vec<Project>> {
        let projects = sqlx::query_as::<_, Project>(
            r#"
            SELECT id, name, remote_origin, default_branch, trunk_path,
                   description, metadata, created_at, updated_at, active
            FROM projects
            WHERE active = TRUE
            ORDER BY name
            "#
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to list projects")?;

        Ok(projects)
    }

    pub async fn touch_repository(&self, name: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE projects
            SET updated_at = NOW()
            WHERE name = $1 AND active = TRUE
            "#
        )
        .bind(name)
        .execute(&self.pool)
        .await
        .context("Failed to touch project")?;

        Ok(())
    }

    // ========================================================================
    // Worktree operations
    // ========================================================================

    pub async fn create_worktree(
        &self,
        repo_name: &str,
        worktree_name: &str,
        branch_name: &str,
        worktree_type: &str,
        path: &str,
        agent_id: Option<String>,
    ) -> Result<Worktree> {
        // First get project_id from repo_name
        let project = self.get_repository(repo_name)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Project not found: {}", repo_name))?;

        // Use register_worktree() helper function
        let row = sqlx::query(
            r#"
            SELECT register_worktree($1, $2, $3, $4, $5, $6, '{}'::jsonb) as worktree_id
            "#
        )
        .bind(project.id)
        .bind(worktree_type)
        .bind(worktree_name)
        .bind(branch_name)
        .bind(path)
        .bind(agent_id.as_deref())
        .fetch_one(&self.pool)
        .await
        .context("Failed to register worktree")?;

        let worktree_id: Uuid = row.get("worktree_id");

        // Fetch the created worktree
        self.get_worktree_by_id(&worktree_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Worktree not found after creation"))
    }

    pub async fn get_worktree(
        &self,
        repo_name: &str,
        worktree_name: &str,
    ) -> Result<Option<Worktree>> {
        // Get project first to get project_id
        let project = self.get_repository(repo_name).await?;
        let project_id = match project {
            Some(p) => p.id,
            None => return Ok(None),
        };

        let worktree = sqlx::query_as::<_, Worktree>(
            r#"
            SELECT id, project_id, type_id, name, branch_name, path, agent_id,
                   has_uncommitted_changes, uncommitted_files_count, ahead_of_trunk, behind_trunk,
                   last_commit_hash, last_commit_message, last_sync_at,
                   merged_at, merged_by, merge_commit_hash,
                   metadata, created_at, updated_at, active
            FROM worktrees
            WHERE project_id = $1 AND name = $2 AND active = TRUE
            "#
        )
        .bind(project_id)
        .bind(worktree_name)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch worktree")?;

        Ok(worktree)
    }

    pub async fn get_worktree_by_id(&self, id: &Uuid) -> Result<Option<Worktree>> {
        let worktree = sqlx::query_as::<_, Worktree>(
            r#"
            SELECT id, project_id, type_id, name, branch_name, path, agent_id,
                   has_uncommitted_changes, uncommitted_files_count, ahead_of_trunk, behind_trunk,
                   last_commit_hash, last_commit_message, last_sync_at,
                   merged_at, merged_by, merge_commit_hash,
                   metadata, created_at, updated_at, active
            FROM worktrees
            WHERE id = $1 AND active = TRUE
            "#
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch worktree by ID")?;

        Ok(worktree)
    }

    pub async fn list_worktrees(&self, repo_name: Option<&str>) -> Result<Vec<Worktree>> {
        let worktrees = if let Some(name) = repo_name {
            // Get project_id first
            let project = self.get_repository(name)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Project not found: {}", name))?;

            sqlx::query_as::<_, Worktree>(
                r#"
                SELECT id, project_id, type_id, name, branch_name, path, agent_id,
                       has_uncommitted_changes, uncommitted_files_count, ahead_of_trunk, behind_trunk,
                       last_commit_hash, last_commit_message, last_sync_at,
                       merged_at, merged_by, merge_commit_hash,
                       metadata, created_at, updated_at, active
                FROM worktrees
                WHERE project_id = $1 AND active = TRUE
                ORDER BY created_at DESC
                "#
            )
            .bind(project.id)
            .fetch_all(&self.pool)
            .await
            .context("Failed to list worktrees for project")?
        } else {
            sqlx::query_as::<_, Worktree>(
                r#"
                SELECT id, project_id, type_id, name, branch_name, path, agent_id,
                       has_uncommitted_changes, uncommitted_files_count, ahead_of_trunk, behind_trunk,
                       last_commit_hash, last_commit_message, last_sync_at,
                       merged_at, merged_by, merge_commit_hash,
                       metadata, created_at, updated_at, active
                FROM worktrees
                WHERE active = TRUE
                ORDER BY created_at DESC
                "#
            )
            .fetch_all(&self.pool)
            .await
            .context("Failed to list all worktrees")?
        };

        Ok(worktrees)
    }

    pub async fn list_all_worktrees(&self, repo_name: Option<&str>) -> Result<Vec<Worktree>> {
        // Same as list_worktrees but includes inactive
        let worktrees = if let Some(name) = repo_name {
            let project = self.get_repository(name)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Project not found: {}", name))?;

            sqlx::query_as::<_, Worktree>(
                r#"
                SELECT id, project_id, type_id, name, branch_name, path, agent_id,
                       has_uncommitted_changes, uncommitted_files_count, ahead_of_trunk, behind_trunk,
                       last_commit_hash, last_commit_message, last_sync_at,
                       merged_at, merged_by, merge_commit_hash,
                       metadata, created_at, updated_at, active
                FROM worktrees
                WHERE project_id = $1
                ORDER BY created_at DESC
                "#
            )
            .bind(project.id)
            .fetch_all(&self.pool)
            .await
            .context("Failed to list all worktrees for project")?
        } else {
            sqlx::query_as::<_, Worktree>(
                r#"
                SELECT id, project_id, type_id, name, branch_name, path, agent_id,
                       has_uncommitted_changes, uncommitted_files_count, ahead_of_trunk, behind_trunk,
                       last_commit_hash, last_commit_message, last_sync_at,
                       merged_at, merged_by, merge_commit_hash,
                       metadata, created_at, updated_at, active
                FROM worktrees
                ORDER BY created_at DESC
                "#
            )
            .fetch_all(&self.pool)
            .await
            .context("Failed to list all worktrees")?
        };

        Ok(worktrees)
    }

    pub async fn deactivate_worktree(&self, repo_name: &str, worktree_name: &str) -> Result<()> {
        let project = self.get_repository(repo_name)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Project not found: {}", repo_name))?;

        sqlx::query(
            r#"
            UPDATE worktrees
            SET active = FALSE, updated_at = NOW()
            WHERE project_id = $1 AND name = $2
            "#
        )
        .bind(project.id)
        .bind(worktree_name)
        .execute(&self.pool)
        .await
        .context("Failed to deactivate worktree")?;

        Ok(())
    }

    pub async fn touch_worktree(&self, repo_name: &str, worktree_name: &str) -> Result<()> {
        let project = self.get_repository(repo_name)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Project not found: {}", repo_name))?;

        sqlx::query(
            r#"
            UPDATE worktrees
            SET updated_at = NOW()
            WHERE project_id = $1 AND name = $2 AND active = TRUE
            "#
        )
        .bind(project.id)
        .bind(worktree_name)
        .execute(&self.pool)
        .await
        .context("Failed to touch worktree")?;

        Ok(())
    }

    pub async fn find_worktree_by_name(&self, worktree_name: &str) -> Result<Option<Worktree>> {
        let worktree = sqlx::query_as::<_, Worktree>(
            r#"
            SELECT id, project_id, type_id, name, branch_name, path, agent_id,
                   has_uncommitted_changes, uncommitted_files_count, ahead_of_trunk, behind_trunk,
                   last_commit_hash, last_commit_message, last_sync_at,
                   merged_at, merged_by, merge_commit_hash,
                   metadata, created_at, updated_at, active
            FROM worktrees
            WHERE name = $1 AND active = TRUE
            "#
        )
        .bind(worktree_name)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to find worktree by name")?;

        Ok(worktree)
    }

    // ========================================================================
    // Agent activity operations
    // ========================================================================

    pub async fn log_agent_activity(
        &self,
        agent_id: &str,
        worktree_id: &Uuid,
        activity_type: &str,
        file_path: Option<&str>,
        description: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO agent_activities (agent_id, worktree_id, activity_type, file_path, description, metadata)
            VALUES ($1, $2, $3, $4, $5, '{}'::jsonb)
            "#
        )
        .bind(agent_id)
        .bind(worktree_id)
        .bind(activity_type)
        .bind(file_path)
        .bind(description)
        .execute(&self.pool)
        .await
        .context("Failed to log agent activity")?;

        Ok(())
    }

    pub async fn get_recent_activities(
        &self,
        worktree_id: Option<&Uuid>,
        limit: i64,
    ) -> Result<Vec<AgentActivity>> {
        let activities = if let Some(wt_id) = worktree_id {
            sqlx::query_as::<_, AgentActivity>(
                r#"
                SELECT id, agent_id, worktree_id, activity_type, file_path, description, metadata, created_at
                FROM agent_activities
                WHERE worktree_id = $1
                ORDER BY created_at DESC
                LIMIT $2
                "#
            )
            .bind(wt_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .context("Failed to fetch recent activities for worktree")?
        } else {
            sqlx::query_as::<_, AgentActivity>(
                r#"
                SELECT id, agent_id, worktree_id, activity_type, file_path, description, metadata, created_at
                FROM agent_activities
                ORDER BY created_at DESC
                LIMIT $1
                "#
            )
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .context("Failed to fetch all recent activities")?
        };

        Ok(activities)
    }

    // ========================================================================
    // Worktree claim/release operations
    // ========================================================================

    pub async fn claim_worktree(&self, worktree_id: &Uuid, agent_id: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE worktrees
            SET agent_id = $1, updated_at = NOW()
            WHERE id = $2
            "#
        )
        .bind(agent_id)
        .bind(worktree_id)
        .execute(&self.pool)
        .await
        .context("Failed to claim worktree")?;

        Ok(())
    }

    pub async fn release_worktree(&self, worktree_id: &Uuid, agent_id: &str) -> Result<()> {
        // Verify the agent owns the worktree before releasing
        let result = sqlx::query(
            r#"
            UPDATE worktrees
            SET agent_id = NULL, updated_at = NOW()
            WHERE id = $1 AND agent_id = $2
            "#
        )
        .bind(worktree_id)
        .bind(agent_id)
        .execute(&self.pool)
        .await
        .context("Failed to release worktree")?;

        if result.rows_affected() == 0 {
            return Err(anyhow::anyhow!(
                "Cannot release worktree: not owned by agent '{}'",
                agent_id
            ));
        }

        Ok(())
    }

    // ========================================================================
    // Worktree type operations
    // ========================================================================

    pub async fn get_worktree_type(&self, name: &str) -> Result<WorktreeType> {
        let wt_type = sqlx::query_as::<_, WorktreeType>(
            r#"
            SELECT id, name, branch_prefix, worktree_prefix, description, is_builtin,
                   color, icon, metadata, created_at
            FROM worktree_types
            WHERE name = $1
            "#
        )
        .bind(name)
        .fetch_one(&self.pool)
        .await
        .context("Failed to fetch worktree type")?;

        Ok(wt_type)
    }

    pub async fn list_worktree_types(&self) -> Result<Vec<WorktreeType>> {
        let types = sqlx::query_as::<_, WorktreeType>(
            r#"
            SELECT id, name, branch_prefix, worktree_prefix, description, is_builtin,
                   color, icon, metadata, created_at
            FROM worktree_types
            ORDER BY name
            "#
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to list worktree types")?;

        Ok(types)
    }

    pub async fn add_worktree_type(
        &self,
        name: &str,
        branch_prefix: Option<&str>,
        worktree_prefix: Option<&str>,
        description: Option<&str>,
    ) -> Result<WorktreeType> {
        // Use defaults if not provided
        let branch_prefix = branch_prefix
            .map(String::from)
            .unwrap_or_else(|| format!("{}/", name));
        let worktree_prefix = worktree_prefix
            .map(String::from)
            .unwrap_or_else(|| format!("{}-", name));

        sqlx::query(
            r#"
            INSERT INTO worktree_types (name, branch_prefix, worktree_prefix, description, is_builtin, metadata)
            VALUES ($1, $2, $3, $4, FALSE, '{}'::jsonb)
            "#
        )
        .bind(name)
        .bind(&branch_prefix)
        .bind(&worktree_prefix)
        .bind(description)
        .execute(&self.pool)
        .await
        .context("Failed to add worktree type")?;

        // Fetch the created type
        self.get_worktree_type(name).await
    }

    pub async fn remove_worktree_type(&self, name: &str) -> Result<()> {
        sqlx::query(
            r#"
            DELETE FROM worktree_types
            WHERE name = $1 AND is_builtin = FALSE
            "#
        )
        .bind(name)
        .execute(&self.pool)
        .await
        .context("Failed to remove worktree type")?;

        Ok(())
    }

    // ========================================================================
    // Worktree metadata operations
    // ========================================================================

    pub async fn set_worktree_metadata(
        &self,
        worktree_id: &Uuid,
        key: &str,
        value: serde_json::Value,
    ) -> Result<()> {
        // Build the JSON object for merging
        let update_json = {
            let keys: Vec<&str> = key.split('.').collect();
            if keys.len() == 1 {
                // Simple key - direct update
                let mut obj = serde_json::Map::new();
                obj.insert(key.to_string(), value);
                serde_json::Value::Object(obj)
            } else {
                // Nested key - build from innermost to outermost
                let mut result = value;
                for k in keys.iter().rev() {
                    let mut obj = serde_json::Map::new();
                    obj.insert(k.to_string(), result);
                    result = serde_json::Value::Object(obj);
                }
                result
            }
        };

        sqlx::query(
            r#"
            UPDATE worktrees
            SET metadata = COALESCE(metadata, '{}'::jsonb) || $1::jsonb,
                updated_at = NOW()
            WHERE id = $2
            "#
        )
        .bind(&update_json)
        .bind(worktree_id)
        .execute(&self.pool)
        .await
        .context("Failed to set worktree metadata")?;

        Ok(())
    }

    pub async fn get_worktree_metadata(
        &self,
        worktree_id: &Uuid,
        key: Option<&str>,
    ) -> Result<serde_json::Value> {
        if let Some(k) = key {
            // Get specific key (supports dot notation for nested keys)
            let keys: Vec<&str> = k.split('.').collect();

            // Build JSONB path query
            let path_query = if keys.len() == 1 {
                format!("metadata->'{}'", keys[0])
            } else {
                let path_parts: Vec<String> = keys.iter().map(|k| format!("'{}'", k)).collect();
                format!("metadata->{}", path_parts.join("->"))
            };

            let query_str = format!(
                r#"
                SELECT {}
                FROM worktrees
                WHERE id = $1
                "#,
                path_query
            );

            let result: Option<serde_json::Value> = sqlx::query_scalar(&query_str)
                .bind(worktree_id)
                .fetch_optional(&self.pool)
                .await
                .context("Failed to get worktree metadata")?;

            match result {
                Some(val) if !val.is_null() => Ok(val),
                _ => Err(anyhow::anyhow!("Metadata key '{}' not found", k)),
            }
        } else {
            // Get entire metadata object
            let result: Option<serde_json::Value> = sqlx::query_scalar(
                r#"
                SELECT COALESCE(metadata, '{}'::jsonb)
                FROM worktrees
                WHERE id = $1
                "#
            )
            .bind(worktree_id)
            .fetch_optional(&self.pool)
            .await
            .context("Failed to get worktree metadata")?;

            Ok(result.unwrap_or(serde_json::Value::Object(serde_json::Map::new())))
        }
    }
}
