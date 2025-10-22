use anyhow::{Context, Result};
use clap::Parser;
use colored::*;

mod cli;
mod config;
mod database;
mod error;
mod git;
mod init;
mod monitor;
mod worktree;

use cli::{Cli, Commands};
use config::Config;
use database::Database;
use git::GitManager;
use init::InitCommand;
use worktree::WorktreeManager;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize the CLI
    let cli = Cli::parse();

    if let Some(command) = cli.command {
        match command {
            Commands::Init { repo, force } => {
                handle_init_command(repo, force).await?;
            }
            _ => {
                // Load configuration
                let config = Config::load()
                    .await
                    .context("Failed to load configuration. Have you run 'imi init'?")?;

                // Initialize database
                let db = Database::new(&config.database_path)
                    .await
                    .context("Failed to initialize database")?;

                // Initialize Git manager
                let git_manager = GitManager::new();

                // Initialize worktree manager
                let worktree_manager = WorktreeManager::new(git_manager, db, config.clone());

                match command {
                    Commands::Feat { name, repo } => {
                        handle_feature_command(&worktree_manager, &name, repo.as_deref()).await?;
                    }
                    Commands::Review { pr_number, repo } => {
                        handle_review_command(&worktree_manager, pr_number, repo.as_deref())
                            .await?;
                    }
                    Commands::Fix { name, repo } => {
                        handle_fix_command(&worktree_manager, &name, repo.as_deref()).await?;
                    }
                    Commands::Aiops { name, repo } => {
                        handle_aiops_command(&worktree_manager, &name, repo.as_deref()).await?;
                    }
                    Commands::Devops { name, repo } => {
                        handle_devops_command(&worktree_manager, &name, repo.as_deref()).await?;
                    }
                    Commands::Trunk { repo } => {
                        handle_trunk_command(&worktree_manager, repo.as_deref()).await?;
                    }
                    Commands::Status { repo } => {
                        handle_status_command(&worktree_manager, repo.as_deref()).await?;
                    }
                    Commands::List { repo } => {
                        handle_list_command(&worktree_manager, repo.as_deref()).await?;
                    }
                    Commands::Remove {
                        name,
                        repo,
                        keep_branch,
                        keep_remote,
                    } => {
                        handle_remove_command(
                            &worktree_manager,
                            &name,
                            repo.as_deref(),
                            keep_branch,
                            keep_remote,
                        )
                        .await?;
                    }
                    Commands::Monitor { repo } => {
                        handle_monitor_command(&worktree_manager, repo.as_deref()).await?;
                    }
                    Commands::Sync { repo } => {
                        handle_sync_command(&worktree_manager, repo.as_deref()).await?;
                    }
                    Commands::Init { .. } => {
                        // Already handled
                    }
                    Commands::Completion { shell } => {
                        handle_completion_command(&shell);
                    }
                    Commands::Prune { repo } => {
                        handle_prune_command(&worktree_manager, repo.as_deref()).await?;
                    }
                    Commands::Close { name, repo } => {
                        handle_close_command(&worktree_manager, &name, repo.as_deref()).await?;
                    }
                    Commands::Merge { name, repo } => {
                        handle_merge_command(&worktree_manager, &name, repo.as_deref()).await?;
                    }
                }
            }
        }
    }

    Ok(())
}

async fn handle_feature_command(
    manager: &WorktreeManager,
    name: &str,
    repo: Option<&str>,
) -> Result<()> {
    println!(
        "{} Creating feature worktree: {}",
        "🚀".bright_cyan(),
        name.bright_green()
    );

    match manager.create_feature_worktree(name, repo).await {
        Ok(worktree_path) => {
            println!(
                "{} Feature worktree created at: {}",
                "✅".bright_green(),
                worktree_path.display()
            );

            // Print command to change directory (processes can't change parent shell's directory)
            println!(
                "\n{} To navigate to the worktree, run:\n   {}",
                "💡".bright_yellow(),
                format!("cd {}", worktree_path.display()).bright_cyan()
            );
        }
        Err(e) => {
            let error_msg = e.to_string().to_lowercase();
            // Check if it's an authentication error
            if error_msg.contains("authentication")
                || error_msg.contains("auth")
                || error_msg.contains("credential")
                || error_msg.contains("ssh")
            {
                println!("{} Authentication failed", "❌".bright_red());
                println!();

                // Show authentication help
                let git_manager = GitManager::new();
                git_manager.show_auth_help();
                println!();

                return Err(e);
            }
            return Err(e);
        }
    }

    Ok(())
}

