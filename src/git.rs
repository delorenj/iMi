use anyhow::{Context, Result};
use git2::{BranchType, Repository, WorktreeAddOptions};
use std::path::Path;
use std::process::Command;

use crate::error::ImiError;

#[derive(Debug, Clone)]
pub struct GitManager;

impl GitManager {
    pub fn new() -> Self {
        Self
    }

    pub fn is_in_repository(&self, path: &Path) -> bool {
        Repository::discover(path).is_ok()
    }

    pub async fn get_remote_url(&self, path: &Path) -> Result<String> {
        let repo = self.find_repository(Some(path))?;
        let remote = repo.find_remote("origin")?;
        Ok(remote.url().unwrap_or_default().to_string())
    }

    pub async fn get_default_branch(&self, path: &Path) -> Result<String> {
        let repo = self.find_repository(Some(path))?;
        let head = repo.head()?;
        let head_name = head.shorthand().unwrap_or("main");
        Ok(head_name.to_string())
    }

    /// Find the Git repository from the current directory or a specified path
    pub fn find_repository(&self, path: Option<&Path>) -> Result<Repository> {
        let search_path = path.unwrap_or_else(|| Path::new("."));

        Repository::discover(search_path).map_err(|_e| {
            ImiError::GitRepositoryNotFound {
                path: search_path.display().to_string(),
            }
            .into()
        })
    }

    /// Get the repository name from the remote URL
    pub fn get_repository_name(&self, repo: &Repository) -> Result<String> {
        let remote = repo
            .find_remote("origin")
            .or_else(|_| {
                repo.remotes()?
                    .get(0)
                    .ok_or(git2::Error::from_str("No remotes found"))
                    .and_then(|name| repo.find_remote(name))
            })
            .context("No suitable remote found")?;

        let url = remote.url().context("Remote URL not found")?;

        // Extract repo name from URL (handles both SSH and HTTPS)
        let name = url
            .split('/')
            .last()
            .context("Could not extract repository name from URL")?
            .trim_end_matches(".git");

        Ok(name.to_string())
    }

    /// Create a new worktree
    pub fn create_worktree(
        &self,
        repo: &Repository,
        name: &str,
        path: &Path,
        branch: &str,
        base_branch: Option<&str>,
    ) -> Result<()> {
        // Ensure we have the latest changes from remote
        self.fetch_all(repo)?;

        // Create the branch if it doesn't exist
        let base = if let Some(base_ref) = base_branch {
            format!("origin/{}", base_ref)
        } else {
            "HEAD".to_string()
        };

        // Check if branch already exists locally
        let branch_exists = repo.find_branch(branch, BranchType::Local).is_ok();

        if !branch_exists {
            // Create new branch from base
            let base_commit = repo.revparse_single(&base)?.peel_to_commit()?;
            repo.branch(branch, &base_commit, false)?;
        }

        // Clean up any existing worktree artifacts before creation
        self.cleanup_worktree_artifacts(repo, name, path)?;

        // Add the worktree
        let mut options = WorktreeAddOptions::new();
        let worktree = repo.worktree(name, path, Some(&mut options))?;

        // Open the worktree repository to set up the branch
        let worktree_repo = Repository::open_from_worktree(&worktree)?;

        // Checkout the branch in the worktree
        let branch_ref = worktree_repo.find_branch(branch, BranchType::Local)?;
        let _branch_commit = branch_ref.get().peel_to_commit()?;
        worktree_repo.set_head(&format!("refs/heads/{}", branch))?;
        worktree_repo.checkout_head(Some(
            git2::build::CheckoutBuilder::new()
                .force()
                .remove_untracked(true),
        ))?;

        Ok(())
    }

    /// Remove a worktree
    pub fn remove_worktree(&self, repo: &Repository, name: &str) -> Result<()> {
        if let Ok(worktree) = repo.find_worktree(name) {
            // First, try to prune the worktree (removes it from Git's tracking)
            if worktree.is_prunable(None)? {
                worktree.prune(None)?;
            }
        }

        Ok(())
    }

    /// Clean up any existing worktree files and directories before creation
    pub fn cleanup_worktree_artifacts(&self, repo: &Repository, name: &str, path: &Path) -> Result<()> {
        println!("🧹 Cleaning up worktree artifacts for: {}", name);
        
        // Remove the git worktree entry if it exists
        if let Ok(worktree) = repo.find_worktree(name) {
            println!("📝 Found existing git worktree entry, removing...");
            if worktree.is_prunable(None)? {
                worktree.prune(None)?;
                println!("✅ Git worktree entry pruned");
            }
        }

        // Remove the filesystem directory if it exists
        if path.exists() {
            println!("📁 Removing filesystem directory: {}", path.display());
            std::fs::remove_dir_all(path)
                .context("Failed to remove existing worktree directory")?;
            println!("✅ Filesystem directory removed");
        }

        // Remove any stale git worktree administrative directories
        let git_dir = repo.path();
        let worktree_admin_dir = git_dir.join("worktrees").join(name);
        if worktree_admin_dir.exists() {
            println!("⚙️ Removing git admin directory: {}", worktree_admin_dir.display());
            std::fs::remove_dir_all(worktree_admin_dir)
                .context("Failed to remove git worktree admin directory")?;
            println!("✅ Git admin directory removed");
        }

        println!("🎯 Cleanup complete for: {}", name);
        Ok(())
    }

