use anyhow::{Context, Result};
use colored::*;
use dialoguer::Confirm;
use std::env;
use std::os::unix::fs;
use std::path::{Path, PathBuf};
use tokio::fs as async_fs;

use crate::config::Config;
use crate::database::Database;
use crate::error::ImiError;
use crate::fuzzy::FuzzyMatcher;
use crate::git::{GitManager, WorktreeStatus};

#[derive(Debug, Clone)]
pub struct WorktreeManager {
    pub git: GitManager,
    pub db: Database,
    pub config: Config,
    pub repo_path: Option<PathBuf>,
}

impl WorktreeManager {
    pub fn new(git: GitManager, db: Database, config: Config, repo_path: Option<PathBuf>) -> Self {
        Self {
            git,
            db,
            config,
            repo_path,
        }
    }

    /// Create a feature worktree
    pub async fn create_feature_worktree(&self, name: &str, repo: Option<&str>) -> Result<PathBuf> {
        let worktree_name = format!("feat-{}", name);
        let branch_name = format!("feat/{}", name);

        self.create_worktree_internal(
            repo,
            &worktree_name,
            &branch_name,
            "feat",
            Some(&self.config.git_settings.default_branch),
        )
        .await
    }

    /// Create a review worktree for a PR
    pub async fn create_review_worktree(
        &self,
        pr_number: u32,
        repo: Option<&str>,
    ) -> Result<PathBuf> {
        // Use gh CLI for PR checkout - this includes validation
        self.create_pr_worktree_with_gh(pr_number, repo).await
    }

    /// Create a fix worktree
    pub async fn create_fix_worktree(&self, name: &str, repo: Option<&str>) -> Result<PathBuf> {
        let worktree_name = format!("fix-{}", name);
        let branch_name = format!("fix/{}", name);

        self.create_worktree_internal(
            repo,
            &worktree_name,
            &branch_name,
            "fix",
            Some(&self.config.git_settings.default_branch),
        )
        .await
    }

    /// Create an aiops worktree
    pub async fn create_aiops_worktree(&self, name: &str, repo: Option<&str>) -> Result<PathBuf> {
        let worktree_name = format!("aiops-{}", name);
        let branch_name = format!("aiops/{}", name);

        self.create_worktree_internal(
            repo,
            &worktree_name,
            &branch_name,
            "aiops",
            Some(&self.config.git_settings.default_branch),
        )
        .await
    }

    /// Create a devops worktree
    pub async fn create_devops_worktree(&self, name: &str, repo: Option<&str>) -> Result<PathBuf> {
        let worktree_name = format!("devops-{}", name);
        let branch_name = format!("devops/{}", name);

        self.create_worktree_internal(
            repo,
            &worktree_name,
            &branch_name,
            "devops",
            Some(&self.config.git_settings.default_branch),
        )
        .await
    }

    /// Create a custom worktree using database-defined type metadata
    pub async fn create_custom_worktree(
        &self,
        name: &str,
        worktree_type: &str,
        repo: Option<&str>,
    ) -> Result<PathBuf> {
        // Get the worktree type metadata from database
        let wt_type = self.db
            .get_worktree_type(worktree_type)
            .await
            .context(format!(
                "Unknown worktree type '{}'. Run 'imi types' to see available types.",
                worktree_type
            ))?;

        // Build worktree and branch names using type metadata
        let worktree_name = format!("{}{}", wt_type.worktree_prefix, name);
        let branch_name = format!("{}{}", wt_type.branch_prefix, name);

        self.create_worktree_internal(
            repo,
            &worktree_name,
            &branch_name,
            worktree_type,
            Some(&self.config.git_settings.default_branch),
        )
        .await
    }

    /// Get the trunk worktree path
    pub async fn get_trunk_worktree(&self, repo: Option<&str>) -> Result<PathBuf> {
        let repo_name = self.resolve_repo_name(repo).await?;
        let trunk_name = format!("trunk-{}", self.config.git_settings.default_branch);

        let worktree_path = self.config.get_worktree_path(&repo_name, &trunk_name);

        if !worktree_path.exists() {
            return Err(anyhow::anyhow!(
                "Trunk worktree not found at: {}. Please run 'imi trunk' from the repository root first.",
                worktree_path.display()
            ));
        }

        Ok(worktree_path)
    }

    /// Internal worktree creation logic
    async fn create_worktree_internal(
        &self,
        repo: Option<&str>,
        worktree_name: &str,
        branch_name: &str,
        worktree_type: &str,
        base_branch: Option<&str>,
    ) -> Result<PathBuf> {
        let repo_name = self.resolve_repo_name(repo).await?;

        // Get worktree path - apply IMI_PATH logic to both registered and unregistered repos
        let worktree_path =
            if let Some(registered_repo) = self.db.get_repository(&repo_name).await? {
                // Use registered repository path but apply IMI_PATH detection
                let registered_path = PathBuf::from(&registered_repo.path);
                let imi_path = self.detect_imi_path(&registered_path)?;
                imi_path.join(worktree_name)
            } else {
                // Fall back to current repository location with IMI_PATH detection
                let current_dir = env::current_dir()?;
                let repo = self.git.find_repository(Some(&current_dir))?;
                let repo_root = repo
                    .workdir()
                    .ok_or_else(|| anyhow::anyhow!("Repository has no working directory"))?;

                // Detect IMI_PATH - if we're in a trunk directory, use its parent
                let imi_path = self.detect_imi_path(repo_root)?;
                imi_path.join(worktree_name)
            };

        // Check if worktree already exists
        if let Some(_existing) = self.db.get_worktree(&repo_name, worktree_name).await? {
            if worktree_path.exists() {
                println!(
                    "{} Worktree already exists: {}",
                    "ℹ️".bright_blue(),
                    worktree_path.display()
                );
                return Ok(worktree_path);
            } else {
                // Clean up stale database entry
                self.db
                    .deactivate_worktree(&repo_name, worktree_name)
                    .await?;
            }
        }

        // Find the repository - use current directory and register if needed
        let current_dir = env::current_dir()?;
        let repo = self.git.find_repository(Some(&current_dir))?;
        let repo_root = repo
            .workdir()
            .ok_or_else(|| anyhow::anyhow!("Repository has no working directory"))?;

        // Auto-register repository if not in database
        self.ensure_repository_registered(&repo_name, repo_root)
            .await?;

        // Create the Git worktree
        self.git
            .create_worktree(
                &repo,
                worktree_name,
                &worktree_path,
                branch_name,
                base_branch,
            )
            .context("Failed to create Git worktree")?;

        // Create sync directories
        self.create_sync_directories(&repo_name).await?;

        // Create symlinks for dotfiles
        self.create_symlinks(&repo_name, &worktree_path).await?;

        // Record the worktree in the database
        self.db
            .create_worktree(
                &repo_name,
                worktree_name,
                branch_name,
                worktree_type,
                worktree_path.to_str().unwrap(),
                None, // agent_id will be set later if needed
            )
            .await?;

        println!("{} Worktree created successfully", "✅".bright_green());

        Ok(worktree_path)
    }