async fn handle_review_command(
    manager: &WorktreeManager,
    pr_number: u32,
    repo: Option<&str>,
) -> Result<()> {
    println!(
        "{} Creating review worktree for PR: {}",
        "🔍".bright_yellow(),
        pr_number.to_string().bright_green()
    );
    let worktree_path = manager.create_review_worktree(pr_number, repo).await?;
    println!(
        "{} Review worktree created at: {}",
        "✅".bright_green(),
        worktree_path.display()
    );

    // Print command to change directory (processes can't change parent shell's directory)
    println!(
        "\n{} To navigate to the worktree, run:\n   {}",
        "💡".bright_yellow(),
        format!("cd {}", worktree_path.display()).bright_cyan()
    );

    Ok(())
}

async fn handle_fix_command(
    manager: &WorktreeManager,
    name: &str,
    repo: Option<&str>,
) -> Result<()> {
    println!(
        "{} Creating fix worktree: {}",
        "🔧".bright_red(),
        name.bright_green()
    );
    let worktree_path = manager.create_fix_worktree(name, repo).await?;
    println!(
        "{} Fix worktree created at: {}",
        "✅".bright_green(),
        worktree_path.display()
    );

    // Print command to change directory (processes can't change parent shell's directory)
    println!(
        "\n{} To navigate to the worktree, run:\n   {}",
        "💡".bright_yellow(),
        format!("cd {}", worktree_path.display()).bright_cyan()
    );

    Ok(())
}

async fn handle_aiops_command(
    manager: &WorktreeManager,
    name: &str,
    repo: Option<&str>,
) -> Result<()> {
    println!(
        "{} Creating aiops worktree: {}",
        "🤖".bright_magenta(),
        name.bright_green()
    );
    let worktree_path = manager.create_aiops_worktree(name, repo).await?;
    println!(
        "{} Aiops worktree created at: {}",
        "✅".bright_green(),
        worktree_path.display()
    );

    // Print command to change directory (processes can't change parent shell's directory)
    println!(
        "\n{} To navigate to the worktree, run:\n   {}",
        "💡".bright_yellow(),
        format!("cd {}", worktree_path.display()).bright_cyan()
    );

    Ok(())
}

async fn handle_devops_command(
    manager: &WorktreeManager,
    name: &str,
    repo: Option<&str>,
) -> Result<()> {
    println!(
        "{} Creating devops worktree: {}",
        "⚙️".bright_blue(),
        name.bright_green()
    );
    let worktree_path = manager.create_devops_worktree(name, repo).await?;
    println!(
        "{} Devops worktree created at: {}",
        "✅".bright_green(),
        worktree_path.display()
    );

    // Print command to change directory (processes can't change parent shell's directory)
    println!(
        "\n{} To navigate to the worktree, run:\n   {}",
        "💡".bright_yellow(),
        format!("cd {}", worktree_path.display()).bright_cyan()
    );

    Ok(())
}

async fn handle_trunk_command(manager: &WorktreeManager, repo: Option<&str>) -> Result<()> {
    println!("{} Switching to trunk worktree", "🌳".bright_green());
    let worktree_path = manager.get_trunk_worktree(repo).await?;

    // Print command to change directory (processes can't change parent shell's directory)
    println!(
        "{} To navigate to trunk, run:\n   {}",
        "💡".bright_yellow(),
        format!("cd {}", worktree_path.display()).bright_cyan()
    );

    Ok(())
}

async fn handle_status_command(manager: &WorktreeManager, repo: Option<&str>) -> Result<()> {
    println!("{} Worktree Status", "📊".bright_cyan());
    manager.show_status(repo).await?;
    Ok(())
}

async fn handle_list_command(manager: &WorktreeManager, repo: Option<&str>) -> Result<()> {
    println!("{} Active Worktrees", "📋".bright_cyan());
    manager.list_worktrees(repo).await?;
    Ok(())
}

