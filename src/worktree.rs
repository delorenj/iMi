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
        ).await
    }
    
    /// Create a review worktree for a PR
    pub async fn create_review_worktree(&self, pr_number: u32, repo: Option<&str>) -> Result<PathBuf> {
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
        ).await
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
        ).await
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
        ).await
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
        ).await
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
        let worktree_path = self.config.get_worktree_path(&repo_name, worktree_name);
        
        // Check if worktree already exists
        if let Some(existing) = self.db.get_worktree(&repo_name, worktree_name).await? {
            if worktree_path.exists() {
                println!("{} Worktree already exists: {}", "ℹ️".bright_blue(), worktree_path.display());
                return Ok(worktree_path);
            } else {
                // Clean up stale database entry
                self.db.deactivate_worktree(&repo_name, worktree_name).await?;
            }
        }
        
        // Find the repository
        let trunk_path = self.config.get_trunk_path(&repo_name);
        let repo = self.git.find_repository(Some(&trunk_path))?;
        
        // Create the worktree directory
        async_fs::create_dir_all(&worktree_path).await
            .context("Failed to create worktree directory")?;
        
        // Create the Git worktree
        self.git.create_worktree(&repo, worktree_name, &worktree_path, branch_name, base_branch)
            .context("Failed to create Git worktree")?;
        
        // Create sync directories
        self.create_sync_directories(&repo_name).await?;
        
        // Create symlinks for dotfiles
        self.create_symlinks(&repo_name, &worktree_path).await?;
        
        // Record the worktree in the database
        self.db.create_worktree(
            &repo_name,
            worktree_name,
            branch_name,
            worktree_type,
            worktree_path.to_str().unwrap(),
            None, // agent_id will be set later if needed
        ).await?;
        
        println!("{} Worktree created successfully", "✅".bright_green());
        
        Ok(worktree_path)
    }
    
    /// Create PR worktree using gh CLI
    async fn create_pr_worktree_with_gh(&self, pr_number: u32, repo: Option<&str>) -> Result<PathBuf> {
        let repo_name = self.resolve_repo_name(repo).await?;
        let worktree_name = format!("pr-{}", pr_number);
        let worktree_path = self.config.get_worktree_path(&repo_name, &worktree_name);
        let trunk_path = self.config.get_trunk_path(&repo_name);
        
        // Try to checkout PR using gh CLI
        let repo = self.git.find_repository(Some(&trunk_path))?;
        self.git.checkout_pr(&trunk_path, pr_number, &worktree_path)?;
        
        // Create sync directories and symlinks
        self.create_sync_directories(&repo_name).await?;
        self.create_symlinks(&repo_name, &worktree_path).await?;
        
        // Get the actual branch name from the checked out PR
        let branch_name = self.git.get_current_branch(&worktree_path)
            .unwrap_or_else(|_| format!("pr/{}", pr_number));
        
        // Record in database
        self.db.create_worktree(
            &repo_name,
            &worktree_name,
            &branch_name,
            "pr",
            worktree_path.to_str().unwrap(),
            None,
        ).await?;
        
        Ok(worktree_path)
    }
    
    /// Create sync directories as per PRD specifications
    async fn create_sync_directories(&self, repo_name: &str) -> Result<()> {
        let global_sync = self.config.get_sync_path(repo_name, true);
        let repo_sync = self.config.get_sync_path(repo_name, false);
        
        // Create sync/global directory
        async_fs::create_dir_all(&global_sync).await
            .context("Failed to create global sync directory")?;
        
        // Create sync/repo directory  
        async_fs::create_dir_all(&repo_sync).await
            .context("Failed to create repo sync directory")?;
        
        // Create default sync files if they don't exist
        let coding_rules = global_sync.join("coding-rules.md");
        if !coding_rules.exists() {
            async_fs::write(&coding_rules, "# Coding Rules\n\n## Style Guidelines\n\n## Best Practices\n").await?;
        }
        
        let stack_specific = global_sync.join("stack-specific.md");
        if !stack_specific.exists() {
            async_fs::write(&stack_specific, "# Stack-Specific Guidelines\n\n## Frontend\n\n## Backend\n\n## Database\n").await?;
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
                fs::symlink(&source, &target).map_err(|e| {
                    ImiError::SymlinkCreationFailed {
                        source: source.display().to_string(),
                        target: target.display().to_string(),
                        io_error: e,
                    }
                })?;
                
                println!("{} Created symlink: {} -> {}", 
                    "🔗".bright_cyan(), 
                    target.display(), 
                    source.display()
                );
            }
        }
        
        Ok(())
    }
    
    /// Remove a worktree
    pub async fn remove_worktree(&self, name: &str, repo: Option<&str>) -> Result<()> {
        let repo_name = self.resolve_repo_name(repo).await?;
        let worktree_path = self.config.get_worktree_path(&repo_name, name);
        
        // Find the repository
        let trunk_path = self.config.get_trunk_path(&repo_name);
        let repo = self.git.find_repository(Some(&trunk_path))?;
        
        // Remove from Git
        if self.git.worktree_exists(&repo, name) {
            self.git.remove_worktree(&repo, name)?;
        }
        
        // Remove directory
        if worktree_path.exists() {
            async_fs::remove_dir_all(&worktree_path).await
                .context("Failed to remove worktree directory")?;
        }
        
        // Deactivate in database
        self.db.deactivate_worktree(&repo_name, name).await?;
        
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
            
            println!("{} {} {} ({})", 
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
                println!("   {} Path not found: {}", "⚠️".bright_yellow(), worktree.path);
            }
            
            if let Some(agent_id) = &worktree.agent_id {
                println!("   {} Agent: {}", "🤖".bright_magenta(), agent_id);
            }
            
            println!("   {} Created: {}", "📅".bright_black(), 
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
                println!("   {} Modified: {}", "📝".bright_yellow(), status.modified_files.len());
            }
            if !status.new_files.is_empty() {
                println!("   {} New files: {}", "➕".bright_green(), status.new_files.len());
            }
            if !status.deleted_files.is_empty() {
                println!("   {} Deleted: {}", "➖".bright_red(), status.deleted_files.len());
            }
        }
        
        if status.commits_ahead > 0 {
            println!("   {} {} commits ahead", "⬆️".bright_green(), status.commits_ahead);
        }
        if status.commits_behind > 0 {
            println!("   {} {} commits behind", "⬇️".bright_red(), status.commits_behind);
        }
    }
    
    /// List all worktrees
    pub async fn list_worktrees(&self, repo: Option<&str>) -> Result<()> {
        self.show_status(repo).await
    }
    
    /// Start real-time monitoring
    pub async fn start_monitoring(&self, repo: Option<&str>) -> Result<()> {
        use crate::monitor::MonitorManager;
        
        let monitor = MonitorManager::new(self.clone());
        monitor.start(repo).await
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
            return self.git.get_repository_name(&repo);
        }
        
        // Try to infer from directory name
        if let Some(dir_name) = current_dir.file_name() {
            if let Some(name) = dir_name.to_str() {
                // Handle worktree directory names (feat-name, pr-123, etc.)
                if let Some(captures) = regex::Regex::new(r"^(feat|pr|fix|aiops|devops|trunk)-.*$")
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
        
        Err(anyhow::anyhow!("Could not determine repository name. Please specify with --repo or run from within a Git repository."))
    }
}