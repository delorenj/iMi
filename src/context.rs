//! Context detection for Git repositories and worktrees
//!
//! This module provides types and methods for detecting the user's current location
//! within the Git worktree hierarchy and making intelligent decisions about
//! which repositories and worktrees to display.

use std::path::PathBuf;

/// Represents the Git context of the current location
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitContext {
    /// Currently in a specific worktree of a repository
    InWorktree {
        repo_path: PathBuf,
        worktree_path: PathBuf,
    },
    /// In the main repository directory (trunk)
    InTrunk { repo_path: PathBuf },
    /// In a repository but not in any specific worktree
    InRepository { repo_path: PathBuf },
    /// Not in any Git repository
    Outside,
}

/// Represents the location context relative to the iMi workspace
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocationContext {
    /// Inside a specific repository
    InRepository {
        repo_path: PathBuf,
        git_context: GitContext,
    },
    /// In the iMi root but not in any repository
    InRoot { root_path: PathBuf },
    /// Outside the iMi root
    Outside,
}

/// Full context information about a repository
#[derive(Debug, Clone)]
pub struct RepositoryContext {
    /// The repository path
    pub repo_path: PathBuf,
    /// The repository name
    pub repo_name: String,
    /// Git context within this repository
    pub git_context: GitContext,
    /// Type of worktree location if applicable
    pub worktree_type: Option<WorktreeLocationType>,
    /// Registration status of the repository
    pub registration: RepositoryRegistration,
}

/// Type of worktree location
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorktreeLocationType {
    /// In the main/trunk worktree
    Trunk,
    /// In a feature worktree (feat/*)
    Feature,
    /// In a fix worktree (fix/*)
    Fix,
    /// In a review/PR worktree (review-pr-*)
    Review,
    /// In an aiops worktree (aiops/*)
    Aiops,
    /// In a devops worktree (devops/*)
    Devops,
    /// In an unknown/unrecognized worktree
    Other,
}

/// Repository registration status in the iMi database
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepositoryRegistration {
    /// Repository is registered in the database
    Registered {
        // TODO: Add database ID when database integration is implemented
        // db_id: i64,
    },
    /// Repository is not registered in the database
    Unregistered,
    /// Database lookup failed or is not available
    Unknown,
}

impl GitContext {
    /// Check if the context is inside any worktree
    pub fn is_in_worktree(&self) -> bool {
        matches!(self, GitContext::InWorktree { .. })
    }

    /// Check if the context is in trunk
    pub fn is_in_trunk(&self) -> bool {
        matches!(self, GitContext::InTrunk { .. })
    }

    /// Check if the context is in any repository
    pub fn is_in_repository(&self) -> bool {
        !matches!(self, GitContext::Outside)
    }

    /// Get the repository path if available
    pub fn repo_path(&self) -> Option<&PathBuf> {
        match self {
            GitContext::InWorktree { repo_path, .. } => Some(repo_path),
            GitContext::InTrunk { repo_path } => Some(repo_path),
            GitContext::InRepository { repo_path } => Some(repo_path),
            GitContext::Outside => None,
        }
    }

    /// Get the worktree path if in a worktree
    pub fn worktree_path(&self) -> Option<&PathBuf> {
        match self {
            GitContext::InWorktree { worktree_path, .. } => Some(worktree_path),
            _ => None,
        }
    }
}

impl LocationContext {
    /// Check if the context is inside a repository
    pub fn is_in_repository(&self) -> bool {
        matches!(self, LocationContext::InRepository { .. })
    }

    /// Check if the context is in the iMi root
    pub fn is_in_root(&self) -> bool {
        matches!(self, LocationContext::InRoot { .. })
    }

    /// Get the repository path if available
    pub fn repo_path(&self) -> Option<&PathBuf> {
        match self {
            LocationContext::InRepository { repo_path, .. } => Some(repo_path),
            _ => None,
        }
    }

    /// Get the Git context if available
    pub fn git_context(&self) -> Option<&GitContext> {
        match self {
            LocationContext::InRepository { git_context, .. } => Some(git_context),
            _ => None,
        }
    }
}

impl WorktreeLocationType {
    /// Detect the worktree type from a branch name
    pub fn from_branch_name(branch: &str) -> Self {
        if branch.starts_with("feat/") || branch.starts_with("feature/") {
            WorktreeLocationType::Feature
        } else if branch.starts_with("fix/") || branch.starts_with("bugfix/") {
            WorktreeLocationType::Fix
        } else if branch.starts_with("review-pr-") || branch.starts_with("pr-") {
            WorktreeLocationType::Review
        } else if branch.starts_with("aiops/") {
            WorktreeLocationType::Aiops
        } else if branch.starts_with("devops/") {
            WorktreeLocationType::Devops
        } else if branch == "main" || branch == "master" {
            WorktreeLocationType::Trunk
        } else {
            WorktreeLocationType::Other
        }
    }

    /// Get the display prefix for this worktree type
    pub fn prefix(&self) -> &'static str {
        match self {
            WorktreeLocationType::Trunk => "trunk",
            WorktreeLocationType::Feature => "feat",
            WorktreeLocationType::Fix => "fix",
            WorktreeLocationType::Review => "review",
            WorktreeLocationType::Aiops => "aiops",
            WorktreeLocationType::Devops => "devops",
            WorktreeLocationType::Other => "other",
        }
    }
}
