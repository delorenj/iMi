use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{migrate::MigrateDatabase, sqlite::SqlitePool, Sqlite, Row};
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Database {
    pool: SqlitePool,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Worktree {
    pub id: String,
    pub repo_name: String,
    pub worktree_name: String,
    pub branch_name: String,
    pub worktree_type: String, // feat, pr, fix, aiops, devops, trunk
    pub path: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub active: bool,
    pub agent_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AgentActivity {
    pub id: String,
    pub agent_id: String,
    pub worktree_id: String,
    pub activity_type: String, // created, modified, deleted, committed, pushed
    pub file_path: Option<String>,
    pub description: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Repository {
    pub id: String,
    pub name: String,
    pub path: String,
    pub remote_url: String,
    pub default_branch: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub active: bool,
}

impl Database {
    pub async fn new<P: AsRef<Path>>(database_path: P) -> Result<Self> {
        let database_url = format!("sqlite:{}", database_path.as_ref().display());
        
        // Create database if it doesn't exist
        if !Sqlite::database_exists(&database_url).await.unwrap_or(false) {
            Sqlite::create_database(&database_url).await
                .context("Failed to create database")?;
        }
        
        let pool = SqlitePool::connect(&database_url).await
            .context("Failed to connect to database")?;
        
        let db = Self { pool };
        db.run_migrations().await?;
        
        Ok(db)
    }
    
    async fn run_migrations(&self) -> Result<()> {
        // Create repositories table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS repositories (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                path TEXT NOT NULL,
                remote_url TEXT NOT NULL,
                default_branch TEXT NOT NULL DEFAULT 'main',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                active BOOLEAN NOT NULL DEFAULT TRUE
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .context("Failed to create repositories table")?;
        
        // Create worktrees table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS worktrees (
                id TEXT PRIMARY KEY,
                repo_name TEXT NOT NULL,
                worktree_name TEXT NOT NULL,
                branch_name TEXT NOT NULL,
                worktree_type TEXT NOT NULL,
                path TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                active BOOLEAN NOT NULL DEFAULT TRUE,
                agent_id TEXT,
                FOREIGN KEY (repo_name) REFERENCES repositories (name),
                UNIQUE(repo_name, worktree_name)
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .context("Failed to create worktrees table")?;
        
        // Create agent_activities table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS agent_activities (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                worktree_id TEXT NOT NULL,
                activity_type TEXT NOT NULL,
                file_path TEXT,
                description TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (worktree_id) REFERENCES worktrees (id)
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .context("Failed to create agent_activities table")?;
        
        // Create indexes for performance
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_worktrees_repo_name ON worktrees (repo_name)")
            .execute(&self.pool)
            .await?;
            
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_worktrees_active ON worktrees (active)")
            .execute(&self.pool)
            .await?;
            
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_agent_activities_worktree_id ON agent_activities (worktree_id)")
            .execute(&self.pool)
            .await?;
        
        Ok(())
    }
    
    // Repository operations
    pub async fn create_repository(
        &self,
        name: &str,
        path: &str,
        remote_url: &str,
        default_branch: &str,
    ) -> Result<Repository> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        
        let repo = Repository {
            id: id.clone(),
            name: name.to_string(),
            path: path.to_string(),
            remote_url: remote_url.to_string(),
            default_branch: default_branch.to_string(),
            created_at: now,
            updated_at: now,
            active: true,
        };
        
        sqlx::query(
            r#"
            INSERT INTO repositories (id, name, path, remote_url, default_branch, created_at, updated_at, active)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&repo.id)
        .bind(&repo.name)
        .bind(&repo.path)
        .bind(&repo.remote_url)
        .bind(&repo.default_branch)
        .bind(repo.created_at.to_rfc3339())
        .bind(repo.updated_at.to_rfc3339())
        .bind(repo.active)
        .execute(&self.pool)
        .await
        .context("Failed to insert repository")?;
        
        Ok(repo)
    }
    
    pub async fn get_repository(&self, name: &str) -> Result<Option<Repository>> {
        let row = sqlx::query("SELECT * FROM repositories WHERE name = ? AND active = TRUE")
            .bind(name)
            .fetch_optional(&self.pool)
            .await
            .context("Failed to fetch repository")?;
        
        if let Some(row) = row {
            Ok(Some(Repository {
                id: row.get("id"),
                name: row.get("name"),
                path: row.get("path"),
                remote_url: row.get("remote_url"),
                default_branch: row.get("default_branch"),
                created_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at"))?
                    .with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("updated_at"))?
                    .with_timezone(&Utc),
                active: row.get("active"),
            }))
        } else {
            Ok(None)
        }
    }
    
    // Worktree operations
    pub async fn create_worktree(
        &self,
        repo_name: &str,
        worktree_name: &str,
        branch_name: &str,
        worktree_type: &str,
        path: &str,
        agent_id: Option<&str>,
    ) -> Result<Worktree> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        
        let worktree = Worktree {
            id: id.clone(),
            repo_name: repo_name.to_string(),
            worktree_name: worktree_name.to_string(),
            branch_name: branch_name.to_string(),
            worktree_type: worktree_type.to_string(),
            path: path.to_string(),
            created_at: now,
            updated_at: now,
            active: true,
            agent_id: agent_id.map(|s| s.to_string()),
        };
        
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO worktrees 
            (id, repo_name, worktree_name, branch_name, worktree_type, path, created_at, updated_at, active, agent_id)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&worktree.id)
        .bind(&worktree.repo_name)
        .bind(&worktree.worktree_name)
        .bind(&worktree.branch_name)
        .bind(&worktree.worktree_type)
        .bind(&worktree.path)
        .bind(worktree.created_at.to_rfc3339())
        .bind(worktree.updated_at.to_rfc3339())
        .bind(worktree.active)
        .bind(&worktree.agent_id)
        .execute(&self.pool)
        .await
        .context("Failed to insert worktree")?;
        
        Ok(worktree)
    }
    
    pub async fn get_worktree(&self, repo_name: &str, worktree_name: &str) -> Result<Option<Worktree>> {
        let row = sqlx::query(
            "SELECT * FROM worktrees WHERE repo_name = ? AND worktree_name = ? AND active = TRUE"
        )
        .bind(repo_name)
        .bind(worktree_name)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch worktree")?;
        
        if let Some(row) = row {
            Ok(Some(Worktree {
                id: row.get("id"),
                repo_name: row.get("repo_name"),
                worktree_name: row.get("worktree_name"),
                branch_name: row.get("branch_name"),
                worktree_type: row.get("worktree_type"),
                path: row.get("path"),
                created_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at"))?
                    .with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("updated_at"))?
                    .with_timezone(&Utc),
                active: row.get("active"),
                agent_id: row.get("agent_id"),
            }))
        } else {
            Ok(None)
        }
    }
    
    pub async fn list_worktrees(&self, repo_name: Option<&str>) -> Result<Vec<Worktree>> {
        let query = if let Some(repo) = repo_name {
            sqlx::query("SELECT * FROM worktrees WHERE repo_name = ? AND active = TRUE ORDER BY created_at DESC")
                .bind(repo)
        } else {
            sqlx::query("SELECT * FROM worktrees WHERE active = TRUE ORDER BY created_at DESC")
        };
        
        let rows = query.fetch_all(&self.pool).await
            .context("Failed to fetch worktrees")?;
        
        let mut worktrees = Vec::new();
        for row in rows {
            worktrees.push(Worktree {
                id: row.get("id"),
                repo_name: row.get("repo_name"),
                worktree_name: row.get("worktree_name"),
                branch_name: row.get("branch_name"),
                worktree_type: row.get("worktree_type"),
                path: row.get("path"),
                created_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at"))?
                    .with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("updated_at"))?
                    .with_timezone(&Utc),
                active: row.get("active"),
                agent_id: row.get("agent_id"),
            });
        }
        
        Ok(worktrees)
    }
    
    pub async fn deactivate_worktree(&self, repo_name: &str, worktree_name: &str) -> Result<()> {
        sqlx::query(
            "UPDATE worktrees SET active = FALSE, updated_at = ? WHERE repo_name = ? AND worktree_name = ?"
        )
        .bind(Utc::now().to_rfc3339())
        .bind(repo_name)
        .bind(worktree_name)
        .execute(&self.pool)
        .await
        .context("Failed to deactivate worktree")?;
        
        Ok(())
    }
    
    // Agent activity operations
    pub async fn log_agent_activity(
        &self,
        agent_id: &str,
        worktree_id: &str,
        activity_type: &str,
        file_path: Option<&str>,
        description: &str,
    ) -> Result<AgentActivity> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        
        let activity = AgentActivity {
            id: id.clone(),
            agent_id: agent_id.to_string(),
            worktree_id: worktree_id.to_string(),
            activity_type: activity_type.to_string(),
            file_path: file_path.map(|s| s.to_string()),
            description: description.to_string(),
            created_at: now,
        };
        
        sqlx::query(
            r#"
            INSERT INTO agent_activities (id, agent_id, worktree_id, activity_type, file_path, description, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&activity.id)
        .bind(&activity.agent_id)
        .bind(&activity.worktree_id)
        .bind(&activity.activity_type)
        .bind(&activity.file_path)
        .bind(&activity.description)
        .bind(activity.created_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .context("Failed to insert agent activity")?;
        
        Ok(activity)
    }
    
    pub async fn get_recent_activities(&self, worktree_id: Option<&str>, limit: i64) -> Result<Vec<AgentActivity>> {
        let query = if let Some(wt_id) = worktree_id {
            sqlx::query(
                "SELECT * FROM agent_activities WHERE worktree_id = ? ORDER BY created_at DESC LIMIT ?"
            ).bind(wt_id)
        } else {
            sqlx::query("SELECT * FROM agent_activities ORDER BY created_at DESC LIMIT ?")
        };
        
        let rows = query.bind(limit).fetch_all(&self.pool).await
            .context("Failed to fetch agent activities")?;
        
        let mut activities = Vec::new();
        for row in rows {
            activities.push(AgentActivity {
                id: row.get("id"),
                agent_id: row.get("agent_id"),
                worktree_id: row.get("worktree_id"),
                activity_type: row.get("activity_type"),
                file_path: row.get("file_path"),
                description: row.get("description"),
                created_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at"))?
                    .with_timezone(&Utc),
            });
        }
        
        Ok(activities)
    }
}