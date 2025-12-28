//! iMi - Git Worktree Management Tool
//!
//! A sophisticated worktree management tool designed for asynchronous,
//! parallel multi-agent workflows with opinionated defaults and real-time visibility.

pub mod cli;
pub mod commands;
pub mod config;
pub mod context;
pub mod database;
pub mod error;
pub mod fuzzy;
pub mod git;
pub mod github;
pub mod init;
pub mod monitor;
pub mod worktree;

// Re-export commonly used types
pub use config::Config;
pub use context::{
    GitContext, LocationContext, RepositoryContext, RepositoryRegistration, WorktreeLocationType,
};
pub use database::{AgentActivity, Database, Repository, Worktree};
pub use error::ImiError;
pub use git::{GitManager, WorktreeStatus};
pub use init::{InitCommand, InitResult};
pub use worktree::WorktreeManager;

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default configuration values
pub mod defaults {
    /// Default root directory for iMi repositories
    pub const DEFAULT_ROOT: &str = "~/code";

    /// Default database filename
    pub const DEFAULT_DB_NAME: &str = "iMi.db";

    /// Default config filename
    pub const DEFAULT_CONFIG_NAME: &str = "config.toml";

    /// Default branch name
    pub const DEFAULT_BRANCH: &str = "main";

    /// Default remote name
    pub const DEFAULT_REMOTE: &str = "origin";
}
