use clap::{Parser, Subcommand};
use clap_complete::Shell;

#[derive(Parser)]
#[command(
    name = "imi",
    author = "Jarad DeLorenzo <jarad@33god.ai>",
    version,
    about = "iMi Git Worktree Management Tool - Component of 33GOD Agentic Software Pipeline",
    long_about = "A sophisticated worktree management tool designed for asynchronous, parallel multi-agent workflows. Features opinionated defaults and real-time visibility into worktree activities.",
    disable_version_flag = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(short = 'v', long = "version", action = clap::ArgAction::Version)]
    version: Option<bool>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create a new feature worktree
    #[command(alias = "feature")]
    Feat {
        /// Name of the feature (will create feat-{name} worktree)
        name: String,

        /// Repository name (optional, uses current repo if not specified)
        repo: Option<String>,
    },

    /// Create a worktree for reviewing a pull request
    #[command(alias = "pr")]
    Review {
        /// Pull request number
        pr_number: u32,

        /// Repository name (optional, uses current repo if not specified)  
        repo: Option<String>,
    },

    /// Create a worktree for bug fixes
    Fix {
        /// Name of the fix (will create fix-{name} worktree)
        name: String,

        /// Repository name (optional, uses current repo if not specified)
        repo: Option<String>,
    },

    /// Create a worktree for AI operations (agents, rules, MCP configs, workflows)
    Aiops {
        /// Name of the aiops task (will create aiops-{name} worktree)
        name: String,

        /// Repository name (optional, uses current repo if not specified)
        repo: Option<String>,
    },

    /// Create a worktree for DevOps tasks (CI, repo organization, deploys)
    Devops {
        /// Name of the devops task (will create devops-{name} worktree)
        name: String,

        /// Repository name (optional, uses current repo if not specified)
        repo: Option<String>,
    },

    /// Switch to the trunk worktree (main branch)
    Trunk {
        /// Repository name (optional, uses current repo if not specified)
        repo: Option<String>,
    },

    /// Show status of all worktrees
    Status {
        /// Repository name (optional, shows all repos if not specified)
        repo: Option<String>,
    },

    /// List all active worktrees
    #[command(alias = "ls")]
    List {
        /// Repository name (optional, shows all repos if not specified)
        repo: Option<String>,

        /// List only worktrees (conflicts with --projects)
        #[arg(short = 'w', long, conflicts_with = "projects")]
        worktrees: bool,

        /// List only projects/repositories (conflicts with --worktrees)
        #[arg(short = 'p', long, conflicts_with = "worktrees")]
        projects: bool,
    },

    /// Remove a worktree
    #[command(alias = "rm")]
    Remove {
        /// Name of the worktree to remove
        name: String,

        /// Repository name (optional, uses current repo if not specified)
        repo: Option<String>,

        /// Keep local branch after removing worktree
        #[arg(long)]
        keep_branch: bool,

        /// Keep remote branch after removing worktree (requires --keep-branch)
        #[arg(long)]
        keep_remote: bool,
    },

    /// Close a worktree without merging (cancel the branch)
    #[command(alias = "cancel")]
    Close {
        /// Name of the worktree to close
        name: String,

        /// Repository name (optional, uses current repo if not specified)
        repo: Option<String>,
    },

    /// Start real-time monitoring of worktree activities
    Monitor {
        /// Repository name (optional, monitors all repos if not specified)
        repo: Option<String>,
    },

    /// Sync database with actual Git worktrees
    Sync {
        /// Repository name (optional, syncs all repos if not specified)
        repo: Option<String>,
    },

    /// Initialize iMi in the current directory or clone from GitHub (format: owner/repo)
    Init {
        /// GitHub repository to clone (format: owner/repo), or path to existing repository
        repo: Option<String>,

        /// Force initialization even if configuration already exists
        #[arg(long)]
        force: bool,
    },

    /// Generate shell completions for iMi
    Completion {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },

    /// Clean up stale worktree references from Git
    #[command(alias = "cleanup")]
    Prune {
        /// Repository name (optional, uses current repo if not specified)
        repo: Option<String>,
    },
}
