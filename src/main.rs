use anyhow::{Context, Result};
use clap::Parser;
use colored::*;
use serde::{Deserialize, Serialize};
use serde_json;

mod cli;
mod commands;
mod config;
mod context;
mod database;
mod error;
mod fuzzy;
mod git;
mod github;
mod init;
mod local;
mod monitor;
mod worktree;

use cli::{Cli, Commands, ProjectCommands, TypeCommands};
use commands::project::{ProjectConfig, ProjectCreator};
use config::Config;
use database::Database;
use git::GitManager;
use init::InitCommand;
use worktree::WorktreeManager;

/// JSON response structure for --json output mode
#[derive(Serialize, Deserialize)]
struct JsonResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl JsonResponse {
    fn success(data: serde_json::Value) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
        }
    }

    fn print(&self) {
        println!("{}", serde_json::to_string_pretty(self).unwrap());
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize the CLI
    let cli = Cli::parse();
    let json_mode = cli.json;

    if let Some(command) = cli.command {
        match command {
            Commands::Init { repo, force } => {
                handle_init_command(repo, force, json_mode).await?;
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
                let worktree_manager =
                    WorktreeManager::new(git_manager, db, config.clone(), config.repo_path.clone());

                match command {
                    Commands::Add {
                        worktree_type,
                        name,
                        repo,
                        pr,
                    } => {
                        handle_add_command(
                            &worktree_manager,
                            &worktree_type,
                            &name,
                            repo.as_deref(),
                            pr,
                            json_mode,
                        )
                        .await?;
                    }
                    Commands::Types(type_cmd) => {
                        handle_types_command(&worktree_manager, type_cmd, json_mode).await?;
                    }
                    Commands::Feat { name, repo } => {
                        eprintln!(
                            "⚠️  Warning: 'imi feat' is deprecated. Use 'imi add feat {}' instead.",
                            name
                        );
                        handle_feature_command(
                            &worktree_manager,
                            &name,
                            repo.as_deref(),
                            json_mode,
                        )
                        .await?;
                    }
                    Commands::Review { pr_number, repo } => {
                        handle_review_command(
                            &worktree_manager,
                            pr_number,
                            repo.as_deref(),
                            json_mode,
                        )
                        .await?;
                    }
                    Commands::Fix { name, repo } => {
                        handle_fix_command(&worktree_manager, &name, repo.as_deref(), json_mode)
                            .await?;
                    }
                    Commands::Aiops { name, repo } => {
                        handle_aiops_command(&worktree_manager, &name, repo.as_deref(), json_mode)
                            .await?;
                    }
                    Commands::Devops { name, repo } => {
                        handle_devops_command(&worktree_manager, &name, repo.as_deref(), json_mode)
                            .await?;
                    }
                    Commands::Trunk { repo } => {
                        handle_trunk_command(&worktree_manager, repo.as_deref(), json_mode).await?;
                    }
                    Commands::Status { repo } => {
                        handle_status_command(&worktree_manager, repo.as_deref(), json_mode)
                            .await?;
                    }
                    Commands::List {
                        repo,
                        worktrees,
                        projects,
                    } => {
                        handle_list_command(
                            &worktree_manager,
                            repo.as_deref(),
                            worktrees,
                            projects,
                            json_mode,
                        )
                        .await?;
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
                            json_mode,
                        )
                        .await?;
                    }
                    Commands::Monitor { repo } => {
                        handle_monitor_command(&worktree_manager, repo.as_deref(), json_mode)
                            .await?;
                    }
                    Commands::Sync { repo } => {
                        handle_sync_command(&worktree_manager, repo.as_deref(), json_mode).await?;
                    }
                    Commands::Init { .. } => {
                        // Already handled
                    }
                    Commands::Completion { shell } => {
                        handle_completion_command(&shell);
                    }
                    Commands::Prune {
                        repo,
                        dry_run,
                        force,
                    } => {
                        handle_prune_command(
                            &worktree_manager,
                            repo.as_deref(),
                            dry_run,
                            force,
                            json_mode,
                        )
                        .await?;
                    }
                    Commands::Close { name, repo } => {
                        handle_close_command(&worktree_manager, &name, repo.as_deref(), json_mode)
                            .await?;
                    }
                    Commands::Merge { name, repo } => {
                        handle_merge_command(&worktree_manager, &name, repo.as_deref(), json_mode)
                            .await?;
                    }
                    Commands::Go {
                        query,
                        repo,
                        worktrees_only,
                        include_inactive,
                    } => {
                        handle_go_command(
                            &worktree_manager,
                            query.as_deref(),
                            repo.as_deref(),
                            worktrees_only,
                            include_inactive,
                            json_mode,
                        )
                        .await?;
                    }
                    Commands::Project { command } => {
                        handle_project_command(command, json_mode).await?;
                    }
                    Commands::Clone { repo } => {
                        handle_clone_command(&repo).await?;
                    }
                }
            }
        }
    }

    Ok(())
}

