use anyhow::{Context, Result};
use colored::*;
use std::env;
use std::os::unix::fs;
use std::path::{Path, PathBuf};
use tokio::fs as async_fs;

use crate::config::Config;
use crate::database::Database;
use crate::error::ImiError;
use crate::git::{GitManager, WorktreeStatus};

#[derive(Debug, Clone)]
pub struct WorktreeManager {
    pub git: GitManager,
    pub db: Database,
    pub config: Config,
}

impl WorktreeManager {
    pub fn new(git: GitManager, db: Database, config: Config) -> Self {
        Self { git, db, config }
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
        let worktree_name = format!("pr-{}", pr_number);
        let branch_name = format!("pr/{}", pr_number);

        // Try to use gh CLI for PR checkout
        if let Ok(path) = self.create_pr_worktree_with_gh(pr_number, repo).await {
            return Ok(path);
        }

        // Fallback to manual creation
        self.create_worktree_internal(
            repo,
            &worktree_name,
            &branch_name,
            "pr",
            Some(&self.config.git_settings.default_branch),
        )
        .await
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

        // Use IMI_PATH detection for consistent worktree placement
        let current_dir = env::current_dir()?;
        let repo = self.git.find_repository(Some(&current_dir))?;
        let repo_root = repo
            .workdir()
            .ok_or_else(|| anyhow::anyhow!("Repository has no working directory"))?;
        let imi_path = self.detect_imi_path(repo_root)?;
        let worktree_path = imi_path.join(&worktree_name);

        let trunk_path = self.config.get_trunk_path(&repo_name);

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
        let global_sync = self.config.get_sync_path(repo_name, true);
        let repo_sync = self.config.get_sync_path(repo_name, false);

        // Create sync/global directory
        async_fs::create_dir_all(&global_sync)
            .await
            .context("Failed to create global sync directory")?;

        // Create sync/repo directory
        async_fs::create_dir_all(&repo_sync)
            .await
            .context("Failed to create repo sync directory")?;

        // Create default sync files if they don't exist
        let coding_rules = global_sync.join("coding-rules.md");
        if !coding_rules.exists() {
            async_fs::write(
                &coding_rules,
                "# Coding Rules\n\n## Style Guidelines\n\n## Best Practices\n",
            )
            .await?;
        }

        let stack_specific = global_sync.join("stack-specific.md");
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
        let repo_sync = self.config.get_sync_path(repo_name, false);

        for file_name in &self.config.symlink_files {
            let source = repo_sync.join(file_name);
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
        let repo_name = self.resolve_repo_name(repo).await?;

        // Find the actual worktree name - it might be prefixed (e.g., feat-interactive-learning)
        let actual_worktree_name = self.find_actual_worktree_name(name, &repo_name).await?;

        // Use IMI_PATH detection for consistent worktree removal
        let current_dir = env::current_dir()?;
        let repo = self.git.find_repository(Some(&current_dir))?;
        let repo_root = repo
            .workdir()
            .ok_or_else(|| anyhow::anyhow!("Repository has no working directory"))?;
        let imi_path = self.detect_imi_path(repo_root)?;
        let worktree_path = imi_path.join(&actual_worktree_name);

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

    /// List all worktrees with detailed metadata
    pub async fn list_worktrees(&self, repo: Option<&str>) -> Result<()> {
        let worktrees = self.db.list_worktrees(repo).await?;

        if worktrees.is_empty() {
            println!("{} No active worktrees found", "ℹ️".bright_blue());
            return Ok(());
        }

        println!("\n{}", "Detailed Worktree Information:".bright_cyan().bold());
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
            println!("   {} Repo: {}", "📦".bright_cyan(), worktree.repo_name.bright_white());
            println!("   {} Path: {}", "📂".bright_cyan(), worktree.path.bright_white());

            // Timestamps
            println!(
                "   {} Created: {} | Updated: {}",
                "📅".bright_cyan(),
                worktree.created_at.format("%Y-%m-%d %H:%M:%S").to_string().bright_green(),
                worktree.updated_at.format("%Y-%m-%d %H:%M:%S").to_string().bright_yellow()
            );

            // Agent assignment
            if let Some(agent_id) = &worktree.agent_id {
                println!("   {} Agent: {}", "🤖".bright_magenta(), agent_id.bright_white());
            } else {
                println!("   {} Agent: {}", "🤖".bright_black(), "Unassigned".bright_black());
            }

            // Database ID for debugging
            println!("   {} ID: {}", "🔑".bright_black(), worktree.id.bright_black());

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

        println!("\n{} Total: {} active worktrees", "📊".bright_cyan(), worktrees.len().to_string().bright_white().bold());
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

        println!("   {} Checking Git worktrees for repo: {}", "🔍".bright_blue(), repo_name);
        
        // Get actual Git worktrees
        let git_worktrees = self.git.list_git_worktrees(&current_dir)?;
        println!("   {} Found {} Git worktrees", "📊".bright_green(), git_worktrees.len());

        // Get database worktrees
        let db_worktrees = self.db.list_worktrees(Some(&repo_name)).await?;
        println!("   {} Found {} database entries", "💾".bright_blue(), db_worktrees.len());

        let mut synced = 0;
        let mut deactivated = 0;
        let mut added = 0;

        // Deactivate database entries that don't exist in Git
        for db_worktree in &db_worktrees {
            let exists_in_git = git_worktrees.iter().any(|git_wt| {
                PathBuf::from(&git_wt.path) == PathBuf::from(&db_worktree.path)
            });
            
            if !exists_in_git {
                self.db.deactivate_worktree(&repo_name, &db_worktree.worktree_name).await?;
                println!("   {} Deactivated: {}", "❌".bright_red(), db_worktree.worktree_name);
                deactivated += 1;
            } else {
                synced += 1;
            }
        }

        // Add Git worktrees that aren't in database
        for git_worktree in &git_worktrees {
            let exists_in_db = db_worktrees.iter().any(|db_wt| {
                PathBuf::from(&db_wt.path) == PathBuf::from(&git_worktree.path)
            });

            if !exists_in_db {
                // Extract worktree info from Git data
                let path = PathBuf::from(&git_worktree.path);
                let worktree_name = path.file_name()
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

                self.db.create_worktree(
                    &repo_name,
                    &worktree_name,
                    &git_worktree.branch,
                    worktree_type,
                    &git_worktree.path,
                    None,
                ).await?;
                
                println!("   {} Added: {} ({})", "➕".bright_green(), worktree_name, worktree_type);
                added += 1;
            }
        }

        println!("\n{} Sync complete:", "✅".bright_green());
        println!("   {} {} entries synced", "🔄".bright_cyan(), synced);
        println!("   {} {} entries deactivated", "❌".bright_red(), deactivated);
        println!("   {} {} entries added", "➕".bright_green(), added);

        Ok(())
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

    /// Resolve repository name from current directory or provided name
    async fn resolve_repo_name(&self, repo: Option<&str>) -> Result<String> {
        if let Some(name) = repo {
            return Ok(name.to_string());
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

    /// Prune stale worktree references
    pub async fn prune_stale_worktrees(&self, repo: Option<&str>) -> Result<()> {
        let repo_name = self.resolve_repo_name(repo).await?;

        // Find the repository
        let current_dir = env::current_dir()?;
        let git_repo = self.git.find_repository(Some(&current_dir))?;

        // Prune stale worktrees using Git manager
        self.git.prune_worktrees(&git_repo)?;

        // Also clean up database entries for worktrees that no longer exist
        let db_worktrees = self.db.list_worktrees(Some(&repo_name)).await?;
        let mut cleaned_count = 0;

        for worktree in db_worktrees {
            let worktree_path = PathBuf::from(&worktree.path);
            if !worktree_path.exists() {
                self.db
                    .deactivate_worktree(&repo_name, &worktree.worktree_name)
                    .await?;
                println!(
                    "🗑️ Cleaned up database entry for: {}",
                    worktree.worktree_name
                );
                cleaned_count += 1;
            }
        }

        if cleaned_count > 0 {
            println!("📊 Cleaned up {} stale database entries", cleaned_count);
        }

        Ok(())
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
}
