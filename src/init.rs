use anyhow::{Context, Result};
use colored::*;
use std::env;
use std::path::PathBuf;

use crate::config::Config;
use crate::database::Database;

/// Validation result for init command
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    pub fn new() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
        self.is_valid = false;
    }

    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    pub fn is_valid(&self) -> bool {
        self.is_valid
    }
}

/// Result type for init operations
#[derive(Debug, Clone)]
pub struct InitResult {
    pub success: bool,
    pub message: String,
    pub config_path: Option<PathBuf>,
    pub database_path: Option<PathBuf>,
    pub repo_name: Option<String>,
    pub repo_path: Option<PathBuf>,
}

impl InitResult {
    pub fn success(message: String) -> Self {
        Self {
            success: true,
            message,
            config_path: None,
            database_path: None,
            repo_name: None,
            repo_path: None,
        }
    }

    pub fn failure(message: String) -> Self {
        Self {
            success: false,
            message,
            config_path: None,
            database_path: None,
            repo_name: None,
            repo_path: None,
        }
    }
}

/// Main InitCommand implementation following TDD patterns
#[derive(Debug, Clone)]
pub struct InitCommand {
    pub force: bool,
}

impl InitCommand {
    pub fn new(force: bool) -> Self {
        Self { force }
    }

    /// Execute the init command with comprehensive validation and setup
    pub async fn execute(&self) -> Result<InitResult> {
        println!(
            "{} Initializing iMi for current repository...",
            "🔧".bright_cyan()
        );

        // Step 1: Validate current environment
        let validation = self.validate_environment().await?;
        if !validation.is_valid() {
            return Ok(InitResult::failure(format!(
                "Validation failed: {}",
                validation.errors.join(", ")
            )));
        }

        // Display warnings if any
        for warning in &validation.warnings {
            println!("{} Warning: {}", "⚠️".bright_yellow(), warning);
        }

        // Step 2: Detect directory structure and repository info
        let (repo_path, repo_name) = self.detect_repository_structure().await?;

        // Step 3: Handle configuration
        let config = self.setup_configuration().await?;

        // Step 4: Initialize database
        let database = self.initialize_database(&config).await?;

        // Step 5: Register repository if needed
        self.register_repository(&database, &repo_name, &repo_path).await?;

        // Step 6: Register trunk worktree if applicable
        self.register_trunk_worktree(&database, &repo_name).await?;

        // Step 7: Display success information
        self.display_success_info(&config, &repo_name, &repo_path).await?;

        Ok(InitResult::success("iMi initialization complete!".to_string()))
    }

    /// Validate the current environment for init requirements
    async fn validate_environment(&self) -> Result<ValidationResult> {
        let mut result = ValidationResult::new();

        // Check if current directory exists and is accessible
        let current_dir = env::current_dir().context("Failed to get current directory");
        match current_dir {
            Ok(_) => {},
            Err(e) => {
                result.add_error(format!("Cannot access current directory: {}", e));
                return Ok(result);
            }
        }

        // Additional validation can be added here
        // For example: Check for git repository, check permissions, etc.

        Ok(result)
    }

    /// Detect repository structure and extract repo information
    async fn detect_repository_structure(&self) -> Result<(PathBuf, String)> {
        let current_dir = env::current_dir().context("Failed to get current directory")?;
        let current_dir = current_dir.canonicalize().unwrap_or(current_dir);

        let current_dir_name = current_dir
            .file_name()
            .and_then(|n| n.to_str())
            .context("Failed to get current directory name")?;

        let (repo_path, repo_name) = if current_dir_name.starts_with("trunk-") {
            // We're in a trunk directory, so the parent is the repository
            let repo_dir = current_dir
                .parent()
                .context("Failed to get parent directory")?;
            let repo_name = repo_dir
                .file_name()
                .and_then(|n| n.to_str())
                .context("Failed to get repository name")?
                .to_string();

            println!(
                "{} Detected trunk directory: {}",
                "🔍".bright_yellow(),
                current_dir_name.bright_green()
            );
            println!(
                "{} Repository: {}",
                "📁".bright_blue(),
                repo_name.bright_cyan()
            );
            println!(
                "{} Repository path: {}",
                "📦".bright_blue(),
                repo_dir.display()
            );
            (repo_dir.to_path_buf(), repo_name)
        } else {
            // We're at the repo root
            let repo_name = current_dir
                .file_name()
                .and_then(|n| n.to_str())
                .context("Failed to get repository name")?
                .to_string();

            println!(
                "{} Current directory is repository root",
                "📁".bright_blue()
            );
            println!(
                "{} Repository: {}",
                "📁".bright_blue(),
                repo_name.bright_cyan()
            );
            println!(
                "{} Repository path: {}",
                "📦".bright_blue(),
                current_dir.display()
            );
            (current_dir.clone(), repo_name)
        };

        Ok((repo_path, repo_name))
    }