async fn handle_clone_command(repo: &str) -> Result<()> {
    println!("{} Cloning repository: {}", "🔄".bright_cyan(), repo.bright_white());

    let config = Config::load().await?;
    let db = Database::new(&config.database_path).await?;
    let init_cmd = InitCommand::new(false, config, db);

    let result = init_cmd.clone_repository(repo).await?;

    if result.success {
        println!("{}", result.message.green());
    } else {
        println!("{}", result.message.red());
    }

    Ok(())
}

async fn handle_project_command(command: ProjectCommands, json_mode: bool) -> Result<()> {
    match command {
        ProjectCommands::Create {
            concept,
            prd,
            name,
            payload,
        } => {
            // Check GitHub authentication first
            if let Err(e) = github::check_auth() {
                if json_mode {
                    JsonResponse::error(format!("GitHub authentication failed: {}", e)).print();
                } else {
                    eprintln!("{}", format!("{}", e).red());
                    github::show_auth_help();
                }
                return Err(e);
            }

            // Build project config from input
            let config = if let Some(payload_str) = payload {
                ProjectConfig::from_json(&payload_str)?
            } else if let Some(prd_path) = prd {
                ProjectConfig::from_prd(&prd_path, name)?
            } else if let Some(concept_str) = concept {
                ProjectConfig::from_concept(&concept_str, name)?
            } else {
                let err_msg = "Must provide one of: --concept, --prd, or --payload";
                if json_mode {
                    JsonResponse::error(err_msg.to_string()).print();
                }
                return Err(anyhow::anyhow!(err_msg));
            };

            // Create the project
            let creator = ProjectCreator::new()?;
            let project_path = creator.create_project(config.clone()).await?;

            if json_mode {
                JsonResponse::success(serde_json::json!({
                    "message": "Project created successfully",
                    "project_name": config.name,
                    "project_path": project_path.display().to_string(),
                    "stack": format!("{:?}", config.stack),
                    "github_url": format!("https://github.com/{}/{}",
                        std::env::var("USER").unwrap_or_else(|_| "user".to_string()),
                        config.name)
                }))
                .print();
            }

            Ok(())
        }
    }
}

async fn handle_add_command(
    manager: &WorktreeManager,
    worktree_type: &str,
    name: &str,
    repo: Option<&str>,
    pr: Option<u32>,
    json_mode: bool,
) -> Result<()> {
    // Get the database from manager
    let db = &manager.db;

    // Validate worktree type exists
    let wt_type = db.get_worktree_type(worktree_type).await.context(format!(
        "Unknown worktree type '{}'. Run 'imi types' to see available types.",
        worktree_type
    ))?;

    if !json_mode {
        println!(
            "{} Creating {} worktree: {}",
            "🚀".bright_cyan(),
            wt_type.name,
            name.bright_green()
        );
    }

    // Handle review type specially (needs PR number)
    if worktree_type == "review" {
        let pr_number = match pr {
            Some(pr_number) => pr_number,
            None => name.parse::<u32>().context(
                "PR number required for review worktree. Use: imi add review <pr-number> or --pr <number>"
            )?,
        };
        return handle_review_command(manager, pr_number, repo, json_mode).await;
    }

    // Route to appropriate handler based on type
    match worktree_type {
        "feat" => handle_feature_command(manager, name, repo, json_mode).await,
        "fix" => handle_fix_command(manager, name, repo, json_mode).await,
        "aiops" => handle_aiops_command(manager, name, repo, json_mode).await,
        "devops" => handle_devops_command(manager, name, repo, json_mode).await,
        _ => {
            // Custom worktree type - use generic creation
            let worktree_path = manager
                .create_custom_worktree(name, worktree_type, repo)
                .await?;

            if json_mode {
                JsonResponse::success(serde_json::json!({
                    "worktree_path": worktree_path.display().to_string(),
                    "worktree_name": format!("{}-{}", worktree_type, name),
                    "worktree_type": worktree_type,
                    "message": format!("{} worktree created successfully", worktree_type)
                }))
                .print();
            } else {
                println!(
                    "{} {} worktree created at: {}",
                    "✅".bright_green(),
                    worktree_type,
                    worktree_path.display()
                );

                println!(
                    "\n{} To navigate to the worktree, run:\n   {}",
                    "💡".bright_yellow(),
                    format!("cd {}", worktree_path.display()).bright_cyan()
                );
            }

            Ok(())
        }
    }
}

