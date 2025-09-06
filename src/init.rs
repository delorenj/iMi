use anyhow::{Context, Result};
use colored::*;
use std::env;
use std::path::{Path, PathBuf};
use tokio::fs;

use crate::config::Config;
use crate::database::Database;
use crate::git::GitManager;

#[derive(Debug, Clone)]
pub struct InitResult {
    pub success: bool,
    pub message: String,
}

impl InitResult {
    pub fn success(message: String) -> Self {
        Self {
            success: true,
            message,
        }
    }

    pub fn failure(message: String) -> Self {
        Self {
            success: false,
            message,
        }
    }
}

#[derive(Debug, Clone)]
pub struct InitCommand {
    pub force: bool,
}

impl InitCommand {
    pub fn new(force: bool) -> Self {
        Self { force }
    }

    pub async fn execute(&self) -> Result<InitResult> {
        let git_manager = GitManager::new();
        let current_dir = env::current_dir().context("Failed to get current directory")?;

        if git_manager.is_in_repository(&current_dir) {
            self.handle_inside_repo(&current_dir).await
        } else {
            self.handle_outside_repo().await
        }
    }

    async fn handle_outside_repo(&self) -> Result<InitResult> {
        println!(
            "{} Running outside of a git repository. Setting up global iMi configuration...",
            "🌍".bright_blue()
        );

        let config_path = Config::get_config_path()?;
        if !config_path.exists() || self.force {
            let config = Config::default();
            config.save().await.context("Failed to save default configuration")?;
            println!("{} Created default configuration at {}", "✅".bright_green(), config_path.display());
        } else {
            println!("{} Configuration already exists at {}. Use --force to overwrite.", "ℹ️".bright_yellow(), config_path.display());
        }

        let config = Config::load().await.context("Failed to load configuration")?;
        let db_path = &config.database_path;
        if !db_path.exists() || self.force {
            let db = Database::new(db_path).await.context("Failed to create database")?;
            db.ensure_tables().await.context("Failed to create database tables")?;
            println!("{} Created database at {}", "✅".bright_green(), db_path.display());
        } else {
             println!("{} Database already exists at {}. Use --force to overwrite.", "ℹ️".bright_yellow(), db_path.display());
        }

        Ok(InitResult::success(
            "Global iMi configuration setup complete.".to_string(),
        ))
    }

    async fn handle_inside_repo(&self, current_dir: &Path) -> Result<InitResult> {
        // First, ensure global setup is done.
        self.handle_outside_repo().await?;

        println!("{} Running inside a git repository. Initializing...", "🚀".bright_cyan());

        let (imi_path, repo_name) = self.detect_paths(current_dir)?;

        let config = Config::load().await.context("Failed to load configuration")?;
        let db = Database::new(&config.database_path).await.context("Failed to connect to database")?;

        if let Some(existing_repo) = db.get_repository(&repo_name).await? {
            if !self.force {
                return Ok(InitResult::failure(format!(
                    "Repository '{}' is already registered at {}. Use --force to re-initialize.",
                    repo_name, existing_repo.path
                )));
            }
        }
        
        let remote_url = GitManager::new().get_remote_url(current_dir).await.unwrap_or_default();
        let default_branch = GitManager::new().get_default_branch(current_dir).await.unwrap_or_else(|_| "main".to_string());


        db.create_repository(&repo_name, imi_path.to_str().unwrap(), &remote_url, &default_branch).await?;
        println!("{} Registered repository '{}' in the database.", "✅".bright_green(), repo_name);

        let imi_dir = imi_path.join(".iMi");
        fs::create_dir_all(&imi_dir).await.context("Failed to create .iMi directory")?;
        println!("{} Created .iMi directory at {}", "✅".bright_green(), imi_dir.display());

        Ok(InitResult::success(format!(
            "Successfully initialized iMi for repository '{}'.",
            repo_name
        )))
    }

    fn detect_paths(&self, current_dir: &Path) -> Result<(PathBuf, String)> {
        let current_dir_name = current_dir
            .file_name()
            .and_then(|n| n.to_str())
            .context("Failed to get current directory name")?;

        if current_dir_name.starts_with("trunk-") {
            let imi_path = current_dir.parent().context("Failed to get parent directory")?;
            let repo_name = imi_path
                .file_name()
                .and_then(|n| n.to_str())
                .context("Failed to get repository name")?
                .to_string();
            Ok((imi_path.to_path_buf(), repo_name))
        } else {
             Err(anyhow::anyhow!("Not in a trunk-* directory. Please run `imi init` from a directory like '.../repo-name/trunk-main'"))
        }
    }
}