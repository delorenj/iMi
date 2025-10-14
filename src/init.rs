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

#[derive(Clone)]
pub struct InitCommand {
    pub force: bool,
    config: Config,
    db: Database,
}

impl InitCommand {
    pub fn new(force: bool, config: Config, db: Database) -> Self {
        Self { force, config, db }
    }

    pub async fn execute(&self, path: Option<&Path>) -> Result<InitResult> {
        let git_manager = GitManager::new();
        let current_dir = match path {
            Some(p) => p.to_path_buf(),
            None => env::current_dir().context("Failed to get current directory")?,
        };

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
            self.config
                .save_to(&config_path)
                .await
                .context("Failed to save default configuration")?;
            println!(
                "{} Created default configuration at {}",
                "✅".bright_green(),
                config_path.display()
            );
        } else {
            println!(
                "{} Configuration already exists at {}. Use --force to overwrite.",
                "ℹ️".bright_yellow(),
                config_path.display()
            );
        }

        let db_path = &self.config.database_path;
        if !db_path.exists() || self.force {
            self.db
                .ensure_tables()
                .await
                .context("Failed to create database tables")?;
            println!(
                "{} Created database at {}",
                "✅".bright_green(),
                db_path.display()
            );
        } else {
            println!(
                "{} Database already exists at {}. Use --force to overwrite.",
                "ℹ️".bright_yellow(),
                db_path.display()
            );
        }

        Ok(InitResult::success(
            "Global iMi configuration setup complete.".to_string(),
        ))
    }

    async fn handle_inside_repo(&self, current_dir: &Path) -> Result<InitResult> {
        println!(
            "{} Running inside a git repository. Initializing...",
            "🚀".bright_cyan()
        );

        let (_imi_path, repo_name) = self.detect_paths(current_dir)?;
        let repo_path = GitManager::new().find_repository(Some(current_dir))?.path().parent().unwrap().to_path_buf();

        if let Some(existing_repo) = self.db.get_repository(&repo_name).await? {
            if !self.force {
                return Ok(InitResult::failure(format!(
                    "Repository '{}' is already registered at {}. Use --force to re-initialize.",
                    repo_name, existing_repo.path
                )));
            }
        }

        let remote_url = GitManager::new()
            .get_remote_url(current_dir)
            .await
            .unwrap_or_default();
        let default_branch = GitManager::new()
            .get_default_branch(current_dir)
            .await
            .unwrap_or_else(|_| "main".to_string());

        self.db.create_repository(
            &repo_name,
            repo_path.to_str().unwrap(),
            &remote_url,
            &default_branch,
        )
        .await?;
        println!(
            "{} Registered repository '{}' in the database.",
            "✅".bright_green(),
            repo_name
        );

        let imi_dir = repo_path.join(".iMi");
        fs::create_dir_all(&imi_dir)
            .await
            .context("Failed to create .iMi directory")?;
        println!(
            "{} Created .iMi directory at {}",
            "✅".bright_green(),
            imi_dir.display()
        );

        Ok(InitResult::success(format!(
            "Successfully initialized iMi for repository '{}'.",
            repo_name
        )))
    }

    fn detect_paths(&self, current_dir: &Path) -> Result<(PathBuf, String)> {
        let git_manager = GitManager::new();
        let repo = git_manager.find_repository(Some(current_dir))?;
        let repo_path = repo
            .workdir()
            .context("Repository has no working directory")?
            .to_path_buf();

        let repo_name = git_manager.get_repository_name(&repo)?;

        // The "imi_path" is the parent of the repository directory.
        let imi_path = repo_path.parent().unwrap_or(&repo_path).to_path_buf();

        Ok((imi_path, repo_name))
    }
}