async fn handle_types_command(
    manager: &WorktreeManager,
    type_cmd: TypeCommands,
    json_mode: bool,
) -> Result<()> {
    let db = &manager.db;

    match type_cmd {
        TypeCommands::List => {
            let types = db.list_worktree_types().await?;

            if json_mode {
                let types_json: Vec<_> = types
                    .iter()
                    .map(|t| {
                        serde_json::json!({
                            "name": t.name,
                            "branch_prefix": t.branch_prefix,
                            "worktree_prefix": t.worktree_prefix,
                            "description": t.description,
                            "is_builtin": t.is_builtin,
                        })
                    })
                    .collect();

                JsonResponse::success(serde_json::json!({
                    "types": types_json,
                    "count": types.len()
                }))
                .print();
            } else {
                println!("{} Available Worktree Types:\n", "📋".bright_cyan());

                for wt_type in types {
                    let builtin_badge = if wt_type.is_builtin {
                        "[builtin]".bright_blue()
                    } else {
                        "[custom]".bright_yellow()
                    };

                    println!(
                        "  {} {} - {}",
                        wt_type.name.bright_green(),
                        builtin_badge,
                        wt_type.description.as_deref().unwrap_or("No description")
                    );
                    println!(
                        "      Branch: {}  Worktree: {}",
                        wt_type.branch_prefix.bright_cyan(),
                        wt_type.worktree_prefix.bright_cyan()
                    );
                }

                println!("\n{} Usage: imi add <type> <name>", "💡".bright_yellow());
            }
        }
        TypeCommands::Add {
            name,
            branch_prefix,
            worktree_prefix,
            description,
        } => {
            if !json_mode {
                println!(
                    "{} Adding new worktree type: {}",
                    "➕".bright_cyan(),
                    name.bright_green()
                );
            }

            let wt_type = db
                .add_worktree_type(
                    &name,
                    branch_prefix.as_deref(),
                    worktree_prefix.as_deref(),
                    description.as_deref(),
                )
                .await?;

            if json_mode {
                JsonResponse::success(serde_json::json!({
                    "message": "Worktree type added successfully",
                    "type": {
                        "name": wt_type.name,
                        "branch_prefix": wt_type.branch_prefix,
                        "worktree_prefix": wt_type.worktree_prefix,
                        "description": wt_type.description,
                    }
                }))
                .print();
            } else {
                println!(
                    "{} Worktree type '{}' added successfully!",
                    "✅".bright_green(),
                    name
                );
                println!("  Branch prefix: {}", wt_type.branch_prefix.bright_cyan());
                println!(
                    "  Worktree prefix: {}",
                    wt_type.worktree_prefix.bright_cyan()
                );
                if let Some(desc) = wt_type.description {
                    println!("  Description: {}", desc);
                }
            }
        }
        TypeCommands::Remove { name } => {
            if !json_mode {
                println!(
                    "{} Removing worktree type: {}",
                    "🗑️".bright_red(),
                    name.bright_yellow()
                );
            }

            db.remove_worktree_type(&name).await?;

            if json_mode {
                JsonResponse::success(serde_json::json!({
                    "message": "Worktree type removed successfully",
                    "type_name": name
                }))
                .print();
            } else {
                println!(
                    "{} Worktree type '{}' removed successfully",
                    "✅".bright_green(),
                    name
                );
            }
        }