async fn handle_remove_command(
    manager: &WorktreeManager,
    name: &str,
    repo: Option<&str>,
    keep_branch: bool,
    keep_remote: bool,
) -> Result<()> {
    println!(
        "{} Removing worktree: {}",
        "🗑️".bright_red(),
        name.bright_yellow()
    );
    manager
        .remove_worktree(name, repo, keep_branch, keep_remote)
        .await?;
    println!("{} Worktree removed successfully", "✅".bright_green());
    Ok(())
}

async fn handle_monitor_command(manager: &WorktreeManager, repo: Option<&str>) -> Result<()> {
    println!("{} Starting real-time monitoring...", "👁️".bright_purple());
    manager.start_monitoring(repo).await?;
    Ok(())
}

async fn handle_sync_command(manager: &WorktreeManager, repo: Option<&str>) -> Result<()> {
    println!("{} Syncing database with Git worktrees...", "🔄".bright_cyan());
    manager.sync_with_git(repo).await?;
    Ok(())
}

async fn handle_init_command(repo: Option<String>, force: bool) -> Result<()> {
    let config = Config::load().await?;
    let db = Database::new(&config.database_path).await?;
    let init_cmd = InitCommand::new(force, config, db);

    // Check if repo argument looks like a GitHub repo (owner/repo format)
    if let Some(ref repo_arg) = repo {
        if repo_arg.contains('/') && !repo_arg.contains(':') {
            // Looks like owner/repo format - clone from GitHub
            let result = init_cmd.clone_from_github(repo_arg).await?;

            if result.success {
                println!("{}", result.message.green());
            } else {
                println!("{}", result.message.red());
            }

            return Ok(());
        } else {
            // Treat as a local path
            let path = std::path::PathBuf::from(repo_arg);
            let result = init_cmd.execute(Some(&path)).await?;

            if result.success {
                println!("{}", result.message.green());
            } else {
                println!("{}", result.message.red());
            }

            return Ok(());
        }
    }

    // No repo argument - normal init
    let result = init_cmd.execute(None).await?;

    if result.success {
        println!("{}", result.message.green());
    } else {
        println!("{}", result.message.red());
    }

    Ok(())
}

async fn handle_prune_command(manager: &WorktreeManager, repo: Option<&str>) -> Result<()> {
    println!(
        "{} Cleaning up stale worktree references",
        "🧹".bright_cyan()
    );
    manager.prune_stale_worktrees(repo).await?;
    println!("{} Cleanup complete", "✅".bright_green());
    Ok(())
}

async fn handle_close_command(
    manager: &WorktreeManager,
    name: &str,
    repo: Option<&str>,
) -> Result<()> {
    println!(
        "{} Closing worktree: {}",
        "🚫".bright_yellow(),
        name.bright_yellow()
    );
    manager.close_worktree(name, repo).await?;

    println!("{} Worktree closed successfully", "✅".bright_green());

    match manager.get_trunk_worktree(repo).await {
        Ok(trunk_path) => {
            println!(
                "\n{} To navigate to trunk, run:\n   {}",
                "💡".bright_yellow(),
                format!("cd {}", trunk_path.display()).bright_cyan()
            );
        }
        Err(err) => {
            println!(
                "{} Unable to locate trunk worktree: {}",
                "⚠️".bright_yellow(),
                err
            );
            println!(
                "{} If needed you can recreate it with: {}",
                "💡".bright_yellow(),
                "imi trunk".bright_cyan()
            );
        }
    }

    Ok(())
}

async fn handle_merge_command(
    manager: &WorktreeManager,
    name: &str,
    repo: Option<&str>,
) -> Result<()> {
    println!(
        "{} Merging worktree: {}",
        "🔀".bright_cyan(),
        name.bright_yellow()
    );

    manager.merge_worktree(name, repo).await?;

    Ok(())
}

fn handle_completion_command(shell: &clap_complete::Shell) {
    use clap::CommandFactory;
    use clap_complete::{generate, Generator};
    use std::io;

    fn print_completions<G: Generator>(gen: G, cmd: &mut clap::Command) {
        generate(gen, cmd, cmd.get_name().to_string(), &mut io::stdout());
    }

    let mut cmd = cli::Cli::command();
    print_completions(*shell, &mut cmd);
}