    /// List all worktrees for a repository
    #[allow(dead_code)]
    pub fn list_worktrees(&self, repo: &Repository) -> Result<Vec<String>> {
        let worktrees = repo.worktrees()?;
        let mut result = Vec::new();

        for name in worktrees.iter() {
            if let Some(name_str) = name {
                result.push(name_str.to_string());
            }
        }

        Ok(result)
    }

    /// Check if a worktree exists
    pub fn worktree_exists(&self, repo: &Repository, name: &str) -> bool {
        repo.find_worktree(name).is_ok()
    }

    /// Fetch all remotes
    pub fn fetch_all(&self, repo: &Repository) -> Result<()> {
        let mut remote = repo.find_remote("origin")?;
        let refspecs = remote.fetch_refspecs()?;
        let refspecs: Vec<&str> = refspecs.iter().filter_map(|s| s).collect();

        remote.fetch(&refspecs, None, None)?;
        Ok(())
    }

    /// Check if a branch exists (local or remote)
    #[allow(dead_code)]
    pub fn branch_exists(&self, repo: &Repository, branch_name: &str) -> bool {
        repo.find_branch(branch_name, BranchType::Local).is_ok()
            || repo
                .find_branch(&format!("origin/{}", branch_name), BranchType::Remote)
                .is_ok()
    }

    /// Get the current branch name for a worktree
    pub fn get_current_branch(&self, repo_path: &Path) -> Result<String> {
        let repo = Repository::open(repo_path)?;
        let head = repo.head()?;

        if let Some(branch_name) = head.shorthand() {
            Ok(branch_name.to_string())
        } else {
            Err(anyhow::anyhow!("Could not determine current branch"))
        }
    }

    /// Get worktree status (modified files, commits ahead/behind, etc.)
    pub fn get_worktree_status(&self, repo_path: &Path) -> Result<WorktreeStatus> {
        let repo = Repository::open(repo_path)?;
        let statuses = repo.statuses(None)?;

        let mut modified_files = Vec::new();
        let mut new_files = Vec::new();
        let mut deleted_files = Vec::new();

        for status in statuses.iter() {
            let file_path = status.path().unwrap_or("").to_string();
            let status_flags = status.status();

            if status_flags.is_wt_modified() || status_flags.is_index_modified() {
                modified_files.push(file_path);
            } else if status_flags.is_wt_new() || status_flags.is_index_new() {
                new_files.push(file_path);
            } else if status_flags.is_wt_deleted() || status_flags.is_index_deleted() {
                deleted_files.push(file_path);
            }
        }

        // Get commits ahead/behind info
        let (ahead, behind) = self.get_ahead_behind(&repo)?;

        Ok(WorktreeStatus {
            modified_files,
            new_files,
            deleted_files,
            commits_ahead: ahead,
            commits_behind: behind,
            clean: statuses.is_empty(),
        })
    }

    /// Get commits ahead/behind compared to upstream
    fn get_ahead_behind(&self, repo: &Repository) -> Result<(usize, usize)> {
        let head = repo.head()?;
        let head_oid = head.target().context("HEAD has no target")?;

        // Try to find upstream branch
        if let Ok(branch) = repo.find_branch(&head.shorthand().unwrap_or("HEAD"), BranchType::Local)
        {
            if let Ok(upstream) = branch.upstream() {
                let upstream_oid = upstream.get().target().context("Upstream has no target")?;
                let (ahead, behind) = repo.graph_ahead_behind(head_oid, upstream_oid)?;
                return Ok((ahead, behind));
            }
        }

        Ok((0, 0))
    }

    /// Execute git command using system git (for operations not available in git2)
    pub fn execute_git_command(&self, repo_path: &Path, args: &[&str]) -> Result<String> {
        let output = Command::new("git")
            .current_dir(repo_path)
            .args(args)
            .output()
            .context("Failed to execute git command")?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow::anyhow!("Git command failed: {}", stderr))
        }
    }

    /// Checkout a PR using gh cli
    pub fn checkout_pr(
        &self,
        repo_path: &Path,
        pr_number: u32,
        worktree_path: &Path,
    ) -> Result<()> {
        // Use gh CLI to checkout PR as worktree
        let output = Command::new("gh")
            .current_dir(repo_path)
            .args(&[
                "pr",
                "checkout",
                &pr_number.to_string(),
                "--worktree",
                worktree_path.to_str().unwrap(),
            ])
            .output();

        match output {
            Ok(output) if output.status.success() => Ok(()),
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(anyhow::anyhow!("Failed to checkout PR: {}", stderr))
            }
            Err(_e) => {
                // Fallback: try to create worktree manually
                self.create_worktree_for_pr(repo_path, pr_number, worktree_path)
                    .context("Failed to checkout PR and fallback method also failed")
            }
        }
    }

    fn create_worktree_for_pr(
        &self,
        repo_path: &Path,
        pr_number: u32,
        worktree_path: &Path,
    ) -> Result<()> {
        let repo = Repository::open(repo_path)?;
        let pr_branch = format!("pr-{}", pr_number);

        // Fetch the PR ref
        self.execute_git_command(
            repo_path,
            &[
                "fetch",
                "origin",
                &format!("pull/{}/head:{}", pr_number, pr_branch),
            ],
        )?;

        // Create worktree for the PR branch
        self.create_worktree(&repo, &pr_branch, worktree_path, &pr_branch, None)?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct WorktreeStatus {
    pub modified_files: Vec<String>,
    pub new_files: Vec<String>,
    pub deleted_files: Vec<String>,
    pub commits_ahead: usize,
    pub commits_behind: usize,
    pub clean: bool,
}

impl Default for GitManager {
    fn default() -> Self {
        Self::new()
    }
}