    /// Create PR worktree using gh CLI
    async fn create_pr_worktree_with_gh(
        &self,
        pr_number: u32,
        repo: Option<&str>,
    ) -> Result<PathBuf> {
        let repo_name = self.resolve_repo_name(repo).await?;
        let worktree_name = format!("pr-{}", pr_number);

        // Get worktree path - apply IMI_PATH logic to both registered and unregistered repos
        let (worktree_path, trunk_path) =
            if let Some(registered_repo) = self.db.get_repository(&repo_name).await? {
                // Use registered repository path with IMI_PATH detection
                let registered_path = PathBuf::from(&registered_repo.path);
                let imi_path = self.detect_imi_path(&registered_path)?;
                let worktree_path = imi_path.join(&worktree_name);
                let trunk_path = self.config.get_trunk_path(&repo_name);
                (worktree_path, trunk_path)
            } else {
                // Fall back to current repository location with IMI_PATH detection
                let current_dir = env::current_dir()?;
                let repo = self.git.find_repository(Some(&current_dir))?;
                let repo_root = repo
                    .workdir()
                    .ok_or_else(|| anyhow::anyhow!("Repository has no working directory"))?;
                let imi_path = self.detect_imi_path(repo_root)?;
                let worktree_path = imi_path.join(&worktree_name);
                let trunk_path = self.config.get_trunk_path(&repo_name);
                (worktree_path, trunk_path)
            };

        // Try to checkout PR using gh CLI
        let _repo = self.git.find_repository(Some(&trunk_path))?;
        self.git
            .checkout_pr(&trunk_path, pr_number, &worktree_path)?;

        // Create sync directories and symlinks
        self.create_sync_directories(&repo_name).await?;
        self.create_symlinks(&repo_name, &worktree_path).await?;

        // Get the actual branch name from the checked out PR
        let branch_name = self
            .git
            .get_current_branch(&worktree_path)
            .unwrap_or_else(|_| format!("pr/{}", pr_number));

        // Record in database
        self.db
            .create_worktree(
                &repo_name,
                &worktree_name,
                &branch_name,
                "pr",
                worktree_path.to_str().unwrap(),
                None,
            )
            .await?;

        Ok(worktree_path)
    }

    /// Create sync directories as per PRD specifications
    async fn create_sync_directories(&self, repo_name: &str) -> Result<()> {
        let user_sync = self.config.get_sync_path(repo_name, true);
        let local_sync = self.config.get_sync_path(repo_name, false);

        // Create sync/user directory
        async_fs::create_dir_all(&user_sync)
            .await
            .context("Failed to create user sync directory")?;

        // Create sync/local directory
        async_fs::create_dir_all(&local_sync)
            .await
            .context("Failed to create local sync directory")?;

        // Create default sync files if they don't exist
        let coding_rules = user_sync.join("coding-rules.md");
        if !coding_rules.exists() {
            async_fs::write(
                &coding_rules,
                "# Coding Rules\n\n## Style Guidelines\n\n## Best Practices\n",
            )
            .await?;
        }

        let stack_specific = user_sync.join("stack-specific.md");
        if !stack_specific.exists() {
            async_fs::write(
                &stack_specific,
                "# Stack-Specific Guidelines\n\n## Frontend\n\n## Backend\n\n## Database\n",
            )
            .await?;
        }

        Ok(())
    }

    /// Create symlinks for dotfiles and config files
    async fn create_symlinks(&self, repo_name: &str, worktree_path: &Path) -> Result<()> {
        let local_sync = self.config.get_sync_path(repo_name, false);

        for file_name in &self.config.symlink_files {
            let source = local_sync.join(file_name);
            let target = worktree_path.join(file_name);

            // Create parent directories if needed
            if let Some(parent) = target.parent() {
                async_fs::create_dir_all(parent).await?;
            }

            // Create symlink if source exists and target doesn't
            if source.exists() && !target.exists() {
                fs::symlink(&source, &target).map_err(|e| ImiError::SymlinkCreationFailed {
                    source: source.display().to_string(),
                    target: target.display().to_string(),
                    io_error: e,
                })?;

                println!(
                    "{} Created symlink: {} -> {}",
                    "🔗".bright_cyan(),
                    target.display(),
                    source.display()
                );
            }
        }

        Ok(())
    }

    /// Remove a worktree
    pub async fn remove_worktree(
        &self,
        name: &str,
        repo: Option<&str>,
        keep_branch: bool,
        keep_remote: bool,
    ) -> Result<()> {
        let repo_name = self.resolve_repo_name(repo).await?;

        // Find the actual worktree name - it might be prefixed (e.g., feat-iteractive-learning)
        let actual_worktree_name = self.find_actual_worktree_name(name, &repo_name).await?;

        // Use IMI_PATH detection for consistent worktree removal
        let current_dir = env::current_dir()?;
        let repo = self.git.find_repository(Some(&current_dir))?;
        let repo_root = repo
            .workdir()
            .ok_or_else(|| anyhow::anyhow!("Repository has no working directory"))?;
        let imi_path = self.detect_imi_path(repo_root)?;
        let worktree_path = imi_path.join(&actual_worktree_name);

        // Get worktree info from database before removing
        let worktree_info = self
            .db
            .get_worktree(&repo_name, &actual_worktree_name)
            .await?;
        let branch_name = worktree_info.as_ref().map(|w| w.branch_name.clone());

        // Remove directory first
        if worktree_path.exists() {
            async_fs::remove_dir_all(&worktree_path)
                .await
                .context("Failed to remove worktree directory")?;
        }

        // Remove from Git (this will now be able to prune since directory is gone)
        if self.git.worktree_exists(&repo, &actual_worktree_name) {
            self.git.remove_worktree(&repo, &actual_worktree_name)?;
        }

        // Handle branch deletion (default is to delete unless explicitly kept)
        if !keep_branch {
            if let Some(branch) = &branch_name {
                // Delete local branch
                self.git.delete_local_branch(&repo, branch)?;

                // Delete remote branch (default is to delete unless explicitly kept)
                if !keep_remote {
                    if let Err(e) = self.git.delete_remote_branch(&repo, branch).await {
                        println!("⚠️ Could not delete remote branch '{}': {}", branch, e);
                        println!(
                            "   (This is normal if the branch was already deleted or never pushed)"
                        );
                    }
                }
            }
        }

        // Deactivate in database
        self.db
            .deactivate_worktree(&repo_name, &actual_worktree_name)
            .await?;

        Ok(())
    }

