use anyhow::{Context, Result};
use dirs;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub database_path: PathBuf,
    pub root_path: PathBuf,
    pub sync_settings: SyncSettings,
    pub git_settings: GitSettings,
    pub monitoring_settings: MonitoringSettings,
    pub symlink_files: Vec<String>,
    #[serde(skip)]
    pub repo_path: Option<PathBuf>,
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
        let config_dir = home_dir.join(".config").join("iMi");

        Self {
            database_path: config_dir.join("iMi.db"),
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
            repo_path: None,
        }
    }
}

impl Config {
    pub async fn load() -> Result<Self> {
        let global_config_path = Self::get_global_config_path()?;
        let mut config = Self::load_from(&global_config_path).await?;

        if let Some(project_root) = Self::find_project_root()? {
            let project_config_path = project_root.join(".iMi").join("config.toml");
            if project_config_path.exists() {
                let project_config = Self::load_from(&project_config_path).await?;
                // Simple merge: project config overrides global
                config.database_path = project_config.database_path;
                config.root_path = project_config.root_path;
                config.sync_settings = project_config.sync_settings;
                config.git_settings = project_config.git_settings;
                config.monitoring_settings = project_config.monitoring_settings;
                config.symlink_files = project_config.symlink_files;
            }
            config.repo_path = Some(project_root);
        }

        Ok(config)
    }

    pub fn find_project_root() -> Result<Option<PathBuf>> {
        let current_dir = env::current_dir().context("Failed to get current directory")?;
        let mut current = current_dir.as_path();

        loop {
            if current.join(".iMi").is_dir() {
                return Ok(Some(current.to_path_buf()));
            }

            match current.parent() {
                Some(parent) => current = parent,
                None => return Ok(None),
            }
        }
    }


    pub async fn load_from(path: &std::path::Path) -> Result<Self> {
        if path.exists() {
            let contents = fs::read_to_string(path)
                .await
                .context(format!("Failed to read config file at {:?}", path))?;

            let config: Config =
                toml::from_str(&contents).context("Failed to parse config file")?;

            Ok(config)
        } else {
            let config = Self::default();
            if path == Self::get_global_config_path()? {
                 config.save_to(path).await?;
            }
            Ok(config)
        }
    }

    pub async fn save(&self) -> Result<()> {
        let config_path = Self::get_global_config_path()?;
        self.save_to(&config_path).await
    }

    pub async fn save_to(&self, path: &std::path::Path) -> Result<()> {
        // Ensure config directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .context("Failed to create config directory")?;
        }

        let contents = toml::to_string_pretty(self).context("Failed to serialize config")?;

        fs::write(path, contents)
            .await
            .context("Failed to write config file")?;

        Ok(())
    }

    pub fn get_global_config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("Could not find config directory")?
            .join("iMi");

        Ok(config_dir.join("config.toml"))
    }

    pub fn get_repo_path(&self, repo_name: &str) -> PathBuf {
        self.root_path.join(repo_name)
    }

    pub fn get_trunk_path(&self, repo_name: &str) -> PathBuf {
        let main_branch = &self.git_settings.default_branch;
        self.get_repo_path(repo_name)
            .join(format!("trunk-{}", main_branch))
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

    #[allow(dead_code)]
    pub async fn ensure_database_directory(&self) -> Result<()> {
        if let Some(parent) = self.database_path.parent() {
            fs::create_dir_all(parent)
                .await
                .context("Failed to create database directory")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

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

    #[tokio::test]
    async fn test_find_project_root() {
        let dir = tempdir().unwrap();
        let project_root = dir.path().join("my-project");
        let imi_dir = project_root.join(".iMi");
        let sub_dir = project_root.join("sub");
        std::fs::create_dir_all(&imi_dir).unwrap();
        std::fs::create_dir_all(&sub_dir).unwrap();

        env::set_current_dir(&sub_dir).unwrap();

        let found_root = Config::find_project_root().unwrap();
        assert_eq!(found_root, Some(project_root));

        env::set_current_dir(dir.path()).unwrap();
        let not_found_root = Config::find_project_root().unwrap();
        assert_eq!(not_found_root, None);
    }
}
