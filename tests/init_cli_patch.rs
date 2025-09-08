/// This file contains the exact code changes needed to implement the init command
/// It serves as both documentation and as a reference for implementation

#[cfg(test)]
mod implementation_guide {
    

    /// Add this to src/cli.rs Commands enum (after Monitor command)
    #[test]
    fn cli_enum_addition() {
        let code_to_add = r#"
    /// Initialize iMi in the current trunk directory
    Init {
        /// Force initialization even if already initialized
        #[arg(long, short)]
        force: bool,
        
        /// Show what would be done without making changes  
        #[arg(long, short = 'n')]
        dry_run: bool,
        
        /// Show detailed output during initialization
        #[arg(long, short)]
        verbose: bool,
        
        /// Use custom config file instead of default
        #[arg(long)]
        config: Option<PathBuf>,
    },"#;

        println!("Add this to Commands enum in cli.rs:\n{}", code_to_add);
    }

    /// Add this to src/main.rs match statement (after Monitor handler)
    #[test]
    fn main_handler_addition() {
        let code_to_add = r#"        Commands::Init { force, dry_run, verbose, config } => {
            handle_init_command(&worktree_manager, force, dry_run, verbose, config).await?;
        }"#;

        println!("Add this to match statement in main.rs:\n{}", code_to_add);
    }

    /// Add this function to src/main.rs (after handle_monitor_command)
    #[test]
    fn main_function_addition() {
        let code_to_add = r#"async fn handle_init_command(
    manager: &WorktreeManager,
    force: bool,
    dry_run: bool,
    verbose: bool,
    config: Option<PathBuf>,
) -> Result<()> {
    use crate::init::InitCommand;
    use std::env;

    if verbose {
        println!("{} Initializing iMi...", "🚀".bright_cyan());
    }

    // Check if we're in a trunk directory
    let current_dir = env::current_dir()?;
    let dir_name = current_dir
        .file_name()
        .context("Invalid current directory")?
        .to_str()
        .context("Invalid directory name")?;

    if !dir_name.starts_with("trunk-") {
        return Err(anyhow::anyhow!(
            "{}\\n\\nCurrent directory: {}\\nExpected pattern: trunk-<branch-name>\\n\\nExamples:\\n  trunk-main\\n  trunk-develop\\n  trunk-staging\\n\\nRun 'iMi init' from your trunk directory to initialize iMi for this repository.",
            "Error: iMi init must be run from a directory starting with 'trunk-'".bright_red(),
            dir_name.bright_yellow()
        ));
    }

    // Create and run init command
    let init_cmd = InitCommand::new(
        manager.git.clone(),
        manager.db.clone(), 
        manager.config.clone()
    );

    if dry_run {
        init_cmd.dry_run().await?;
    } else {
        init_cmd.init_with_options(force, verbose, config).await?;
    }

    Ok(())
}"#;

        println!("Add this function to main.rs:\n{}", code_to_add);
    }

    /// Add this new module to src/ directory
    #[test]
    fn init_module_creation() {
        let code_to_add = r##"// File: src/init.rs

use anyhow::{Context, Result};
use colored::*;
use std::env;
use std::path::{Path, PathBuf};
use tokio::fs;

use crate::config::Config;
use crate::database::Database;
use crate::git::GitManager;

pub struct InitCommand {
    git: GitManager,
    db: Database,
    config: Config,
}

impl InitCommand {
    pub fn new(git: GitManager, db: Database, config: Config) -> Self {
        Self { git, db, config }
    }

    pub async fn init(&self) -> Result<()> {
        self.init_with_options(false, false, None).await
    }

    pub async fn init_with_options(
        &self,
        force: bool,
        verbose: bool,
        custom_config: Option<PathBuf>,
    ) -> Result<()> {
        let start_time = std::time::Instant::now();
        
        if verbose {
            println!("{} Checking current directory...", "🔍".bright_blue());
        }

        // Validate we're in a trunk directory
        let current_dir = env::current_dir().context("Failed to get current directory")?;
        let dir_name = current_dir
            .file_name()
            .context("Invalid current directory")?
            .to_str()
            .context("Invalid directory name")?;

        if !dir_name.starts_with("trunk-") {
            return Err(anyhow::anyhow!(
                "iMi init can only be run from a directory starting with 'trunk-'. Current directory: {}",
                dir_name
            ));
        }

        if verbose {
            println!("  {} Current directory: {} {}", "📁".bright_blue(), dir_name, "✅".bright_green());
        }

        // Get repository name from parent directory
        let repo_name = current_dir
            .parent()
            .context("No parent directory found")?
            .file_name()
            .context("Invalid parent directory")?
            .to_str()
            .context("Invalid parent directory name")?
            .to_string();

        if verbose {
            println!("  {} Parent directory: {} {}", "📁".bright_blue(), repo_name, "✅".bright_green());
        }

        // Check if already initialized
        let imi_dir = current_dir.join(".imi");
        if imi_dir.exists() && !force {
            let repo_config_path = imi_dir.join("repo.toml");
            let timestamp = if repo_config_path.exists() {
                fs::metadata(&repo_config_path)
                    .await
                    .and_then(|m| m.modified())
                    .map(|t| format!("{:?}", t))
                    .unwrap_or_else(|_| "Unknown".to_string())
            } else {
                "Unknown".to_string()
            };

            return Err(anyhow::anyhow!(
                "{}\\n\\nFound existing .imi directory at: {}\\nInitialized: {}\\n\\nUse 'iMi init --force' to reinitialize, which will:\\n  - Recreate configuration files\\n  - Reset database entries\\n  - Preserve existing worktree data",
                "Error: Repository already initialized".bright_red(),
                imi_dir.display(),
                timestamp
            ));
        }

        if verbose {
            println!("\\n{} Loading configuration...", "🔧".bright_blue());
        }

        // Load or create config
        let config = if let Some(config_path) = custom_config {
            // Load custom config logic would go here
            self.config.clone()
        } else {
            self.config.clone()
        };

        // Ensure global config exists
        config.save().await.context("Failed to save global configuration")?;
        
        if verbose {
            println!("  {} Global config: {} {}", 
                "📄".bright_blue(), 
                config.get_config_path()?.display(),
                "✅".bright_green()
            );
        }

        if verbose {
            println!("\\n{} Initializing database...", "💾".bright_blue());
        }

        // Initialize database
        self.db.ensure_tables().await.context("Failed to initialize database tables")?;
        
        if verbose {
            println!("  {} Database path: {}", "🗄️".bright_blue(), config.database_path.display());
            println!("  {} Creating tables: worktrees, agents, activities {}", 
                "📊".bright_blue(), "✅".bright_green());
        }

        if verbose {
            println!("\\n{} Creating directories...", "📂".bright_blue());
        }

        // Create .imi directory
        if force {
            fs::remove_dir_all(&imi_dir).await.ok(); // Ignore errors
        }
        fs::create_dir_all(&imi_dir).await
            .context("Failed to create .imi directory")?;
        
        if verbose {
            println!("  {} .imi/ {}", "📁".bright_blue(), "✅".bright_green());
        }

        // Create sync directories
        let global_sync = config.get_sync_path(&repo_name, true);
        let repo_sync = config.get_sync_path(&repo_name, false);
        
        fs::create_dir_all(&global_sync).await
            .context("Failed to create global sync directory")?;
        fs::create_dir_all(&repo_sync).await
            .context("Failed to create repo sync directory")?;
        
        if verbose {
            println!("  {} sync/global/ {}", "📁".bright_blue(), "✅".bright_green());
            println!("  {} sync/repo/ {}", "📁".bright_blue(), "✅".bright_green());
        }

        if verbose {
            println!("\\n{} Writing configuration...", "📄".bright_blue());
        }

        // Create repository configuration
        let repo_config_path = imi_dir.join("repo.toml");
        let repo_config_content = format!(
            r#"[repository]
name = "{}"
root_path = "{}"
trunk_path = "{}"
initialized_at = "{}"

[settings]
auto_sync = true
track_agents = true
monitor_enabled = true

[paths]
sync_global = "sync/global"
sync_repo = "sync/repo"

[git]
trunk_branch = "{}"
remote_name = "{}"
auto_fetch = {}
"#,
            repo_name,
            current_dir.parent().unwrap().display(),
            current_dir.display(),
            chrono::Utc::now().to_rfc3339(),
            config.git_settings.default_branch,
            config.git_settings.remote_name,
            config.git_settings.auto_fetch
        );

        fs::write(&repo_config_path, repo_config_content).await
            .context("Failed to write repository configuration")?;
        
        if verbose {
            println!("  {} .imi/repo.toml {}", "📝".bright_blue(), "✅".bright_green());
        }

        // Create default sync files if they don't exist
        let coding_rules = global_sync.join("coding-rules.md");
        if !coding_rules.exists() {
            let content = include_str!("../templates/coding-rules.md");
            fs::write(&coding_rules, content).await?;
            
            if verbose {
                println!("  {} sync/global/coding-rules.md {}", "📝".bright_blue(), "✅".bright_green());
            }
        }

        let stack_specific = global_sync.join("stack-specific.md");
        if !stack_specific.exists() {
            let content = include_str!("../templates/stack-specific.md");
            fs::write(&stack_specific, content).await?;
            
            if verbose {
                println!("  {} sync/global/stack-specific.md {}", "📝".bright_blue(), "✅".bright_green());
            }
        }

        if verbose {
            println!("\\n{} Registering trunk worktree...", "🗄️".bright_blue());
        }

        // Register trunk worktree in database
        let trunk_name = dir_name;
        self.db.create_worktree(
            &repo_name,
            trunk_name,
            &config.git_settings.default_branch,
            "trunk",
            current_dir.to_str().unwrap(),
            None,
        ).await.context("Failed to record trunk worktree in database")?;

        if verbose {
            println!("  {} Worktree ID: {}", "📊".bright_blue(), trunk_name);
            println!("  {} Branch: {}", "🌿".bright_blue(), config.git_settings.default_branch);
            println!("  {} Path: {} {}", 
                "📁".bright_blue(), 
                current_dir.display(),
                "✅".bright_green()
            );
        }

        let duration = start_time.elapsed();
        
        if verbose {
            println!("\\n{} Initialization complete! ({}ms)", 
                "✅".bright_green(), 
                duration.as_millis()
            );
        } else {
            println!("{} iMi initialized successfully!", "✅".bright_green());
            println!("\\n{} Repository: {}", "📁".bright_blue(), repo_name.bright_green());
            println!("{} Trunk path: {}", "🌳".bright_green(), current_dir.display());
            println!("{} Configuration: {}", "🔧".bright_blue(), repo_config_path.display());
            
            println!("\\n{}:", "Created".bright_cyan());
            println!("  {} .imi/                    - Repository configuration", "📂".bright_blue());
            println!("  {} sync/global/             - Global sync files", "📂".bright_blue());
            println!("  {} sync/repo/               - Repository-specific sync files", "📂".bright_blue());
            
            if coding_rules.exists() {
                println!("  {} sync/global/coding-rules.md", "📄".bright_blue());
            }
            if stack_specific.exists() {
                println!("  {} sync/global/stack-specific.md", "📄".bright_blue());
            }
            
            println!("\\n{}:", "Database".bright_cyan());
            println!("  {} Tables initialized", "✅".bright_green());
            println!("  {} Trunk worktree registered", "✅".bright_green());
            
            println!("\\n{}:", "Next steps".bright_cyan());
            println!("  {} Create a feature:    iMi feat my-feature", "🚀".bright_green());
            println!("  {} Review a PR:         iMi pr 123", "🔍".bright_yellow());
            println!("  {} Fix a bug:           iMi fix critical-issue", "🔧".bright_red());
            println!("  {} Check status:        iMi status", "📊".bright_blue());
        }

        Ok(())
    }

    pub async fn dry_run(&self) -> Result<()> {
        println!("{} Dry run mode - no changes will be made", "🔍".bright_yellow());
        println!();

        let current_dir = env::current_dir()?;
        let dir_name = current_dir.file_name().unwrap().to_str().unwrap();
        
        if !dir_name.starts_with("trunk-") {
            return Err(anyhow::anyhow!(
                "Error: Not in trunk directory ({})", dir_name
            ));
        }

        let repo_name = current_dir
            .parent()
            .unwrap()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap();

        println!("{}:", "Would create directories".bright_cyan());
        println!("  {} {}/", "📂".bright_blue(), current_dir.join(".imi").display());
        println!("  {} {}/", "📂".bright_blue(), self.config.get_sync_path(&repo_name, true).display());
        println!("  {} {}/", "📂".bright_blue(), self.config.get_sync_path(&repo_name, false).display());

        println!("\\n{}:", "Would create files".bright_cyan());
        println!("  {} {}", "📄".bright_blue(), current_dir.join(".imi/repo.toml").display());
        println!("  {} {}", "📄".bright_blue(), self.config.get_sync_path(&repo_name, true).join("coding-rules.md").display());
        println!("  {} {}", "📄".bright_blue(), self.config.get_sync_path(&repo_name, true).join("stack-specific.md").display());

        println!("\\n{}:", "Would update database".bright_cyan());
        println!("  {} Create worktree entry: {} (type: trunk, branch: {})", 
            "📊".bright_blue(), 
            dir_name,
            self.config.git_settings.default_branch
        );

        println!("\\n{}:", "Global configuration".bright_cyan());
        println!("  {} Would create: {}", "📄".bright_blue(), self.config.get_config_path()?.display());

        println!("\\n{} Dry run complete - run without --dry-run to apply changes", "✅".bright_green());

        Ok(())
    }
}"##;

        println!("Create this file as src/init.rs:\n{}", code_to_add);
    }

    /// Add this to src/main.rs modules section
    #[test]
    fn main_module_addition() {
        let code_to_add = "mod init;";
        println!("Add this to src/main.rs modules section:\n{}", code_to_add);
    }

    /// Create template files
    #[test]
    fn template_files() {
        let coding_rules_template = r#"# Coding Rules

This file contains coding standards and rules that apply across all worktrees in this repository.

## Style Guidelines

- Follow language-specific style guides
- Use consistent indentation (spaces vs tabs)  
- Maintain consistent naming conventions

## Best Practices

- Write meaningful commit messages
- Include tests for new functionality
- Document public APIs
- Review code before merging

## Repository-Specific Rules

Add your repository-specific coding rules here.

---
*This file is automatically created by `iMi init` and can be customized for your team's needs.*"#;

        let stack_specific_template = r#"# Stack-Specific Guidelines

This file contains guidelines specific to your technology stack.

## Frontend

- Framework-specific best practices
- Component organization  
- State management patterns
- Testing strategies

## Backend

- API design principles
- Database interaction patterns
- Authentication/authorization
- Error handling strategies

## Database

- Schema design principles
- Migration strategies
- Performance optimization
- Data validation rules

## DevOps

- Deployment procedures
- Environment management
- Monitoring and logging
- Security considerations

---
*This file is automatically created by `iMi init` and should be customized for your specific technology stack.*"#;

        println!(
            "Create templates/coding-rules.md:\n{}",
            coding_rules_template
        );
        println!(
            "\nCreate templates/stack-specific.md:\n{}",
            stack_specific_template
        );
    }

    /// Add required imports to main.rs
    #[test]
    fn main_imports_addition() {
        let code_to_add = r#"
// Add these imports to the existing use statements in main.rs:
use std::path::PathBuf;  // If not already imported"#;

        println!("Imports to add:\n{}", code_to_add);
    }

    /// Add methods to Config struct if needed
    #[test]
    fn config_methods_addition() {
        let code_to_add = r#"
// Add this method to Config impl in src/config.rs if it doesn't exist:
impl Config {
    pub fn get_config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("Could not find config directory")?
            .join("imi");
        Ok(config_dir.join("config.toml"))
    }
}"#;

        println!("Methods to add to Config:\n{}", code_to_add);
    }

    /// Add methods to Database struct if needed  
    #[test]
    fn database_methods_addition() {
        let code_to_add = r#"
// Add this method to Database impl in src/database.rs if it doesn't exist:
impl Database {
    pub async fn ensure_tables(&self) -> Result<()> {
        // This method should create all necessary database tables
        // Implementation depends on your existing database setup
        Ok(())
    }
}"#;

        println!("Methods to add to Database:\n{}", code_to_add);
    }
}
