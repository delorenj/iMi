use anyhow::{anyhow, Context, Result};
use colored::*;
use dialoguer::{Confirm, Select};
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

        let repo_path_str = repo_path
            .to_str()
            .context("Repository path contains invalid UTF-8 characters")?;

        self.db
            .create_repository(
                repo_name,
                repo_path_str,
                &remote_url,
                &default_branch,
            )
            .await?;
        println!(
            "{} Registered repository '{}' in the database.",
            "✅".bright_green(),
            repo_name
        );

        let imi_parent = repo_path
            .parent()
            .context("Repository path has no parent directory")?;
        let imi_dir = imi_parent.join(".iMi");
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

    /// Clone a repository with enhanced logic for multiple formats and existing directory handling
    pub async fn clone_repository(&self, repo_arg: &str) -> Result<InitResult> {
        // Parse the repository argument into owner/repo format
        let (owner, repo_name, git_url) = self.parse_repo_argument(repo_arg)?;

        println!(
            "{} Cloning {}/{} from GitHub...",
            "🔍".bright_cyan(),
            owner.bright_white(),
            repo_name.bright_white()
        );

        // Determine clone location using config's IMI_SYSTEM_PATH (root_path)
        let repo_container = self.config.root_path.join(&repo_name);
        let trunk_path = repo_container.join("trunk-main");

        // Check if directory already exists
        if repo_container.exists() {
            return self.handle_existing_directory(&repo_container, &trunk_path, &repo_name, &owner).await;
        }

        // Create container directory and clone
        fs::create_dir_all(&repo_container)
            .await
            .context("Failed to create repository container")?;

        println!(
            "{} Cloning into {}...",
            "📁".bright_blue(),
            trunk_path.display().to_string().bright_white()
        );

        let trunk_path_str = trunk_path
            .to_str()
            .context("Repository path contains invalid UTF-8 characters")?;

        // Ensure git inherits environment for credential helper and disable terminal prompts
        let output = tokio::process::Command::new("git")
            .args(&["clone", &git_url, trunk_path_str])
            .env("GIT_TERMINAL_PROMPT", "0") // Disable interactive credential prompts
            .output()
            .await
            .context("Failed to execute git clone")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);

            // Attempt cleanup with proper error handling
            if let Err(cleanup_err) = fs::remove_dir_all(&repo_container).await {
                eprintln!("{} Failed to cleanup directory after clone failure: {}",
                    "⚠️".bright_yellow(),
                    cleanup_err
                );
            }

            // Parse git errors for better user feedback
            let error_msg = if stderr.contains("Authentication failed") || stderr.contains("authentication") {
                format!("Authentication failed for {}/{}. Check your SSH keys or GitHub token (gh auth login)", owner, repo_name)
            } else if stderr.contains("not found") || stderr.contains("does not exist") {
                format!("Repository {}/{} not found on GitHub. Verify the repository exists and you have access.", owner, repo_name)
            } else if stderr.contains("No space") || stderr.contains("disk") {
                "Insufficient disk space to clone repository".to_string()
            } else {
                format!("Git clone failed: {}", stderr)
            };

            return Err(anyhow!("{}", error_msg));
        }

        println!("{} Clone complete!", "✅".bright_green());

        // Now initialize iMi in the cloned repository
        self.register_repository(&trunk_path, &repo_name).await
    }

    /// Handle existing directory - either switch to existing iMi repo or convert non-iMi repo
    async fn handle_existing_directory(
        &self,
        repo_container: &Path,
        trunk_path: &Path,
        repo_name: &str,
        _owner: &str,
    ) -> Result<InitResult> {
        // Check if trunk-main exists (indicating iMi repo)
        if trunk_path.exists() {
            // This is already an iMi repo
            println!(
                "{} Repository already exists as iMi repo at: {}",
                "ℹ️".bright_blue(),
                repo_container.display()
            );

            // Check if it's registered in the database
            match self.db.get_repository(repo_name).await? {
                Some(_) => {
                    println!(
                        "{} Repository '{}' is already registered. Switching to it...",
                        "✅".bright_green(),
                        repo_name
                    );
                }
                None => {
                    println!(
                        "{} Repository exists but not registered. Registering now...",
                        "🔧".bright_yellow()
                    );
                    self.register_repository(trunk_path, repo_name).await?;
                }
            }

            return Ok(InitResult::success(format!(
                "Repository already exists at {}\nTo navigate: cd {} or use 'igo {}'",
                repo_container.display(),
                trunk_path.display(),
                repo_name
            )));
        }

        // Directory exists but is not an iMi repo - need to convert it
        println!(
            "{} Directory exists but is not an iMi repository.",
            "⚠️".bright_yellow()
        );
        println!(
            "{} Converting to iMi structure using imify.py...",
            "🔄".bright_cyan()
        );

        // Call the imify.py script
        let imify_script = dirs::home_dir()
            .context("Could not determine home directory")?
            .join(".config/zshyzsh/scripts/imify.py");

        if !imify_script.exists() {
            return Err(anyhow!(
                "imify.py script not found at {}. Please ensure it exists.",
                imify_script.display()
            ));
        }

        // Verify parent directory exists for working directory
        let parent_dir = repo_container
            .parent()
            .context("Repository container has no parent directory")?;

        // Run imify.py on the directory
        let output = tokio::process::Command::new("python3")
            .arg(&imify_script)
            .arg(repo_container)
            .current_dir(parent_dir)
            .output()
            .await
            .context("Failed to execute imify.py")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to convert directory to iMi structure: {}", stderr));
        }

        println!("{} Conversion complete!", "✅".bright_green());

        // Now register the repository
        self.register_repository(trunk_path, repo_name).await
    }

    /// Validate repository name for security
    fn validate_repo_name(name: &str) -> Result<()> {
        // Disallow empty names
        if name.is_empty() {
            return Err(anyhow!("Repository name cannot be empty"));
        }

        // Disallow path traversal
        if name.contains("..") {
            return Err(anyhow!("Repository name cannot contain '..'"));
        }

        // Disallow absolute paths
        if name.starts_with('/') || name.starts_with('\\') {
            return Err(anyhow!("Repository name cannot be an absolute path"));
        }

        // Disallow null bytes and newlines
        if name.contains('\0') || name.contains('\n') || name.contains('\r') {
            return Err(anyhow!("Repository name contains invalid characters"));
        }

        // Disallow shell metacharacters
        let dangerous_chars = ['&', '|', ';', '$', '`', '(', ')', '<', '>', '!', '{', '}', '[', ']', '*', '?', '~'];
        if name.chars().any(|c| dangerous_chars.contains(&c)) {
            return Err(anyhow!("Repository name contains shell metacharacters"));
        }

        // Only allow alphanumeric, dash, underscore, dot, forward slash (for owner/repo)
        let valid_chars = name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '/');
        if !valid_chars {
            return Err(anyhow!("Repository name contains invalid characters"));
        }

        Ok(())
    }

    /// Parse repository argument into (owner, repo_name, git_url)
    /// Supports three formats:
    /// 1. "name" -> defaults to "delorenj/name"
    /// 2. "user/name" -> uses provided owner
    /// 3. "https://github.com/user/name.git" -> extracts owner and name
    /// Note: All formats result in SSH URLs (git@github.com:owner/repo.git) for authentication
    fn parse_repo_argument(&self, repo_arg: &str) -> Result<(String, String, String)> {
        // Check if it's a full GitHub URL
        if repo_arg.starts_with("http://") || repo_arg.starts_with("https://") {
            // Reject insecure HTTP
            if repo_arg.starts_with("http://") {
                return Err(anyhow!("HTTP URLs are not supported. Use HTTPS for security."));
            }

            // Verify it's a GitHub URL
            if !repo_arg.contains("github.com") {
                return Err(anyhow!("Only github.com repositories are supported"));
            }

            // Extract owner/repo from URL - handle both https://github.com/owner/repo and https://github.com/owner/repo.git
            let url_without_protocol = repo_arg
                .trim_start_matches("https://")
                .trim_start_matches("http://");

            // Remove github.com/ prefix
            let path_part = url_without_protocol
                .strip_prefix("github.com/")
                .context("Invalid GitHub URL format")?;

            // Split on / to get owner and repo
            let parts: Vec<&str> = path_part.split('/').collect();
            if parts.len() < 2 {
                return Err(anyhow!("Invalid GitHub URL format: expected owner/repo"));
            }

            let owner = parts[0].to_string();
            let repo_name = parts[1].trim_end_matches(".git").to_string();

            // Validate extracted names
            Self::validate_repo_name(&owner)?;
            Self::validate_repo_name(&repo_name)?;

            // Use SSH URL for authentication
            let git_url = format!("git@github.com:{}/{}.git", owner, repo_name);

            return Ok((owner, repo_name, git_url));
        }

        // Check if it contains a slash (owner/repo format)
        if repo_arg.contains('/') {
            let parts: Vec<&str> = repo_arg.split('/').collect();
            if parts.len() != 2 {
                return Err(anyhow!(
                    "Invalid repository format. Expected: name, user/name, or full URL"
                ));
            }

            let owner = parts[0].to_string();
            let repo_name = parts[1].to_string();

            // Validate both parts
            Self::validate_repo_name(&owner)?;
            Self::validate_repo_name(&repo_name)?;

            // Use SSH URL for authentication
            let git_url = format!("git@github.com:{}/{}.git", owner, repo_name);

            return Ok((owner, repo_name, git_url));
        }

        // Just a name - default to delorenj
        let owner = "delorenj".to_string();
        let repo_name = repo_arg.to_string();

        // Validate the repository name
        Self::validate_repo_name(&repo_name)?;

        // Use SSH URL for authentication
        let git_url = format!("git@github.com:{}/{}.git", owner, repo_name);

        Ok((owner, repo_name, git_url))
    }

    /// Clone a repository from GitHub and set up iMi structure (legacy method)
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