    /// Detect appropriate root path based on current repository structure
    async fn detect_appropriate_root_path(&self) -> Result<PathBuf> {
        let current_dir = env::current_dir().context("Failed to get current directory")?;
        let current_dir = current_dir.canonicalize().unwrap_or(current_dir);

        let current_dir_name = current_dir
            .file_name()
            .and_then(|n| n.to_str())
            .context("Failed to get current directory name")?;

        let root_path = if current_dir_name.starts_with("trunk-") {
            // We're in a trunk directory, so the grandparent is the projects root
            let repo_dir = current_dir
                .parent()
                .context("Failed to get repository directory")?;
            let projects_root = repo_dir
                .parent()
                .context("Failed to get projects root directory")?;
            
            println!(
                "{} Auto-detected projects root: {}",
                "🔍".bright_yellow(),
                projects_root.display()
            );
            
            projects_root.to_path_buf()
        } else {
            // We're at repo root, use parent as projects root, or fall back to standard logic
            if let Some(parent) = current_dir.parent() {
                println!(
                    "{} Auto-detected projects root: {}",
                    "🔍".bright_yellow(),
                    parent.display()
                );
                parent.to_path_buf()
            } else {
                // Fallback to environment variable or default
                if let Ok(imi_root) = std::env::var("IMI_ROOT") {
                    PathBuf::from(imi_root)
                } else {
                    // Default to ~/code/
                    dirs::home_dir()
                        .unwrap_or_else(|| PathBuf::from("/home/delorenj"))
                        .join("code")
                }
            }
        };

        Ok(root_path)
    }

    /// Setup configuration handling existing config and force flag
    async fn setup_configuration(&self) -> Result<Config> {
        let config_path = Config::get_config_path()?;
        let config_exists = config_path.exists();

        if config_exists && !self.force {
            println!(
                "{} iMi configuration already exists at: {}",
                "⚠️".bright_yellow(),
                config_path.display()
            );
            println!(
                "{} Use {} to override existing configuration",
                "💡".bright_blue(),
                "--force".bright_green()
            );

            // Load and show current configuration
            if let Ok(existing_config) = Config::load().await {
                println!("\n{} Current configuration:", "🔍".bright_cyan());
                println!(
                    "   {} {}",
                    "Root path:".bright_yellow(),
                    existing_config.root_path.display()
                );
                println!(
                    "   {} {}",
                    "Database:".bright_yellow(),
                    existing_config.database_path.display()
                );
            }

            // Return the existing config
            return Config::load().await.context("Failed to load existing configuration");
        }

        // Load existing config or create default
        let config = if config_exists {
            let mut cfg = Config::load()
                .await
                .context("Failed to load existing configuration")?;
            // When forcing, update root path based on current repository structure
            if self.force {
                cfg.root_path = self.detect_appropriate_root_path().await?;
            }
            cfg
        } else {
            // Create default config with auto-detected root path
            let mut cfg = Config::default();
            cfg.root_path = self.detect_appropriate_root_path().await?;
            cfg
        };

        // Save the configuration if it's new or if forced
        if !config_exists || self.force {
            config
                .save()
                .await
                .context("Failed to save configuration")?;
        }

        Ok(config)
    }

    /// Initialize database connection and ensure tables exist
    async fn initialize_database(&self, config: &Config) -> Result<Database> {
        let database = Database::new(&config.database_path)
            .await
            .context("Failed to initialize database")?;

        // Ensure database tables are created
        database.ensure_tables()
            .await
            .context("Failed to ensure database tables")?;

        Ok(database)
    }