    /// Close a worktree without deleting the branch
    /// This removes the worktree directory and git reference but preserves the branch
    pub async fn close_worktree(&self, name: &str, repo: Option<&str>) -> Result<()> {
        // Try to find the worktree in the database first
        // This allows the command to work from any directory when an explicit name is provided
        let (repo_name, actual_worktree_name, worktree_path, git_repo) =
            if let Some(worktree_info) = self.find_worktree_in_database(name, repo).await? {
                // Found in database - use the stored information
                let path = PathBuf::from(&worktree_info.path);

                // Get the repository path from the database
                let repo_record = self
                    .db
                    .get_repository(&worktree_info.repo_name)
                    .await?
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "Repository not found in database: {}",
                            worktree_info.repo_name
                        )
                    })?;

                let repo_path_buf = PathBuf::from(repo_record.path);

                // Find the git repository - it should be at the registered path
                let git_repo = self
                    .git
                    .find_repository(Some(&repo_path_buf))
                    .context(format!(
                        "Failed to find Git repository at: {}",
                        repo_path_buf.display()
                    ))?;

                (
                    worktree_info.repo_name,
                    worktree_info.worktree_name,
                    path,
                    git_repo,
                )
            } else {
                // Fall back to current directory-based lookup
                let repo_name = self.resolve_repo_name(repo).await?;
                let actual_worktree_name = self.find_actual_worktree_name(name, &repo_name).await?;

                let current_dir = env::current_dir()?;
                let repo = self.git.find_repository(Some(&current_dir))?;
                let repo_root = repo
                    .workdir()
                    .ok_or_else(|| anyhow::anyhow!("Repository has no working directory"))?;
                let imi_path = self.detect_imi_path(repo_root)?;
                let worktree_path = imi_path.join(&actual_worktree_name);

                (repo_name, actual_worktree_name, worktree_path, repo)
            };

        // Remove directory first
        if worktree_path.exists() {
            async_fs::remove_dir_all(&worktree_path)
                .await
                .context("Failed to remove worktree directory")?;
        }

        // Remove from Git (this will now be able to prune since directory is gone)
        if self.git.worktree_exists(&git_repo, &actual_worktree_name) {
            self.git.remove_worktree(&git_repo, &actual_worktree_name)?;
        }

        // Deactivate in database
        self.db
            .deactivate_worktree(&repo_name, &actual_worktree_name)
            .await?;

        Ok(())
    }

    /// Show status of worktrees
    pub async fn show_status(&self, repo: Option<&str>) -> Result<()> {
        let worktrees = self.db.list_worktrees(repo).await?;

        if worktrees.is_empty() {
            println!("{} No active worktrees found", "ℹ️".bright_blue());
            return Ok(());
        }

        println!("\n{}", "Active Worktrees:".bright_cyan().bold());
        println!("{}", "─".repeat(80).bright_black());

        for worktree in worktrees {
            let status_icon = match worktree.worktree_type.as_str() {
                "feat" => "🚀",
                "pr" => "🔍",
                "fix" => "🔧",
                "aiops" => "🤖",
                "devops" => "⚙️",
                "trunk" => "🌳",
                _ => "📁",
            };

            println!(
                "{} {} {} ({})",
                status_icon,
                worktree.worktree_name.bright_green(),
                worktree.branch_name.bright_yellow(),
                worktree.worktree_type.bright_blue()
            );

            // Get Git status if worktree path exists
            let worktree_path = PathBuf::from(&worktree.path);
            if worktree_path.exists() {
                if let Ok(git_status) = self.git.get_worktree_status(&worktree_path) {
                    self.print_git_status(&git_status);
                }
            } else {
                println!(
                    "   {} Path not found: {}",
                    "⚠️".bright_yellow(),
                    worktree.path
                );
            }

            if let Some(agent_id) = &worktree.agent_id {
                println!("   {} Agent: {}", "🤖".bright_magenta(), agent_id);
            }

            println!(
                "   {} Created: {}",
                "📅".bright_black(),
                worktree.created_at.format("%Y-%m-%d %H:%M:%S")
            );
            println!();
        }

        Ok(())
    }

    fn print_git_status(&self, status: &WorktreeStatus) {
        if status.clean {
            println!("   {} Working tree clean", "✅".bright_green());
        } else {
            if !status.modified_files.is_empty() {
                println!(
                    "   {} Modified: {}",
                    "📝".bright_yellow(),
                    status.modified_files.len()
                );
            }
            if !status.new_files.is_empty() {
                println!(
                    "   {} New files: {}",
                    "➕".bright_green(),
                    status.new_files.len()
                );
            }
            if !status.deleted_files.is_empty() {
                println!(
                    "   {} Deleted: {}",
                    "➖".bright_red(),
                    status.deleted_files.len()
                );
            }
        }

        if status.commits_ahead > 0 {
            println!(
                "   {} {} commits ahead",
                "⬆️".bright_green(),
                status.commits_ahead
            );
        }
        if status.commits_behind > 0 {
            println!(
                "   {} {} commits behind",
                "⬇️".bright_red(),
                status.commits_behind
            );
        }
    }

    fn print_git_status_indented(&self, status: &WorktreeStatus) {
        if status.clean {
            println!("     {} Working tree clean", "✅".bright_green());
        } else {
            if !status.modified_files.is_empty() {
                println!(
                    "     {} Modified: {}",
                    "📝".bright_yellow(),
                    status.modified_files.len()
                );
            }
            if !status.new_files.is_empty() {
                println!(
                    "     {} New files: {}",
                    "➕".bright_green(),
                    status.new_files.len()
                );
            }
            if !status.deleted_files.is_empty() {
                println!(
                    "     {} Deleted: {}",
                    "➖".bright_red(),
                    status.deleted_files.len()
                );
            }
        }

        if status.commits_ahead > 0 {
            println!(
                "     {} {} commits ahead",
                "⬆️".bright_green(),
                status.commits_ahead
            );
        }
        if status.commits_behind > 0 {
            println!(
                "     {} {} commits behind",
                "⬇️".bright_red(),
                status.commits_behind
            );
        }
    }

    /// Smart context-aware list command
    /// Implements the decision tree based on context and flags
    pub async fn list_smart(
        &self,
        repo: Option<&str>,
        worktrees_flag: bool,
        projects_flag: bool,
    ) -> Result<()> {
        let current_dir = env::current_dir()?;
        let git_context = self.git.detect_context(Some(&current_dir));

        // Extract repository information from context
        let detected_repo_path = git_context.repo_path();
        let detected_repo_name = if let Some(repo_path) = detected_repo_path {
            self.git.get_repo_name(repo_path).ok()
        } else {
            None
        };

        // Handle explicit --projects flag
        if projects_flag {
            return self.list_all_repositories().await;
        }

        // Handle explicit --worktrees flag
        if worktrees_flag {
            // If repo is specified, list that repo's worktrees
            if let Some(repo_name) = repo {
                return self.list_worktrees_detailed(Some(repo_name)).await;
            }
            // If in a repo, list that repo's worktrees
            if let Some(repo_name) = detected_repo_name.as_ref() {
                return self.list_worktrees_detailed(Some(repo_name)).await;
            }
            // Not in a repo, list all worktrees
            return self.list_worktrees_detailed(None).await;
        }

        // Handle explicit --repo flag
        if let Some(repo_name) = repo {
            // Check if repo is registered
            if let Some(_registered) = self.db.get_repository(repo_name).await? {
                // Registered: list its worktrees
                return self.list_worktrees_detailed(Some(repo_name)).await;
            } else {
                // Not registered: show helpful error
                println!(
                    "{} Repository '{}' is not registered",
                    "⚠️".bright_yellow(),
                    repo_name.bright_red()
                );
                println!(
                    "\n{} To register this repository, run {} from its directory",
                    "💡".bright_yellow(),
                    "imi trunk".bright_green()
                );
                return Ok(());
            }
        }

        // No explicit flags - use context detection
        if git_context.is_in_repository() {
            // We're in a git repository
            if let Some(repo_name) = detected_repo_name {
                // Check if this repo is registered
                if let Some(_registered) = self.db.get_repository(&repo_name).await? {
                    // Registered: list its worktrees
                    return self.list_worktrees_detailed(Some(&repo_name)).await;
                } else {
                    // Unregistered: show helpful message
                    println!(
                        "\n{} {} {}",
                        "📦".bright_cyan(),
                        "Repository:".bright_white(),
                        repo_name.bright_yellow()
                    );
                    println!(
                        "{} {}",
                        "⚠️".bright_yellow(),
                        "This repository is not registered with iMi".bright_yellow()
                    );
                    println!(
                        "\n{} To start using iMi with this repository:",
                        "💡".bright_yellow()
                    );
                    println!(
                        "   1. Run {} to create the trunk worktree",
                        "imi trunk".bright_green()
                    );
                    println!(
                        "   2. Then use {} to create feature worktrees",
                        "imi feat <name>".bright_green()
                    );
                    println!(
                        "\n{} Once registered, {} will show worktrees for this repo",
                        "ℹ️".bright_blue(),
                        "imi list".bright_cyan()
                    );
                    return Ok(());
                }
            } else {
                // In a repo but couldn't detect name - unusual case
                println!(
                    "{} You are in a Git repository, but the repository name could not be determined",
                    "⚠️".bright_yellow()
                );
                println!(
                    "\n{} Run {} to register this repository",
                    "💡".bright_yellow(),
                    "imi trunk".bright_green()
                );
                return Ok(());
            }
        } else {
            // Not in a git repository - list all repositories
            return self.list_all_repositories().await;
        }
    }

    /// List all registered repositories with worktree counts
    pub async fn list_all_repositories(&self) -> Result<()> {
        let repositories = self.db.list_repositories().await?;

        if repositories.is_empty() {
            println!("\n{}", "No Registered Repositories".bright_cyan().bold());
            println!("{}", "─".repeat(80).bright_black());
            println!("\n{} No repositories registered yet", "ℹ️".bright_blue());
            println!(
                "\n{} Run {} from a git repository to register it",
                "💡".bright_yellow(),
                "imi trunk".bright_green()
            );
            return Ok(());
        }

        println!("\n{}", "Registered Repositories".bright_cyan().bold());
        println!("{}", "═".repeat(80).bright_black());

        for (i, repo) in repositories.iter().enumerate() {
            // Get worktree count for this repo
            let worktrees = self.db.list_worktrees(Some(&repo.name)).await?;
            let worktree_count = worktrees.len();

            println!(
                "\n{} {} {}",
                format!("{}.", i + 1).bright_black(),
                "📦".bright_cyan(),
                repo.name.bright_green().bold()
            );
            println!(
                "   {} Path: {}",
                "📂".bright_cyan(),
                repo.path.bright_white()
            );
            println!(
                "   {} Branch: {}",
                "🌿".bright_cyan(),
                repo.default_branch.bright_yellow()
            );

            if !repo.remote_url.is_empty() {
                println!(
                    "   {} Remote: {}",
                    "🔗".bright_cyan(),
                    repo.remote_url.bright_white()
                );
            }

            // Show worktree count with appropriate icon
            let wt_icon = if worktree_count == 0 { "📭" } else { "📬" };
            println!(
                "   {} Worktrees: {}",
                wt_icon,
                if worktree_count == 0 {
                    "None".bright_black().to_string()
                } else {
                    worktree_count.to_string().bright_white().to_string()
                }
            );

            println!(
                "   {} Created: {}",
                "📅".bright_black(),
                repo.created_at
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string()
                    .bright_green()
            );

            if i < repositories.len() - 1 {
                println!("{}", "─".repeat(80).bright_black());
            }
        }

        println!(
            "\n{} Total: {} repositories",
            "📊".bright_cyan(),
            repositories.len().to_string().bright_white().bold()
        );
        println!();

        Ok(())
    }

    /// List all worktrees with detailed metadata
    pub async fn list_worktrees_detailed(&self, repo: Option<&str>) -> Result<()> {
        let worktrees = self.db.list_worktrees(repo).await?;

        if worktrees.is_empty() {
            println!("{} No active worktrees found", "ℹ️".bright_blue());
            return Ok(());
        }

        println!(
            "\n{}",
            "Detailed Worktree Information:".bright_cyan().bold()
        );
        println!("{}", "═".repeat(100).bright_black());

        for (i, worktree) in worktrees.iter().enumerate() {
            let status_icon = match worktree.worktree_type.as_str() {
                "feat" => "🚀",
                "pr" => "🔍",
                "fix" => "🔧",
                "aiops" => "🤖",
                "devops" => "⚙️",
                "trunk" => "🌳",
                _ => "📁",
            };

            println!(
                "\n{} {} {} {} ({})",
                format!("{}.", i + 1).bright_black(),
                status_icon,
                worktree.worktree_name.bright_green().bold(),
                worktree.branch_name.bright_yellow(),
                worktree.worktree_type.bright_blue()
            );

            // Repository and path info
            println!(
                "   {} Repo: {}",
                "📦".bright_cyan(),
                worktree.repo_name.bright_white()
            );
            println!(
                "   {} Path: {}",
                "📂".bright_cyan(),
                worktree.path.bright_white()
            );

            // Timestamps
            println!(
                "   {} Created: {} | Updated: {}",
                "📅".bright_cyan(),
                worktree
                    .created_at
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string()
                    .bright_green(),
                worktree
                    .updated_at
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string()
                    .bright_yellow()
            );

            // Agent assignment
            if let Some(agent_id) = &worktree.agent_id {
                println!(
                    "   {} Agent: {}",
                    "🤖".bright_magenta(),
                    agent_id.bright_white()
                );
            } else {
                println!(
                    "   {} Agent: {}",
                    "🤖".bright_black(),
                    "Unassigned".bright_black()
                );
            }

            // Database ID for debugging
            println!(
                "   {} ID: {}",
                "🔑".bright_black(),
                worktree.id.bright_black()
            );

            // Git status if worktree exists
            let worktree_path = PathBuf::from(&worktree.path);
            if worktree_path.exists() {
                if let Ok(git_status) = self.git.get_worktree_status(&worktree_path) {
                    println!("   {} Git Status:", "📊".bright_cyan());
                    self.print_git_status_indented(&git_status);
                }
            } else {
                println!(
                    "   {} Status: {}",
                    "⚠️".bright_yellow(),
                    "Path not found".bright_red()
                );
            }

            if i < worktrees.len() - 1 {
                println!("{}", "─".repeat(100).bright_black());
            }
        }

        println!(
            "\n{} Total: {} active worktrees",
            "📊".bright_cyan(),
            worktrees.len().to_string().bright_white().bold()
        );
        Ok(())
    }

    /// Start real-time monitoring
    pub async fn start_monitoring(&self, repo: Option<&str>) -> Result<()> {
        use crate::monitor::MonitorManager;

        let monitor = MonitorManager::new(self.clone(), self.config.clone());
        monitor.start(repo).await
    }

    /// Sync database with actual Git worktrees
    pub async fn sync_with_git(&self, repo: Option<&str>) -> Result<()> {
        let current_dir = std::env::current_dir()?;
        let repo_name = if let Some(repo) = repo {
            repo.to_string()
        } else {
            self.git.get_repo_name(&current_dir)?
        };

        println!(
            "   {} Checking Git worktrees for repo: {}",
            "🔍".bright_blue(),
            repo_name
        );

        // Get actual Git worktrees
        let git_worktrees = self.git.list_git_worktrees(&current_dir)?;
        println!(
            "   {} Found {} Git worktrees",
            "📊".bright_green(),
            git_worktrees.len()
        );

        // Get database worktrees
        let db_worktrees = self.db.list_worktrees(Some(&repo_name)).await?;
        println!(
            "   {} Found {} database entries",
            "💾".bright_blue(),
            db_worktrees.len()
        );

        let mut synced = 0;
        let mut deactivated = 0;
        let mut added = 0;

        // Deactivate database entries that don't exist in Git
        for db_worktree in &db_worktrees {
            let exists_in_git = git_worktrees
                .iter()
                .any(|git_wt| PathBuf::from(&git_wt.path) == PathBuf::from(&db_worktree.path));

            if !exists_in_git {
                self.db
                    .deactivate_worktree(&repo_name, &db_worktree.worktree_name)
                    .await?;
                println!(
                    "   {} Deactivated: {}",
                    "❌".bright_red(),
                    db_worktree.worktree_name
                );
                deactivated += 1;
            } else {
                synced += 1;
            }
        }

        // Add Git worktrees that aren't in database
        for git_worktree in &git_worktrees {
            let exists_in_db = db_worktrees
                .iter()
                .any(|db_wt| PathBuf::from(&db_wt.path) == PathBuf::from(&git_worktree.path));

            if !exists_in_db {
                // Extract worktree info from Git data
                let path = PathBuf::from(&git_worktree.path);
                let worktree_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                let worktree_type = if worktree_name.starts_with("feat-") {
                    "feat"
                } else if worktree_name.starts_with("pr-") {
                    "pr"
                } else if worktree_name.starts_with("fix-") {
                    "fix"
                } else if worktree_name.starts_with("aiops-") {
                    "aiops"
                } else if worktree_name.starts_with("devops-") {
                    "devops"
                } else if worktree_name.contains("main") || worktree_name.contains("trunk") {
                    "trunk"
                } else {
                    "unknown"
                };

                self.db
                    .create_worktree(
                        &repo_name,
                        &worktree_name,
                        &git_worktree.branch,
                        worktree_type,
                        &git_worktree.path,
                        None,
                    )
                    .await?;

                println!(
                    "   {} Added: {} ({})",
                    "➕".bright_green(),
                    worktree_name,
                    worktree_type
                );
                added += 1;
            }
        }

        println!("\n{} Sync complete:", "✅".bright_green());
        println!("   {} {} entries synced", "🔄".bright_cyan(), synced);
        println!(
            "   {} {} entries deactivated",
            "❌".bright_red(),
            deactivated
        );
        println!("   {} {} entries added", "➕".bright_green(), added);

        Ok(())
    }

    /// Find a worktree in the database by name, searching across all repos if needed
    async fn find_worktree_in_database(
        &self,
        name: &str,
        repo: Option<&str>,
    ) -> Result<Option<crate::database::Worktree>> {
        // If repo is specified, search within that repo
        if let Some(repo_name) = repo {
            // Try to find with the exact name first
            if let Some(worktree) = self.db.get_worktree(repo_name, name).await? {
                return Ok(Some(worktree));
            }

            // Try different prefixed versions within the specified repo
            let possible_names = vec![
                format!("feat-{}", name),
                format!("fix-{}", name),
                format!("aiops-{}", name),
                format!("devops-{}", name),
                format!("pr-{}", name),
            ];

            for possible_name in possible_names {
                if let Some(worktree) = self.db.get_worktree(repo_name, &possible_name).await? {
                    return Ok(Some(worktree));
                }
            }

            return Ok(None);
        }

        // No repo specified - search across all repos
        // First try exact name match
        if let Some(worktree) = self.db.find_worktree_by_name(name).await? {
            return Ok(Some(worktree));
        }

        // Try different prefixed versions across all repos
        let possible_names = vec![
            format!("feat-{}", name),
            format!("fix-{}", name),
            format!("aiops-{}", name),
            format!("devops-{}", name),
            format!("pr-{}", name),
        ];

        for possible_name in possible_names {
            if let Some(worktree) = self.db.find_worktree_by_name(&possible_name).await? {
                return Ok(Some(worktree));
            }
        }

        Ok(None)
    }

    /// Find the actual worktree name by trying different prefixed versions
    async fn find_actual_worktree_name(&self, name: &str, repo_name: &str) -> Result<String> {
        // If the name is already prefixed, use it as-is
        if name.contains('-')
            && (name.starts_with("feat-")
                || name.starts_with("fix-")
                || name.starts_with("aiops-")
                || name.starts_with("devops-")
                || name.starts_with("pr-"))
        {
            return Ok(name.to_string());
        }

        // Try different prefixed versions
        let possible_names = vec![
            name.to_string(),           // Original name
            format!("feat-{}", name),   // Feature worktree
            format!("fix-{}", name),    // Fix worktree
            format!("aiops-{}", name),  // AI ops worktree
            format!("devops-{}", name), // DevOps worktree
            format!("pr-{}", name),     // PR worktree
        ];

        for possible_name in possible_names {
            if let Some(_worktree) = self.db.get_worktree(repo_name, &possible_name).await? {
                return Ok(possible_name);
            }
        }

        // If none found, return the original name (will likely fail but with better error)
        Ok(name.to_string())
    }

    /// Parse repository argument to extract org/repo pattern
    /// Supports: "name", "org/repo", or defaults org to "delorenj" if omitted
    fn parse_repo_argument(&self, repo_arg: &str) -> (Option<String>, String) {
        if repo_arg.contains('/') {
            // Format: org/repo
            let parts: Vec<&str> = repo_arg.splitn(2, '/').collect();
            (Some(parts[0].to_string()), parts[1].to_string())
        } else {
            // Just repo name - default org to delorenj
            (Some("delorenj".to_string()), repo_arg.to_string())
        }
    }

    /// Resolve repository name from current directory or provided name
    /// Handles GitHub org/repo format: searches database by remote_url pattern
    async fn resolve_repo_name(&self, repo: Option<&str>) -> Result<String> {
        if let Some(repo_arg) = repo {
            // Parse the argument to check if it's org/repo format
            let (org, repo_name) = self.parse_repo_argument(repo_arg);

            if let Some(org) = org {
                // Query database for repo matching github pattern
                let repos = self.db.list_repositories().await?;

                for db_repo in repos {
                    // Match against remote_url pattern: github.com/{org}/{repo}
                    if db_repo.remote_url.contains(&format!("{}/{}", org, repo_name)) {
                        return Ok(db_repo.name);
                    }
                }

                // Not found in database
                return Err(anyhow::anyhow!(
                    "Repository {}/{} is not registered in iMi. Run 'imi init github.com/{}/{}' first.",
                    org, repo_name, org, repo_name
                ));
            } else {
                // Plain name lookup
                return Ok(repo_arg.to_string());
            }
        }

        if let Some(repo_path) = &self.repo_path {
            if let Some(repo_name) = repo_path.file_name().and_then(|n| n.to_str()) {
                return Ok(repo_name.to_string());
            }
        }

        // Try to get repo name from current directory
        let current_dir = env::current_dir()?;

        // Check if we're in a worktree
        if let Ok(repo) = self.git.find_repository(Some(&current_dir)) {
            if let Ok(name) = self.git.get_repository_name(&repo) {
                return Ok(name);
            }
        }

        // Try to infer from directory structure - look up the directory tree
        let mut path = current_dir.as_path();
        while let Some(parent) = path.parent() {
            // Try to find a git repository in parent directories
            if let Ok(repo) = self.git.find_repository(Some(parent)) {
                if let Ok(name) = self.git.get_repository_name(&repo) {
                    return Ok(name);
                }
            }
            path = parent;
        }

        // Try to infer from directory name
        if let Some(dir_name) = current_dir.file_name() {
            if let Some(name) = dir_name.to_str() {
                // Handle worktree directory names (feat-name, pr-123, etc.)
                if let Some(_captures) = regex::Regex::new(r"^(feat|pr|fix|aiops|devops|trunk)-.*$")
                    .unwrap()
                    .captures(name)
                {
                    // Look for parent directory that might be the repo
                    if let Some(parent) = current_dir.parent() {
                        if let Some(parent_name) = parent.file_name() {
                            if let Some(repo_name) = parent_name.to_str() {
                                return Ok(repo_name.to_string());
                            }
                        }
                    }
                }
                return Ok(name.to_string());
            }
        }

        Err(anyhow::anyhow!("Could not determine repository name. Please specify with --repo or run from within a registered Git repository."))
    }

    /// Ensure repository is registered in the database
    async fn ensure_repository_registered(&self, repo_name: &str, repo_path: &Path) -> Result<()> {
        // Check if already registered
        if self.db.get_repository(repo_name).await?.is_some() {
            return Ok(());
        }

        // Get repository information
        let remote_url = self
            .git
            .get_remote_url(repo_path)
            .await
            .unwrap_or_else(|_| "".to_string());
        let default_branch = self
            .git
            .get_default_branch(repo_path)
            .await
            .unwrap_or_else(|_| "main".to_string());

        // Register the repository
        self.db
            .create_repository(
                repo_name,
                repo_path.to_str().unwrap(),
                &remote_url,
                &default_branch,
            )
            .await?;

        println!(
            "📝 Registered repository: {} at {}",
            repo_name.bright_green(),
            repo_path.display()
        );
        Ok(())
    }

    /// Prune stale worktree references and orphaned directories
    ///
    /// This method addresses the issue where manually deleted worktree directories
    /// leave behind stale Git references and database entries. The fix works in three phases:
    ///
    /// Phase 1: Git Cleanup - Force-prune Git worktree references for missing directories
    /// Phase 2: Database Sync - Deactivate database entries where filesystem paths don't exist
    /// Phase 3: Orphan Detection - Remove unregistered directories matching worktree patterns
    ///
    /// The critical fix is in Phase 1 (GitManager::prune_worktrees), which:
    /// - Compares Git's worktree list with actual filesystem state using Path::exists()
    /// - Identifies orphaned Git references (exist in Git but not on disk)
    /// - Force-removes the .git/worktrees/<name> admin directory to clean up the reference
    /// - Falls back to standard pruning for normally-prunable worktrees
    pub async fn prune_stale_worktrees(&self, repo: Option<&str>, dry_run: bool, force: bool) -> Result<()> {
        use colored::Colorize;

        let repo_name = self.resolve_repo_name(repo).await?;
        println!("{} Starting prune operation for: {}", "🧹".bright_cyan(), repo_name.bright_yellow());

        // Find the repository - must be in a valid Git repository
        let current_dir = env::current_dir()?;
        let git_repo = self.git.find_repository(Some(&current_dir))
            .context("Failed to find Git repository. Ensure you're in a repository or worktree directory.")?;

        // PHASE 1: Git State Cleanup
        // This is the CRITICAL FIX for the TASK.md issue:
        // - Iterates through Git's registered worktrees
        // - Uses Path::exists() to verify each worktree directory actually exists
        // - If directory is missing but Git reference exists (orphaned reference):
        //   * Force-removes the .git/worktrees/<name> admin directory
        //   * This allows Git to forget about the manually deleted worktree
        // - If directory exists and worktree is prunable, uses standard Git prune
        //
        // Error Handling: Gracefully handles permission errors and concurrent access
        println!("{} Phase 1: Cleaning up Git worktree references...", "🔍".bright_blue());
        self.git.prune_worktrees(&git_repo)
            .context("Failed to prune Git worktree references")?;

        // PHASE 2: Database State Cleanup
        // Synchronize database with filesystem reality:
        // - Query all database entries for this repository
        // - Verify each entry's path actually exists on disk using Path::exists()
        // - If path doesn't exist, deactivate the database entry
        // - Maintains database consistency with actual worktree state
        //
        // Transaction Safety: Each deactivation is atomic within the database layer
        println!("{} Phase 2: Synchronizing database with filesystem...", "💾".bright_blue());
        let db_worktrees = self.db.list_worktrees(Some(&repo_name)).await
            .context("Failed to list database worktrees")?;

        let mut cleaned_count = 0;
        let mut git_worktrees_set = std::collections::HashSet::new();

        // Build a set of currently valid Git worktrees for cross-reference
        if let Ok(git_worktree_names) = git_repo.worktrees() {
            for wt_name in git_worktree_names.iter().flatten() {
                git_worktrees_set.insert(wt_name.to_string());
            }
        }

        for worktree in db_worktrees {
            let worktree_path = PathBuf::from(&worktree.path);

            // Check both filesystem existence AND Git registration
            // A worktree should be deactivated if:
            // 1. The directory doesn't exist on disk, OR
            // 2. It's not registered in Git's worktree list
            let path_exists = worktree_path.exists();
            let git_registered = git_worktrees_set.contains(&worktree.worktree_name);

            if !path_exists || !git_registered {
                let reason = if !path_exists && !git_registered {
                    "path missing and not in Git"
                } else if !path_exists {
                    "path missing"
                } else {
                    "not in Git"
                };

                // Deactivate the database entry to maintain consistency
                self.db
                    .deactivate_worktree(&repo_name, &worktree.worktree_name)
                    .await
                    .context(format!("Failed to deactivate worktree: {}", worktree.worktree_name))?;

                println!(
                    "   {} Deactivated database entry: {} ({})",
                    "🗑️".bright_red(),
                    worktree.worktree_name.bright_yellow(),
                    reason.bright_black()
                );
                cleaned_count += 1;
            }
        }

        if cleaned_count > 0 {
            println!("{} Cleaned {} stale database entries", "✅".bright_green(), cleaned_count);
        } else {
            println!("{} No stale database entries found", "ℹ️".bright_blue());
        }

        // PHASE 3: Orphaned Directory Cleanup
        // Detect and remove directories that:
        // - Match worktree naming patterns (feat-, fix-, aiops-, devops-, review-)
        // - Are NOT registered in Git as worktrees
        // - Are NOT valid Git repositories themselves
        //
        // These are "orphaned directories" - leftover filesystem cruft from failed operations
        // or manual deletions where the Git reference was already cleaned but directory remains
        //
        // Safety: Requires confirmation unless --force flag is used
        //         Respects --dry-run to preview without deleting
        println!("{} Phase 3: Detecting orphaned worktree directories...", "📦".bright_blue());
        self.prune_orphaned_directories(&git_repo, dry_run, force).await
            .context("Failed to prune orphaned directories")?;

        println!("{} Prune operation completed successfully", "✅".bright_green().bold());
        Ok(())
    }

    /// Detect and remove orphaned worktree directories
    async fn prune_orphaned_directories(&self, git_repo: &git2::Repository, dry_run: bool, force: bool) -> Result<()> {
        // Get the parent directory where worktrees live
        // git_repo.path() returns path to .git directory
        // We want the parent of the trunk directory (where worktrees are siblings to trunk)
        let worktree_root = git_repo.path()
            .parent()  // trunk-main/
            .and_then(|p| p.parent())  // parent containing trunk-main and worktrees
            .context("Failed to determine worktree root directory")?;

        // Get list of currently registered worktrees from git
        let registered_worktrees: Vec<String> = git_repo.worktrees()?
            .iter()
            .flatten()
            .map(|s| s.to_string())
            .collect();

        // Scan parent directory for potential orphaned directories
        let mut orphaned_dirs = Vec::new();
        let mut entries = async_fs::read_dir(worktree_root).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            // Skip if not a directory
            if !path.is_dir() {
                continue;
            }

            let dir_name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");

            // Skip hidden directories and trunk
            if dir_name.starts_with('.') || dir_name.starts_with("trunk-") {
                continue;
            }

            // Check if matches worktree naming pattern
            let matches_pattern = dir_name.starts_with("feat-")
                || dir_name.starts_with("fix-")
                || dir_name.starts_with("aiops-")
                || dir_name.starts_with("devops-")
                || dir_name.starts_with("review-");

            if !matches_pattern {
                continue;
            }

            // Check if registered as a worktree
            let is_registered = registered_worktrees.iter().any(|wt| wt == dir_name);

            if is_registered {
                continue;
            }

            // Check if it's a valid git repository
            let is_valid_repo = git2::Repository::open(&path).is_ok();

            if is_valid_repo {
                // Valid repo but not registered - could be manually created
                continue;
            }

            // This is an orphaned directory - collect info
            let size = self.get_directory_size(&path).await?;
            orphaned_dirs.push((path.clone(), dir_name.to_string(), size));
        }

        if orphaned_dirs.is_empty() {
            return Ok(());
        }

        // Display orphaned directories
        println!("\n{} Found {} orphaned worktree directories:",
            "📦".bright_yellow(),
            orphaned_dirs.len()
        );

        let mut total_size = 0u64;
        for (_, name, size) in &orphaned_dirs {
            println!("  {} {} ({})",
                "•".bright_yellow(),
                name.bright_white(),
                self.format_size(*size).bright_cyan()
            );
            total_size += size;
        }

        println!("\n{} Total size: {}",
            "💾".bright_cyan(),
            self.format_size(total_size).bright_yellow()
        );

        if dry_run {
            println!("\n{} Dry run - no directories removed", "ℹ️".bright_blue());
            return Ok(());
        }

        // Ask for confirmation unless force flag is set
        let should_remove = if force {
            true
        } else {
            Confirm::new()
                .with_prompt("Remove these orphaned directories?")
                .default(false)
                .interact()?
        };

        if !should_remove {
            println!("{} Skipping removal", "⏭️".bright_yellow());
            return Ok(());
        }

        // Remove orphaned directories
        let mut removed_count = 0;
        for (path, name, _) in orphaned_dirs {
            match async_fs::remove_dir_all(&path).await {
                Ok(_) => {
                    println!("🗑️ Removed: {}", name.bright_green());
                    removed_count += 1;
                }
                Err(e) => {
                    println!("❌ Failed to remove {}: {}", name.bright_red(), e);
                }
            }
        }

        if removed_count > 0 {
            println!("\n{} Removed {} orphaned directories",
                "✅".bright_green(),
                removed_count
            );
        }

        Ok(())
    }

    /// Calculate directory size recursively
    fn get_directory_size<'a>(&'a self, path: &'a Path) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<u64>> + 'a>> {
        Box::pin(async move {
            let mut total_size = 0u64;
            let mut entries = async_fs::read_dir(path).await?;

            while let Some(entry) = entries.next_entry().await? {
                let entry_path = entry.path();
                let metadata = async_fs::metadata(&entry_path).await?;

                if metadata.is_file() {
                    total_size += metadata.len();
                } else if metadata.is_dir() {
                    total_size += self.get_directory_size(&entry_path).await?;
                }
            }

            Ok(total_size)
        })
    }

    /// Format byte size to human-readable string
    fn format_size(&self, bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut unit_idx = 0;

        while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
            size /= 1024.0;
            unit_idx += 1;
        }

        format!("{:.2} {}", size, UNITS[unit_idx])
    }

    /// Fuzzy navigate to a worktree or repository
    pub async fn fuzzy_navigate(
        &self,
        query: Option<&str>,
        repo: Option<&str>,
        worktrees_only: bool,
        include_inactive: bool,
    ) -> Result<PathBuf> {
        let matcher = FuzzyMatcher::new(self.db.clone());

        let selected = if let Some(query_str) = query {
            // Perform fuzzy search
            let results = matcher
                .search(query_str, repo, worktrees_only, include_inactive)
                .await?;

            if results.is_empty() {
                return Err(anyhow::anyhow!(
                    "No worktrees or repositories found matching '{}'",
                    query_str
                ));
            }

            // Auto-select top result if score is very high (exact/near match)
            if results[0].score() >= 0.8 {
                eprintln!(
                    "{} Selected: {}",
                    "✅".bright_green(),
                    results[0].display_name()
                );
                results[0].clone()
            } else if results.len() == 1 {
                // Only one result, auto-select it
                eprintln!(
                    "{} Selected: {}",
                    "✅".bright_green(),
                    results[0].display_name()
                );
                results[0].clone()
            } else {
                // Multiple ambiguous results - show picker
                eprintln!(
                    "{} Multiple matches found for '{}':",
                    "🔍".bright_yellow(),
                    query_str
                );

                use dialoguer::{theme::ColorfulTheme, Select};

                let display_items: Vec<String> = results
                    .iter()
                    .map(|target| {
                        let icon = match target.worktree_type() {
                            Some("feat") => "🚀",
                            Some("pr") => "🔍",
                            Some("fix") => "🔧",
                            Some("aiops") => "🤖",
                            Some("devops") => "⚙️",
                            Some("trunk") => "🌳",
                            _ => "📁",
                        };
                        format!(
                            "{} {} ({}) - score: {:.2}",
                            icon,
                            target.display_name(),
                            target.repo_name(),
                            target.score()
                        )
                    })
                    .collect();

                let selection = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Select a target")
                    .items(&display_items)
                    .default(0)
                    .interact()?;

                results[selection].clone()
            }
        } else {
            // No query - show interactive picker
            if let Some(target) = matcher.interactive_select(repo, worktrees_only).await? {
                target
            } else {
                return Err(anyhow::anyhow!("No worktrees or repositories available"));
            }
        };

        Ok(selected.path())
    }

    /// Detect IMI_PATH based on repository structure
    /// If we're in a trunk directory (trunk-*), return its parent
    /// Otherwise, return the repository root's parent
    fn detect_imi_path(&self, repo_root: &Path) -> Result<PathBuf> {
        // Check if the current repo_root is a trunk directory
        if let Some(dir_name) = repo_root.file_name() {
            if let Some(name) = dir_name.to_str() {
                // Check if this is a trunk directory (pattern: trunk-*)
                if name.starts_with("trunk-") {
                    // This is a trunk directory, so its parent is the IMI_PATH
                    if let Some(parent) = repo_root.parent() {
                        return Ok(parent.to_path_buf());
                    }
                }
            }
        }

        // Fall back to repository root's parent (original behavior)
        let imi_path = repo_root.parent().unwrap_or(repo_root);
        Ok(imi_path.to_path_buf())
    }

    /// Merge a worktree into trunk-main and close it
    pub async fn merge_worktree(&self, name: &str, repo: Option<&str>) -> Result<()> {
        let repo_name = self.resolve_repo_name(repo).await?;
        let actual_worktree_name = self.find_actual_worktree_name(name, &repo_name).await?;

        println!("{} Merging worktree: {}", "🔀".bright_cyan(), actual_worktree_name.bright_yellow());

        let worktree_info = self.db.get_worktree(&repo_name, &actual_worktree_name).await?
            .ok_or_else(|| anyhow::anyhow!("Worktree '{}' not found", actual_worktree_name))?;

        let branch_name = worktree_info.branch_name.clone();
        let trunk_path = self.get_trunk_worktree(repo).await?;

        println!("{} Switching to trunk: {}", "🌳".bright_green(), trunk_path.display());

        let trunk_repo = self.git.find_repository(Some(&trunk_path))?;
        println!("{} Fetching latest changes", "⬇️".bright_blue());
        self.git.fetch_all(&trunk_repo)?;

        let default_branch = self.config.git_settings.default_branch.clone();
        let current_branch = self.git.get_current_branch(&trunk_path)?;

        if current_branch != default_branch {
            return Err(anyhow::anyhow!(
                "Trunk is on branch '{}' instead of '{}'. Please checkout '{}' first.",
                current_branch, default_branch, default_branch
            ));
        }

        let worktree_path = PathBuf::from(&worktree_info.path);
        if worktree_path.exists() {
            let worktree_status = self.git.get_worktree_status(&worktree_path)?;
            if !worktree_status.clean {
                let mut error_msg = String::from("Worktree has uncommitted changes. Please commit or stash them first.\n");

                if !worktree_status.modified_files.is_empty() {
                    error_msg.push_str(&format!("\nModified files ({}):\n", worktree_status.modified_files.len()));
                    for file in &worktree_status.modified_files {
                        error_msg.push_str(&format!("  - {}\n", file));
                    }
                }

                if !worktree_status.new_files.is_empty() {
                    error_msg.push_str(&format!("\nNew files ({}):\n", worktree_status.new_files.len()));
                    for file in &worktree_status.new_files {
                        error_msg.push_str(&format!("  - {}\n", file));
                    }
                }

                if !worktree_status.deleted_files.is_empty() {
                    error_msg.push_str(&format!("\nDeleted files ({}):\n", worktree_status.deleted_files.len()));
                    for file in &worktree_status.deleted_files {
                        error_msg.push_str(&format!("  - {}\n", file));
                    }
                }

                return Err(anyhow::anyhow!(error_msg));
            }
        }

        println!("{} Merging branch '{}' into '{}'", "🔀".bright_magenta(), branch_name.bright_yellow(), default_branch.bright_green());

        self.git.merge_branch(&trunk_repo, &branch_name, &default_branch)
            .context("Failed to merge branch into trunk")?;

        println!("{} Pushing merged changes to remote", "⬆️".bright_cyan());

        match self.git.push_to_remote(&trunk_repo, &default_branch) {
            Ok(_) => println!("{} Changes pushed to remote", "✅".bright_green()),
            Err(e) => {
                println!("{} Warning: Failed to push to remote: {}", "⚠️".bright_yellow(), e);
                println!("   You may need to push manually: cd {} && git push", trunk_path.display());
            }
        }

        println!("{} Closing worktree: {}", "🧹".bright_cyan(), actual_worktree_name);
        self.close_worktree(name, repo).await?;

        println!("{} Deleting merged branch: {}", "🗑️".bright_red(), branch_name);
        self.git.delete_local_branch(&trunk_repo, &branch_name)?;

        match self.git.delete_remote_branch(&trunk_repo, &branch_name).await {
            Ok(_) => println!("{} Remote branch deleted", "✅".bright_green()),
            Err(e) => {
                println!("{} Warning: Could not delete remote branch '{}': {}", "⚠️".bright_yellow(), branch_name, e);
                println!("   (This is normal if the branch was already deleted or never pushed)");
            }
        }

        println!("\n{} Merge completed successfully!", "✅".bright_green().bold());
        println!("{} Branch '{}' has been merged into '{}' and cleaned up", "📝".bright_blue(), branch_name.bright_yellow(), default_branch.bright_green());

        Ok(())
    }
}
