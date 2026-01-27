use anyhow::{anyhow, Context, Result};
use colored::*;
use dialoguer::{Confirm, Select};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::{Path, PathBuf};
use tokio::fs;
use uuid::Uuid;

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

/// Project metadata written to .iMi/project.json
/// Provides fast filesystem access for shell integrations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetadata {
    pub project_id: Uuid,
    pub name: String,
    pub remote_origin: String,
    pub default_branch: String,
    pub trunk_path: String,
    pub description: Option<String>,
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
            // Outside any repo - check if we should show TUI selector
            if path.is_none() {
                // User didn't specify a path, so check if we have registered repos
                let repos = self.db.list_repositories().await?;

                if !repos.is_empty() && !self.force {
                    // Show TUI selector
                    println!();
                    println!(
                        "{} {}",
                        "📦".bright_cyan(),
                        "Available Repositories:".bright_cyan().bold()
                    );
                    println!();

                    let repo_names: Vec<String> = repos
                        .iter()
                        .map(|r| format!("{} ({})", r.name.bright_green(), r.path.dimmed()))
                        .collect();

                    let selection = Select::new()
                        .with_prompt("Select a repository to initialize")
                        .items(&repo_names)
                        .default(0)
                        .interact_opt()?;

                    if let Some(idx) = selection {
                        let selected_repo = &repos[idx];
                        let repo_path = PathBuf::from(&selected_repo.path);

                        // Change to that directory and initialize
                        return self.handle_inside_repo(&repo_path).await;
                    } else {
                        return Ok(InitResult::failure(
                            "Repository selection cancelled.".to_string(),
                        ));
                    }
                }
            }

            self.handle_outside_repo().await
        }
    }

    async fn handle_outside_repo(&self) -> Result<InitResult> {
        let config_path = Config::get_global_config_path()?;
        let db_path = &self.config.database_path;

        let config_exists = config_path.exists();
        let db_exists = db_path.exists();

        // Only show the "Running outside" message if we're actually creating something new
        let needs_setup = !config_exists || !db_exists || self.force;

        if needs_setup {
            println!(
                "{} Running outside of a git repository. Setting up global iMi configuration...",
                "🌍".bright_blue()
            );
        }

        if !config_exists || self.force {
            self.config
                .save_to(&config_path)
                .await
                .context("Failed to save default configuration")?;
            println!(
                "{} Created default configuration at {}",
                "✅".bright_green(),
                config_path.display()
            );
        } else if !needs_setup {
            // Silently skip - config already exists and we're not forcing
        } else {
            println!(
                "{} Configuration already exists at {}. Use --force to overwrite.",
                "ℹ️".bright_yellow(),
                config_path.display()
            );
        }

        if !db_exists || self.force {
            self.db
                .ensure_tables()
                .await
                .context("Failed to create database tables")?;
            println!(
                "{} Created database at {}",
                "✅".bright_green(),
                db_path.display()
            );
        } else if !needs_setup {
            // Silently skip - database already exists and we're not forcing
        } else {
            println!(
                "{} Database already exists at {}. Use --force to overwrite.",
                "ℹ️".bright_yellow(),
                db_path.display()
            );
        }

        if needs_setup {
            Ok(InitResult::success(
                "Global iMi configuration setup complete.".to_string(),
            ))
        } else {
            // Silent success - everything already exists
            Ok(InitResult::success(
                "iMi is already configured.".to_string(),
            ))
        }
    }

    async fn handle_inside_repo(&self, current_dir: &Path) -> Result<InitResult> {
        println!(
            "{} Running inside a git repository. Initializing...",
            "🚀".bright_cyan()
        );

        let git_manager = GitManager::new();
        let repo = git_manager.find_repository(Some(current_dir))?;
        let repo_path = repo
            .workdir()
            .context("Repository has no working directory")?
            .to_path_buf();
        let repo_name = git_manager.get_repository_name(&repo)?;

        // Check if we're in a trunk-* directory (proper structure)
        let dir_name = repo_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        let is_trunk = dir_name.starts_with("trunk-");

        // If not in trunk directory, offer automated restructuring
        if !is_trunk && !self.force {
            println!();
            println!(
                "{} {}",
                "⚠️".bright_yellow(),
                "Current directory:".bright_yellow()
            );
            println!("   {}", repo_path.display().to_string().bright_white());
            println!();
            println!("{}", "iMi works best with this structure:".bright_cyan());

            // Determine parent directory and trunk path
            let parent = repo_path
                .parent()
                .context("Cannot determine parent directory")?;
            let repo_container = parent.join(&repo_name);
            let trunk_path = repo_container.join("trunk-main");

            println!(
                "   {}/",
                repo_container.display().to_string().bright_white()
            );
            println!(
                "     ├── {}/ {}",
                "trunk-main".bright_green(),
                "(your main branch)".dimmed()
            );
            println!(
                "     ├── {}/ {}",
                "feat-feature1".bright_blue(),
                "(feature worktrees)".dimmed()
            );
            println!(
                "     └── {}/ {}",
                "fix-bugfix".bright_red(),
                "(fix worktrees)".dimmed()
            );
            println!();

            // Check if target structure already exists
            if repo_container.exists() && repo_container != repo_path {
                return Err(anyhow!(
                    "Target directory already exists: {}\nPlease manually resolve the conflict.",
                    repo_container.display()
                ));
            }

            println!("{}", "This will:".bright_cyan());
            println!(
                "  1. Create parent directory: {}",
                repo_container.display().to_string().bright_white()
            );
            println!(
                "  2. Move current repo to: {}",
                trunk_path.display().to_string().bright_green()
            );
            println!("  3. Register with iMi");
            println!();

            let should_restructure = Confirm::new()
                .with_prompt("Would you like to restructure automatically?")
                .default(false)
                .interact()?;

            if !should_restructure {
                return Ok(InitResult::failure(
                    "Initialization cancelled. Run 'iMi init' again after manual restructuring."
                        .to_string(),
                ));
            }

            // Perform the restructuring
            println!();
            println!("{} Restructuring directory...", "🔄".bright_cyan());

            // Create the rollback point
            let temp_backup = std::env::temp_dir().join(format!("imi_backup_{}", repo_name));

            // Execute restructuring with rollback capability
            match self
                .restructure_directory(&repo_path, &repo_container, &trunk_path, &temp_backup)
                .await
            {
                Ok(_) => {
                    println!(
                        "{} Directory restructured successfully",
                        "✅".bright_green()
                    );

                    // Clean up backup
                    if temp_backup.exists() {
                        let _ = fs::remove_dir_all(&temp_backup).await;
                    }

                    // Update current_dir for registration
                    let new_repo_path = trunk_path;
                    return self.register_repository(&new_repo_path, &repo_name).await;
                }
                Err(e) => {
                    println!("{} Restructuring failed: {}", "❌".bright_red(), e);

                    // Attempt rollback
                    if temp_backup.exists() {
                        println!("{} Attempting rollback...", "🔄".bright_yellow());
                        if let Err(rollback_err) =
                            self.rollback_restructure(&temp_backup, &repo_path).await
                        {
                            println!("{} Rollback failed: {}", "❌".bright_red(), rollback_err);
                            println!(
                                "{} Manual intervention required. Backup at: {}",
                                "⚠️".bright_yellow(),
                                temp_backup.display()
                            );
                        } else {
                            println!("{} Rollback successful", "✅".bright_green());
                            let _ = fs::remove_dir_all(&temp_backup).await;
                        }
                    }

                    return Err(e);
                }
            }
        }

        // Standard registration for trunk-* directories or forced init
        self.register_repository(&repo_path, &repo_name).await
    }

    async fn restructure_directory(
        &self,
        source: &Path,
        container: &Path,
        trunk_path: &Path,
        backup: &Path,
    ) -> Result<()> {
        // Step 1: Create backup
        fs::create_dir_all(backup.parent().unwrap()).await?;

        // Copy source to backup (using tokio::fs for async operations)
        self.copy_dir_recursive(source, backup).await?;

        // Step 2: Create container directory
        fs::create_dir_all(container)
            .await
            .context("Failed to create container directory")?;

        // Step 3: Move source to trunk_path inside container
        // We need to use std::fs::rename for atomic move
        std::fs::rename(source, trunk_path)
            .context("Failed to move repository to trunk directory")?;

        Ok(())
    }

    fn copy_dir_recursive<'a>(
        &'a self,
        src: &'a Path,
        dst: &'a Path,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            fs::create_dir_all(dst).await?;

            let mut entries = fs::read_dir(src).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                let file_name = entry.file_name();
                let dst_path = dst.join(&file_name);

                if path.is_dir() {
                    self.copy_dir_recursive(&path, &dst_path).await?;
                } else {
                    fs::copy(&path, &dst_path).await?;
                }
            }

            Ok(())
        })
    }

    async fn rollback_restructure(&self, backup: &Path, original: &Path) -> Result<()> {
        // Remove any partially created structure
        if let Some(parent) = original.parent() {
            // Only remove if it was newly created (empty or only contains our failed attempt)
            if parent.exists() {
                let mut entries = fs::read_dir(parent).await?;
                let mut count = 0;
                while let Some(_) = entries.next_entry().await? {
                    count += 1;
                    if count > 1 {
                        break;
                    }
                }

                // If parent only has one entry (our failed attempt), safe to remove
                if count <= 1 {
                    fs::remove_dir_all(parent).await?;
                }
            }
        }

        // Restore from backup
        self.copy_dir_recursive(backup, original).await?;

        Ok(())
    }

    async fn register_repository(&self, repo_path: &Path, repo_name: &str) -> Result<InitResult> {
        let git_manager = GitManager::new();

        if let Some(existing_repo) = self.db.get_repository(repo_name).await? {
            if !self.force {
                return Ok(InitResult::failure(format!(
                    "Repository '{}' is already registered at {}. Use --force to re-initialize.",
                    repo_name, existing_repo.path
                )));
            }
        }

        let remote_url = git_manager
            .get_remote_url(repo_path)
            .await
            .unwrap_or_default();
        let default_branch = git_manager
            .get_default_branch(repo_path)
            .await
            .unwrap_or_else(|_| "main".to_string());

        let project = self.db
            .create_repository(
                repo_name,
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
        println!(
            "   {} Project ID: {}",
            "🔑".bright_black(),
            project.id.to_string().bright_cyan()
        );

        let imi_dir = repo_path.parent().unwrap().join(".iMi");
        fs::create_dir_all(&imi_dir)
            .await
            .context("Failed to create .iMi directory")?;
        println!(
            "{} Created .iMi directory at {}",
            "✅".bright_green(),
            imi_dir.display()
        );

        // Write project metadata to .iMi/project.json for fast filesystem access
        let project_metadata = ProjectMetadata {
            project_id: project.id,
            name: project.name.clone(),
            remote_origin: project.remote_url.clone(),
            default_branch: project.default_branch.clone(),
            trunk_path: project.path.clone(),
            description: project.description.clone(),
        };
        let project_json_path = imi_dir.join("project.json");
        let json_content = serde_json::to_string_pretty(&project_metadata)
            .context("Failed to serialize project metadata")?;
        fs::write(&project_json_path, json_content)
            .await
            .context("Failed to write project.json")?;
        println!(
            "{} Created project.json with UUID {}",
            "✅".bright_green(),
            project.id.to_string().bright_cyan()
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

    /// Clone a repository from GitHub and set up iMi structure
    pub async fn clone_from_github(&self, github_repo: &str) -> Result<InitResult> {
        println!(
            "{} Cloning {} from GitHub...",
            "🔍".bright_cyan(),
            github_repo.bright_white()
        );

        // Extract repo name from owner/repo format
        let parts: Vec<&str> = github_repo.split('/').collect();
        if parts.len() != 2 {
            return Err(anyhow!(
                "Invalid GitHub repository format. Expected: owner/repo"
            ));
        }

        let repo_name = parts[1];

        // Determine clone location
        let home_dir = dirs::home_dir().context("Could not determine home directory")?;
        let code_dir = home_dir.join("code");
        fs::create_dir_all(&code_dir)
            .await
            .context("Failed to create code directory")?;

        let repo_container = code_dir.join(repo_name);
        let trunk_path = repo_container.join("trunk-main");

        // Check if already exists
        if trunk_path.exists() {
            return Err(anyhow!(
                "Repository already exists at: {}\nUse 'iMi init {}' to initialize it.",
                trunk_path.display(),
                trunk_path.display()
            ));
        }

        // Create container directory
        fs::create_dir_all(&repo_container)
            .await
            .context("Failed to create repository container")?;

        // Clone the repository using git command
        let git_url = format!("https://github.com/{}.git", github_repo);

        println!(
            "{} Cloning into {}...",
            "📁".bright_blue(),
            trunk_path.display().to_string().bright_white()
        );

        let output = tokio::process::Command::new("git")
            .args(&["clone", &git_url, trunk_path.to_str().unwrap()])
            .output()
            .await
            .context("Failed to execute git clone")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!(
                "Git clone failed: {}\n\nThis might be a private repository. Try:\n  1. Check that {} exists on GitHub\n  2. Ensure you're authenticated (gh auth login or SSH keys)",
                stderr,
                github_repo
            ));
        }

        println!("{} Clone complete!", "✅".bright_green());

        // Now initialize iMi in the cloned repository
        self.register_repository(&trunk_path, repo_name).await
    }
}