    /// Register repository in database if it doesn't exist
    async fn register_repository(
        &self,
        database: &Database,
        repo_name: &str,
        repo_path: &PathBuf,
    ) -> Result<()> {
        let current_dir = env::current_dir().context("Failed to get current directory")?;
        let current_dir_name = current_dir
            .file_name()
            .and_then(|n| n.to_str())
            .context("Failed to get current directory name")?;

        // Check if repository exists, create only if it doesn't
        if database.get_repository(repo_name).await?.is_none() {
            database
                .create_repository(
                    repo_name,
                    repo_path.to_str().unwrap_or(""),
                    "", // Remote URL can be updated later
                    if current_dir_name.starts_with("trunk-") {
                        current_dir_name.trim_start_matches("trunk-")
                    } else {
                        "main" // Default branch name
                    },
                )
                .await
                .context("Failed to create repository record")?;
            
            println!(
                "{} Registered repository in database",
                "📝".bright_cyan()
            );
        } else {
            println!(
                "{} Repository already registered in database",
                "ℹ️".bright_blue()
            );
        }

        Ok(())
    }

    /// Register trunk worktree if we're in a trunk directory
    async fn register_trunk_worktree(
        &self,
        database: &Database,
        repo_name: &str,
    ) -> Result<()> {
        let current_dir = env::current_dir().context("Failed to get current directory")?;
        let current_dir_name = current_dir
            .file_name()
            .and_then(|n| n.to_str())
            .context("Failed to get current directory name")?;

        // Register trunk worktree in database if we're in a trunk directory
        if current_dir_name.starts_with("trunk-") {
            let branch_name = current_dir_name.trim_start_matches("trunk-");

            // Create the trunk worktree record
            database
                .create_worktree(
                    repo_name,
                    current_dir_name,
                    branch_name,
                    "trunk",
                    current_dir.to_str().unwrap_or(""),
                    None, // No agent_id for manual init
                )
                .await
                .context("Failed to register trunk worktree in database")?;

            println!(
                "{} Registered trunk worktree in database",
                "📝".bright_cyan()
            );
        }

        Ok(())
    }

    /// Display success information and paths
    async fn display_success_info(
        &self,
        config: &Config,
        repo_name: &str,
        repo_path: &PathBuf,
    ) -> Result<()> {
        let config_path = Config::get_config_path()?;
        let config_exists = config_path.exists();

        // Success messages
        if config_exists && !self.force {
            println!("{} Using existing iMi configuration", "⚙️".bright_green());
        } else if config_exists && self.force {
            println!("{} Reinitialized iMi configuration", "🔄".bright_green());
        } else {
            println!("{} Created new iMi configuration", "✨".bright_green());
        }

        println!(
            "{} Repository: {}",
            "📦".bright_blue(),
            repo_name.bright_cyan()
        );
        println!(
            "{} Repository path: {}",
            "📂".bright_blue(),
            repo_path.display()
        );
        println!(
            "{} Global iMi root: {}",
            "🏠".bright_green(),
            config.root_path.display()
        );

        println!(
            "{} Configuration path: {}",
            "💾".bright_cyan(),
            config_path.display()
        );
        println!(
            "{} Database path: {}",
            "🗝️".bright_cyan(),
            config.database_path.display()
        );
        println!("{} iMi initialization complete!", "✅".bright_green());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_init_command_new() {
        let cmd = InitCommand::new(false);
        assert!(!cmd.force);

        let cmd_force = InitCommand::new(true);
        assert!(cmd_force.force);
    }

    #[tokio::test]
    async fn test_validation_result() {
        let mut result = ValidationResult::new();
        assert!(result.is_valid());
        assert!(result.errors.is_empty());
        assert!(result.warnings.is_empty());

        result.add_error("Test error".to_string());
        assert!(!result.is_valid());
        assert_eq!(result.errors.len(), 1);

        result.add_warning("Test warning".to_string());
        assert_eq!(result.warnings.len(), 1);
    }

    #[tokio::test]
    async fn test_init_result() {
        let success = InitResult::success("Success message".to_string());
        assert!(success.success);
        assert_eq!(success.message, "Success message");

        let failure = InitResult::failure("Failure message".to_string());
        assert!(!failure.success);
        assert_eq!(failure.message, "Failure message");
    }
}