use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ImiError {
    #[error("Git operation failed: {0}")]
    GitError(#[from] git2::Error),
    
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Worktree not found: {repo}/{name}")]
    WorktreeNotFound { repo: String, name: String },
    
    #[error("Repository not found: {name}")]
    RepositoryNotFound { name: String },
    
    #[error("Worktree already exists: {repo}/{name}")]
    WorktreeAlreadyExists { repo: String, name: String },
    
    #[error("Invalid worktree name: {name}")]
    InvalidWorktreeName { name: String },
    
    #[error("Git repository not found at path: {path}")]
    GitRepositoryNotFound { path: String },
    
    #[error("Branch not found: {branch}")]
    BranchNotFound { branch: String },
    
    #[error("Remote not found: {remote}")]
    RemoteNotFound { remote: String },
    
    #[error("Symlink creation failed: {source} -> {target}: {io_error}")]
    SymlinkCreationFailed { 
        source: String, 
        target: String, 
        #[source]
        io_error: io::Error 
    },
    
    #[error("Monitor error: {0}")]
    MonitorError(String),
    
    #[error("Agent communication error: {0}")]
    AgentCommunicationError(String),
}

pub type Result<T> = std::result::Result<T, ImiError>;