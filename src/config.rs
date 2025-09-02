use anyhow::{Context, Result};
use dirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub database_path: PathBuf,
    pub root_path: PathBuf,
    pub sync_settings: SyncSettings,
    pub git_settings: GitSettings,
    pub monitoring_settings: MonitoringSettings,
    pub symlink_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncSettings {
    pub enabled: bool,
    pub global_sync_path: PathBuf,
    pub repo_sync_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitSettings {
    pub default_branch: String,
    pub remote_name: String,
    pub auto_fetch: bool,
    pub prune_on_fetch: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringSettings {
    pub enabled: bool,
    pub refresh_interval_ms: u64,
    pub watch_file_changes: bool,
    pub track_agent_activity: bool,
}

impl Default for Config {
    fn default() -> Self {
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let config_dir = home_dir.join(".config").join("imi");
        
        Self {
            database_path: config_dir.join("imi.db"),
            root_path: home_dir.join("code"),
            sync_settings: SyncSettings {
                enabled: true,
                global_sync_path: PathBuf::from("sync/global"),
                repo_sync_path: PathBuf::from("sync/repo"),
            },
            git_settings: GitSettings {
                default_branch: "main".to_string(),
                remote_name: "origin".to_string(),
                auto_fetch: true,
                prune_on_fetch: true,
            },
            monitoring_settings: MonitoringSettings {
                enabled: true,
                refresh_interval_ms: 1000,
                watch_file_changes: true,
                track_agent_activity: true,
            },
            symlink_files: vec![
                ".env".to_string(),
                ".jarad-config".to_string(),
                ".vscode/settings.json".to_string(),
                ".gitignore.local".to_string(),
            ],
        }
    }
}

impl Config {
    pub async fn load() -> Result<Self> {
        let config_path = Self::get_config_path()?;
        
        if config_path.exists() {
            let contents = fs::read_to_string(&config_path)
                .await
                .context("Failed to read config file")?;
            
            let config: Config = toml::from_str(&contents)
                .context("Failed to parse config file")?;
            
            Ok(config)
        } else {
            let config = Self::default();
            config.save().await?;
            Ok(config)
        }
    }
    
    pub async fn save(&self) -> Result<()> {
        let config_path = Self::get_config_path()?;
        
        // Ensure config directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).await
                .context("Failed to create config directory")?;
        }
        
        let contents = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;
            
        fs::write(&config_path, contents).await
            .context("Failed to write config file")?;
        
        Ok(())
    }
    
    fn get_config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("Could not find config directory")?
            .join("imi");
        
        Ok(config_dir.join("config.toml"))
    }
    
    pub fn get_repo_path(&self, repo_name: &str) -> PathBuf {
        self.root_path.join(repo_name)
    }
    
    pub fn get_trunk_path(&self, repo_name: &str) -> PathBuf {
        let main_branch = &self.git_settings.default_branch;
        self.get_repo_path(repo_name).join(format!("trunk-{}", main_branch))
    }
    
    pub fn get_worktree_path(&self, repo_name: &str, worktree_name: &str) -> PathBuf {
        self.get_repo_path(repo_name).join(worktree_name)
    }
    
    pub fn get_sync_path(&self, repo_name: &str, is_global: bool) -> PathBuf {
        let repo_path = self.get_repo_path(repo_name);
        
        if is_global {
            repo_path.join(&self.sync_settings.global_sync_path)
        } else {
            repo_path.join(&self.sync_settings.repo_sync_path)
        }
    }
    
    pub async fn ensure_database_directory(&self) -> Result<()> {
        if let Some(parent) = self.database_path.parent() {
            fs::create_dir_all(parent).await
                .context("Failed to create database directory")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.git_settings.default_branch, "main");
        assert!(config.monitoring_settings.enabled);
        assert!(config.sync_settings.enabled);
    }
    
    #[tokio::test]
    async fn test_config_paths() {
        let config = Config::default();
        let repo_name = "test-repo";
        
        let repo_path = config.get_repo_path(repo_name);
        assert!(repo_path.to_string_lossy().contains("test-repo"));
        
        let trunk_path = config.get_trunk_path(repo_name);
        assert!(trunk_path.to_string_lossy().contains("trunk-main"));
        
        let worktree_path = config.get_worktree_path(repo_name, "feat-test");
        assert!(worktree_path.to_string_lossy().contains("feat-test"));
    }
}