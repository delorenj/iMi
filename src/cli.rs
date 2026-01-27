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

    /// Output results in JSON format (available for all commands)
    #[arg(long, global = true)]
    pub json: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Add a new worktree of specified type
    Add {
        /// Worktree type (feat, fix, aiops, devops, review, or custom)
        #[arg(value_name = "TYPE")]
        worktree_type: String,

        /// Descriptive name for the worktree (or PR number for review)
        name: String,

        /// Repository name (optional, uses current if not specified)
        #[arg(short, long)]
        repo: Option<String>,

        /// PR number (optional for 'review' type if name is a PR number)
        #[arg(long)]
        pr: Option<u32>,
    },

    /// Manage worktree types
    #[command(subcommand)]
    Types(TypeCommands),

    /// Create a new feature worktree
    #[command(alias = "feature", hide = true)]
    #[deprecated(note = "Use 'imi add feat <name>' instead")]
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

        /// Repository: local name, {org}/{repo}, or {owner}/{repo} (defaults to delorenj/{repo} if org omitted)
        /// When invoked outside a git project, queries iMi database for registered repos
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

    /// Navigate to a worktree or repository using fuzzy search
    Go {
        /// Fuzzy search query (worktree name, branch name, or repo name)
        /// If not provided, shows an interactive picker
        query: Option<String>,

        /// Exact repository name (skip fuzzy search within this repo)
        #[arg(short = 'r', long)]
        repo: Option<String>,

        /// Limit search to worktrees only (exclude trunk/repos)
        #[arg(short = 'w', long)]
        worktrees_only: bool,

        /// Include inactive/closed worktrees in search
        #[arg(short = 'a', long)]
        include_inactive: bool,
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

    /// Repair repository paths in database (auto-detects moved repositories)
    Repair,

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

        /// Show what would be removed without actually removing
        #[arg(long)]
        dry_run: bool,

        /// Remove orphaned directories without confirmation
        #[arg(long)]
        force: bool,
    },

    /// Merge a worktree into trunk-main and close it
    Merge {
        /// Name of the worktree to merge (optional, defaults to current branch)
        name: Option<String>,

        /// Repository name (optional, uses current repo if not specified)
        repo: Option<String>,
    },

    /// Create a new project with boilerplate scaffolding
    Project {
        #[command(subcommand)]
        command: ProjectCommands,
    },

    /// Claim exclusive access to a worktree for agent work
    Claim {
        /// Name of the worktree to claim
        name: String,

        /// Yi agent identifier
        #[arg(long = "yi-id")]
        yi_id: String,

        /// Repository name (optional, uses current repo if not specified)
        #[arg(short, long)]
        repo: Option<String>,

        /// Force claim even if already claimed by another agent
        #[arg(long)]
        force: bool,
    },

    /// Verify lock ownership for a worktree
    #[command(alias = "check-lock")]
    VerifyLock {
        /// Name of the worktree to verify
        name: String,

        /// Yi agent identifier to check ownership
        #[arg(long = "yi-id")]
        yi_id: String,

        /// Repository name (optional, uses current repo if not specified)
        #[arg(short, long)]
        repo: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum ProjectCommands {
    /// Create a new project with GitHub repository and boilerplate
    Create {
        /// Project concept description (natural language)
        #[arg(short = 'c', long = "concept")]
        concept: Option<String>,

        /// Path to PRD markdown file
        #[arg(short = 'p', long = "prd")]
        prd: Option<String>,

        /// Explicit project name (optional, will be inferred from concept/prd if not provided)
        #[arg(short = 'n', long = "name")]
        name: Option<String>,

        /// JSON payload for structured project definition
        #[arg(long = "payload")]
        payload: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum TypeCommands {
    /// List all available worktree types
    #[command(alias = "ls")]
    List,

    /// Add a new worktree type
    Add {
        /// Type name (lowercase, alphanumeric, hyphens)
        name: String,

        /// Branch prefix (defaults to <type>/)
        #[arg(long)]
        branch_prefix: Option<String>,

        /// Worktree prefix (defaults to <type>-)
        #[arg(long)]
        worktree_prefix: Option<String>,

        /// Description
        #[arg(short, long)]
        description: Option<String>,
    },

    /// Remove a worktree type
    #[command(alias = "rm")]
    Remove {
        /// Type name to remove
        name: String,
    },
}
