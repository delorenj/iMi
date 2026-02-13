use anyhow::{Context, Result};
use clap::Parser;
use colored::*;
use serde::{Deserialize, Serialize};
use serde_json;
use std::path::PathBuf;

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

use cli::{Cli, Commands, MetadataCommands, ProjectCommands, RegistryCommands, TypeCommands};
use commands::project::{ProjectConfig, ProjectCreator};
use config::Config;
use database::Database;
use git::GitManager;
use init::InitCommand;
use local::LocalContext;
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
            Commands::MigrateOffice {
                repo,
                dry_run,
                force,
            } => {
                handle_migrate_office_command(repo, dry_run, force, json_mode).await?;
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
                let worktree_manager = WorktreeManager::new(
                    git_manager,
                    db.clone(),
                    config.clone(),
                    config.repo_path.clone(),
                );

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
                    Commands::Repair => {
                        handle_repair_command(&worktree_manager).await?;
                    }
                    Commands::Doctor { network, verbose } => {
                        handle_doctor_command(&db, network, verbose).await?;
                    }
                    Commands::Registry(cmd) => {
                        handle_registry_command(&db, &cmd).await?;
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
                        handle_merge_command(
                            &worktree_manager,
                            name.as_deref(),
                            repo.as_deref(),
                            json_mode,
                        )
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
                    Commands::Claim {
                        name,
                        yi_id,
                        repo,
                        force,
                    } => {
                        handle_claim_command(
                            &worktree_manager,
                            &name,
                            &yi_id,
                            repo.as_deref(),
                            force,
                            json_mode,
                        )
                        .await?;
                    }
                    Commands::VerifyLock { name, yi_id, repo } => {
                        handle_verify_lock_command(
                            &worktree_manager,
                            &name,
                            &yi_id,
                            repo.as_deref(),
                            json_mode,
                        )
                        .await?;
                    }
                    Commands::Release { name, yi_id, repo } => {
                        handle_release_command(
                            &worktree_manager,
                            &name,
                            &yi_id,
                            repo.as_deref(),
                            json_mode,
                        )
                        .await?;
                    }
                    Commands::Metadata(cmd) => {
                        handle_metadata_command(&worktree_manager, cmd, json_mode).await?;
                    }
                    Commands::MigrateOffice { .. } => {
                        // Already handled before loading repository-scoped managers
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
    json_mode: bool,
) -> Result<()> {
    if !json_mode {
        println!(
            "{} Creating feature worktree: {}",
            "🚀".bright_cyan(),
            name.bright_green()
        );
    }

    match manager.create_feature_worktree(name, repo).await {
        Ok(worktree_path) => {
            if json_mode {
                JsonResponse::success(serde_json::json!({
                    "worktree_path": worktree_path.display().to_string(),
                    "worktree_name": format!("feat-{}", name),
                    "message": "Feature worktree created successfully"
                }))
                .print();
            } else {
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
        }
        Err(e) => {
            if json_mode {
                JsonResponse::error(e.to_string()).print();
                return Err(e);
            }

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
    json_mode: bool,
) -> Result<()> {
    if !json_mode {
        println!(
            "{} Creating review worktree for PR: {}",
            "🔍".bright_yellow(),
            pr_number.to_string().bright_green()
        );
    }

    let worktree_path = manager.create_review_worktree(pr_number, repo).await?;

    if json_mode {
        JsonResponse::success(serde_json::json!({
            "worktree_path": worktree_path.display().to_string(),
            "pr_number": pr_number,
            "message": "Review worktree created successfully"
        }))
        .print();
    } else {
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
    }

    Ok(())
}

async fn handle_fix_command(
    manager: &WorktreeManager,
    name: &str,
    repo: Option<&str>,
    json_mode: bool,
) -> Result<()> {
    if !json_mode {
        println!(
            "{} Creating fix worktree: {}",
            "🔧".bright_red(),
            name.bright_green()
        );
    }

    let worktree_path = manager.create_fix_worktree(name, repo).await?;

    if json_mode {
        JsonResponse::success(serde_json::json!({
            "worktree_path": worktree_path.display().to_string(),
            "worktree_name": format!("fix-{}", name),
            "message": "Fix worktree created successfully"
        }))
        .print();
    } else {
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
    }

    Ok(())
}

async fn handle_aiops_command(
    manager: &WorktreeManager,
    name: &str,
    repo: Option<&str>,
    json_mode: bool,
) -> Result<()> {
    if !json_mode {
        println!(
            "{} Creating aiops worktree: {}",
            "🤖".bright_magenta(),
            name.bright_green()
        );
    }

    let worktree_path = manager.create_aiops_worktree(name, repo).await?;

    if json_mode {
        JsonResponse::success(serde_json::json!({
            "worktree_path": worktree_path.display().to_string(),
            "worktree_name": format!("aiops-{}", name),
            "message": "Aiops worktree created successfully"
        }))
        .print();
    } else {
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
    }

    Ok(())
}

async fn handle_devops_command(
    manager: &WorktreeManager,
    name: &str,
    repo: Option<&str>,
    json_mode: bool,
) -> Result<()> {
    if !json_mode {
        println!(
            "{} Creating devops worktree: {}",
            "⚙️".bright_blue(),
            name.bright_green()
        );
    }

    let worktree_path = manager.create_devops_worktree(name, repo).await?;

    if json_mode {
        JsonResponse::success(serde_json::json!({
            "worktree_path": worktree_path.display().to_string(),
            "worktree_name": format!("devops-{}", name),
            "message": "Devops worktree created successfully"
        }))
        .print();
    } else {
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
    }

    Ok(())
}

async fn handle_trunk_command(
    manager: &WorktreeManager,
    repo: Option<&str>,
    json_mode: bool,
) -> Result<()> {
    if !json_mode {
        println!("{} Switching to trunk worktree", "🌳".bright_green());
    }

    let worktree_path = manager.get_trunk_worktree(repo).await?;

    if json_mode {
        JsonResponse::success(serde_json::json!({
            "worktree_path": worktree_path.display().to_string(),
            "message": "Trunk worktree located"
        }))
        .print();
    } else {
        // Print command to change directory (processes can't change parent shell's directory)
        println!(
            "{} To navigate to trunk, run:\n   {}",
            "💡".bright_yellow(),
            format!("cd {}", worktree_path.display()).bright_cyan()
        );
    }

    Ok(())
}

async fn handle_status_command(
    manager: &WorktreeManager,
    repo: Option<&str>,
    json_mode: bool,
) -> Result<()> {
    if json_mode {
        // For JSON mode, we need to capture the status data instead of printing
        // This would require modifying WorktreeManager.show_status() to return data
        // For now, we'll use a simple response
        JsonResponse::success(serde_json::json!({
            "message": "Status command in JSON mode not yet fully implemented",
            "note": "Use non-JSON mode for detailed status"
        }))
        .print();
    } else {
        println!("{} Worktree Status", "📊".bright_cyan());
        manager.show_status(repo).await?;
    }
    Ok(())
}

async fn handle_list_command(
    manager: &WorktreeManager,
    repo: Option<&str>,
    worktrees: bool,
    projects: bool,
    json_mode: bool,
) -> Result<()> {
    if json_mode {
        // For JSON mode, would need to capture list data
        // For now, simple response
        JsonResponse::success(serde_json::json!({
            "message": "List command in JSON mode not yet fully implemented",
            "note": "Use non-JSON mode for detailed listing"
        }))
        .print();
    } else {
        manager.list_smart(repo, worktrees, projects).await?;
    }
    Ok(())
}

async fn handle_remove_command(
    manager: &WorktreeManager,
    name: &str,
    repo: Option<&str>,
    keep_branch: bool,
    keep_remote: bool,
    json_mode: bool,
) -> Result<()> {
    if !json_mode {
        println!(
            "{} Removing worktree: {}",
            "🗑️".bright_red(),
            name.bright_yellow()
        );
    }

    manager
        .remove_worktree(name, repo, keep_branch, keep_remote)
        .await?;

    if json_mode {
        JsonResponse::success(serde_json::json!({
            "worktree_name": name,
            "message": "Worktree removed successfully"
        }))
        .print();
    } else {
        println!("{} Worktree removed successfully", "✅".bright_green());
    }
    Ok(())
}

async fn handle_monitor_command(
    manager: &WorktreeManager,
    repo: Option<&str>,
    json_mode: bool,
) -> Result<()> {
    if json_mode {
        JsonResponse::error(
            "Monitor command does not support JSON mode (interactive mode only)".to_string(),
        )
        .print();
        return Err(anyhow::anyhow!(
            "Monitor command requires interactive terminal"
        ));
    }

    println!("{} Starting real-time monitoring...", "👁️".bright_purple());
    manager.start_monitoring(repo).await?;
    Ok(())
}

async fn handle_sync_command(
    manager: &WorktreeManager,
    repo: Option<&str>,
    json_mode: bool,
) -> Result<()> {
    if !json_mode {
        println!(
            "{} Syncing database with Git worktrees...",
            "🔄".bright_cyan()
        );
    }

    manager.sync_with_git(repo).await?;

    if json_mode {
        JsonResponse::success(serde_json::json!({
            "message": "Database synced successfully"
        }))
        .print();
    }
    Ok(())
}

async fn handle_repair_command(manager: &WorktreeManager) -> Result<()> {
    println!(
        "{} Repairing repository paths in database...",
        "🔧".bright_cyan()
    );
    println!();

    manager.repair_all_repository_paths().await?;

    println!();
    println!("{} Repair complete!", "✓".bright_green());
    Ok(())
}

async fn handle_doctor_command(db: &Database, network: bool, verbose: bool) -> Result<()> {
    use commands::doctor::{print_report, run_doctor, DoctorOpts};

    let opts = DoctorOpts { network, verbose };
    let checks = run_doctor(db.pool(), opts).await?;
    print_report(&checks);

    Ok(())
}

async fn handle_registry_command(db: &Database, cmd: &RegistryCommands) -> Result<()> {
    use commands::registry;

    match cmd {
        RegistryCommands::Sync { scan_root } => {
            let path = scan_root.as_ref().map(|s| std::path::Path::new(s));

            registry::sync_filesystem(db.pool(), path).await?;
        }
        RegistryCommands::Stats => {
            // Query registry stats
            let stats = sqlx::query_as::<
                _,
                (
                    Option<i64>,
                    Option<i64>,
                    Option<i64>,
                    Option<i64>,
                    Option<i64>,
                    Option<i64>,
                    Option<i64>,
                ),
            >(
                r#"
                SELECT
                    total_projects,
                    active_projects,
                    total_worktrees,
                    active_worktrees,
                    in_flight_worktrees,
                    total_activities,
                    activities_last_24h
                FROM get_registry_stats()
                "#,
            )
            .fetch_one(db.pool())
            .await?;

            println!("\n{}", "━".repeat(60).bright_black());
            println!("{}", "iMi Registry Statistics".bold().bright_white());
            println!("{}\n", "━".repeat(60).bright_black());
            println!(
                "Total projects: {}",
                stats.0.unwrap_or(0).to_string().green()
            );
            println!(
                "Active projects: {}",
                stats.1.unwrap_or(0).to_string().green()
            );
            println!(
                "Total worktrees: {}",
                stats.2.unwrap_or(0).to_string().cyan()
            );
            println!(
                "Active worktrees: {}",
                stats.3.unwrap_or(0).to_string().cyan()
            );
            println!(
                "In-flight worktrees: {}",
                stats.4.unwrap_or(0).to_string().yellow()
            );
            println!(
                "Total activities: {}",
                stats.5.unwrap_or(0).to_string().bright_black()
            );
            println!(
                "Activities (24h): {}",
                stats.6.unwrap_or(0).to_string().bright_black()
            );
            println!();
        }
    }

    Ok(())
}

async fn handle_init_command(repo: Option<String>, force: bool, json_mode: bool) -> Result<()> {
    let config = Config::load().await?;
    let db = Database::new(&config.database_path).await?;
    let init_cmd = InitCommand::new(force, config, db);

    // Check if repo argument looks like a GitHub repo (owner/repo format)
    if let Some(ref repo_arg) = repo {
        if repo_arg.contains('/') && !repo_arg.contains(':') {
            // Looks like owner/repo format - clone from GitHub
            let result = init_cmd.clone_from_github(repo_arg).await?;

            if json_mode {
                if result.success {
                    JsonResponse::success(serde_json::json!({
                        "message": result.message,
                        "repo": repo_arg
                    }))
                    .print();
                } else {
                    JsonResponse::error(result.message).print();
                }
            } else {
                if result.success {
                    println!("{}", result.message.green());
                } else {
                    println!("{}", result.message.red());
                }
            }

            return Ok(());
        } else {
            // Treat as a local path
            let path = std::path::PathBuf::from(repo_arg);
            let result = init_cmd.execute(Some(&path)).await?;

            if json_mode {
                if result.success {
                    JsonResponse::success(serde_json::json!({
                        "message": result.message,
                        "path": path.display().to_string()
                    }))
                    .print();
                } else {
                    JsonResponse::error(result.message).print();
                }
            } else {
                if result.success {
                    println!("{}", result.message.green());
                } else {
                    println!("{}", result.message.red());
                }
            }

            return Ok(());
        }
    }

    // No repo argument - normal init
    let result = init_cmd.execute(None).await?;

    if json_mode {
        if result.success {
            JsonResponse::success(serde_json::json!({
                "message": result.message
            }))
            .print();
        } else {
            JsonResponse::error(result.message).print();
        }
    } else {
        if result.success {
            println!("{}", result.message.green());
        } else {
            println!("{}", result.message.red());
        }
    }

    Ok(())
}

async fn handle_migrate_office_command(
    repo: Option<String>,
    dry_run: bool,
    force: bool,
    json_mode: bool,
) -> Result<()> {
    let config = Config::load().await?;
    let db = Database::new(&config.database_path).await?;
    let init_cmd = InitCommand::new(force, config, db);

    let summary = init_cmd
        .migrate_office_layout(repo.as_deref(), dry_run)
        .await?;

    if json_mode {
        JsonResponse::success(serde_json::json!({
            "processed": summary.processed,
            "migrated": summary.migrated,
            "skipped": summary.skipped,
            "failed": summary.failed,
            "dry_run": summary.dry_run,
            "results": summary.results
        }))
        .print();
    } else {
        println!();
        println!(
            "{} {}",
            "🏢".bright_cyan(),
            "Office migration summary".bright_cyan().bold()
        );
        println!(
            "   processed={} migrated={} skipped={} failed={} dry_run={}",
            summary.processed, summary.migrated, summary.skipped, summary.failed, summary.dry_run
        );

        for result in &summary.results {
            let status = match result.status.as_str() {
                "migrated" => "migrated".bright_green(),
                "skipped" => "skipped".bright_yellow(),
                "failed" => "failed".bright_red(),
                _ => result.status.bright_black(),
            };

            println!(
                "   [{}] {}: {} -> {}",
                status, result.repo_name, result.source_trunk, result.target_trunk
            );
            println!("      {}", result.message);
            for warning in &result.warnings {
                println!("      warning: {}", warning);
            }
        }
    }

    if summary.failed > 0 {
        return Err(anyhow::anyhow!(
            "Office migration finished with {} failed repository migrations",
            summary.failed
        ));
    }

    Ok(())
}

fn parse_metadata_value(raw: &str) -> serde_json::Value {
    serde_json::from_str(raw).unwrap_or_else(|_| serde_json::Value::String(raw.to_string()))
}

async fn handle_metadata_command(
    manager: &WorktreeManager,
    command: MetadataCommands,
    json_mode: bool,
) -> Result<()> {
    match command {
        MetadataCommands::Set {
            worktree,
            key,
            value,
            repo,
        } => {
            let worktree_entry = match manager
                .get_worktree_by_name(&worktree, repo.as_deref())
                .await?
            {
                Some(wt) => wt,
                None => {
                    let error_msg = format!("Worktree '{}' not found", worktree);
                    if json_mode {
                        JsonResponse::error(error_msg.clone()).print();
                    } else {
                        eprintln!("{}", error_msg.red());
                    }
                    return Err(anyhow::anyhow!(error_msg));
                }
            };

            let parsed_value = parse_metadata_value(&value);
            manager
                .db
                .set_worktree_metadata(&worktree_entry.id, &key, parsed_value.clone())
                .await?;

            let stored_value = manager
                .db
                .get_worktree_metadata(&worktree_entry.id, Some(&key))
                .await?;

            if json_mode {
                JsonResponse::success(serde_json::json!({
                    "worktree_id": worktree_entry.id,
                    "worktree_name": worktree_entry.name,
                    "key": key,
                    "value": stored_value,
                    "message": "Metadata updated"
                }))
                .print();
            } else {
                println!(
                    "{} Metadata updated for '{}': {}",
                    "✅".bright_green(),
                    worktree_entry.name.bright_cyan(),
                    key.bright_yellow()
                );
                println!("   {}", serde_json::to_string_pretty(&stored_value)?);
            }
        }
        MetadataCommands::Get {
            worktree,
            key,
            repo,
        } => {
            let worktree_entry = match manager
                .get_worktree_by_name(&worktree, repo.as_deref())
                .await?
            {
                Some(wt) => wt,
                None => {
                    let error_msg = format!("Worktree '{}' not found", worktree);
                    if json_mode {
                        JsonResponse::error(error_msg.clone()).print();
                    } else {
                        eprintln!("{}", error_msg.red());
                    }
                    return Err(anyhow::anyhow!(error_msg));
                }
            };

            let metadata = manager
                .db
                .get_worktree_metadata(&worktree_entry.id, key.as_deref())
                .await?;

            if json_mode {
                JsonResponse::success(serde_json::json!({
                    "worktree_id": worktree_entry.id,
                    "worktree_name": worktree_entry.name,
                    "key": key,
                    "value": metadata
                }))
                .print();
            } else {
                println!(
                    "{} Metadata for '{}'",
                    "📌".bright_cyan(),
                    worktree_entry.name.bright_cyan()
                );
                println!("{}", serde_json::to_string_pretty(&metadata)?);
            }
        }
    }

    Ok(())
}

async fn handle_prune_command(
    manager: &WorktreeManager,
    repo: Option<&str>,
    dry_run: bool,
    force: bool,
    json_mode: bool,
) -> Result<()> {
    if !json_mode {
        println!(
            "{} Cleaning up stale worktree references",
            "🧹".bright_cyan()
        );
    }

    manager.prune_stale_worktrees(repo, dry_run, force).await?;

    if json_mode {
        JsonResponse::success(serde_json::json!({
            "message": "Cleanup complete",
            "dry_run": dry_run
        }))
        .print();
    } else {
        println!("{} Cleanup complete", "✅".bright_green());
    }
    Ok(())
}

async fn handle_close_command(
    manager: &WorktreeManager,
    name: &str,
    repo: Option<&str>,
    json_mode: bool,
) -> Result<()> {
    if !json_mode {
        println!(
            "{} Closing worktree: {}",
            "🚫".bright_yellow(),
            name.bright_yellow()
        );
    }

    manager.close_worktree(name, repo).await?;

    if json_mode {
        match manager.get_trunk_worktree(repo).await {
            Ok(trunk_path) => {
                JsonResponse::success(serde_json::json!({
                    "message": "Worktree closed successfully",
                    "worktree_name": name,
                    "trunk_path": trunk_path.display().to_string()
                }))
                .print();
            }
            Err(_) => {
                JsonResponse::success(serde_json::json!({
                    "message": "Worktree closed successfully",
                    "worktree_name": name,
                    "warning": "Unable to locate trunk worktree"
                }))
                .print();
            }
        }
    } else {
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
    }

    Ok(())
}

async fn handle_go_command(
    manager: &WorktreeManager,
    query: Option<&str>,
    repo: Option<&str>,
    worktrees_only: bool,
    include_inactive: bool,
    json_mode: bool,
) -> Result<()> {
    // Perform fuzzy search and get best match or show interactive picker
    let target_path = manager
        .fuzzy_navigate(query, repo, worktrees_only, include_inactive)
        .await?;

    if json_mode {
        JsonResponse::success(serde_json::json!({
            "target_path": target_path.display().to_string()
        }))
        .print();
    } else {
        // Output only the path to stdout for shell wrapper to capture
        // All other output must go to stderr to avoid polluting the path
        print!("{}", target_path.display());
    }

    Ok(())
}

async fn handle_merge_command(
    manager: &WorktreeManager,
    name: Option<&str>,
    repo: Option<&str>,
    json_mode: bool,
) -> Result<()> {
    let worktree_name = match name {
        Some(n) => n.to_string(),
        None => {
            // Auto-detect worktree name from current directory
            let current_dir = std::env::current_dir()?;
            let dir_name = current_dir
                .file_name()
                .ok_or_else(|| anyhow::anyhow!("Could not determine directory name"))?
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid directory name"))?;
            dir_name.to_string()
        }
    };

    if !json_mode {
        println!(
            "{} Merging worktree: {}",
            "🔀".bright_cyan(),
            worktree_name.bright_yellow()
        );
    }

    manager.merge_worktree(&worktree_name, repo).await?;

    if json_mode {
        JsonResponse::success(serde_json::json!({
            "message": "Worktree merged successfully",
            "worktree_name": worktree_name
        }))
        .print();
    }

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

async fn handle_claim_command(
    manager: &WorktreeManager,
    name: &str,
    yi_id: &str,
    repo: Option<&str>,
    force: bool,
    json_mode: bool,
) -> Result<()> {
    // Resolve worktree by name
    let worktree = match manager.get_worktree_by_name(name, repo).await? {
        Some(wt) => wt,
        None => {
            let error_msg = format!("Worktree '{}' not found", name);
            if json_mode {
                JsonResponse::error(error_msg.clone()).print();
            } else {
                eprintln!("{}", error_msg.red());
            }
            return Err(anyhow::anyhow!(error_msg));
        }
    };

    // Check if already claimed
    if let Some(current_agent) = &worktree.agent_id {
        if !force {
            let error_msg = format!(
                "Worktree '{}' is already claimed by agent '{}'",
                name, current_agent
            );
            if json_mode {
                JsonResponse::error(error_msg.clone()).print();
            } else {
                eprintln!("{}", error_msg.red());
                eprintln!(
                    "Use --force to override, or release with: imi release {} --yi-id {}",
                    name, current_agent
                );
            }
            return Err(anyhow::anyhow!(error_msg));
        } else if json_mode {
            eprintln!("Warning: Force claiming from agent '{}'", current_agent);
        } else {
            println!(
                "{} Force claiming from agent '{}'",
                "⚠️".yellow(),
                current_agent
            );
        }
    }

    // Claim the worktree in database
    manager.db.claim_worktree(&worktree.id, yi_id).await?;

    // Create lock file in .iMi/presence/
    let worktree_path = PathBuf::from(&worktree.path);
    let repo_root = worktree_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Invalid worktree path"))?;

    let local_ctx = LocalContext::new(repo_root);
    let imi_dir = repo_root.join(".iMi");
    local_ctx.create_lock_file(&imi_dir, name, yi_id).await?;

    // Log activity
    manager
        .db
        .log_agent_activity(
            yi_id,
            &worktree.id,
            "claimed",
            None,
            "Agent claimed worktree",
        )
        .await?;

    if json_mode {
        JsonResponse::success(serde_json::json!({
            "worktree_id": worktree.id,
            "worktree_name": name,
            "yi_id": yi_id,
            "path": worktree.path,
            "claimed_at": chrono::Utc::now().to_rfc3339(),
        }))
        .print();
    } else {
        println!(
            "{} Successfully claimed worktree '{}'",
            "✅".bright_green(),
            name
        );
        println!(
            "   {} Agent ID: {}",
            "🔑".bright_black(),
            yi_id.bright_cyan()
        );
        println!(
            "   {} Worktree ID: {}",
            "🆔".bright_black(),
            worktree.id.to_string().bright_black()
        );
        println!("   {} Path: {}", "📂".bright_black(), worktree.path);
    }

    Ok(())
}

async fn handle_verify_lock_command(
    manager: &WorktreeManager,
    name: &str,
    yi_id: &str,
    repo: Option<&str>,
    json_mode: bool,
) -> Result<()> {
    // Resolve worktree by name
    let worktree = match manager.get_worktree_by_name(name, repo).await? {
        Some(wt) => wt,
        None => {
            let error_msg = format!("Worktree '{}' not found", name);
            if json_mode {
                JsonResponse::error(error_msg.clone()).print();
            } else {
                eprintln!("{}", error_msg.red());
            }
            std::process::exit(2);
        }
    };

    // Get repository root to find .iMi directory
    let worktree_path = PathBuf::from(&worktree.path);
    let repo_root = worktree_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Invalid worktree path"))?;

    let lock_file = repo_root
        .join(".iMi/presence")
        .join(format!("{}.lock", name));

    // Check if lock file exists
    if !lock_file.exists() {
        // No lock exists - verification succeeds
        if json_mode {
            JsonResponse::success(serde_json::json!({
                "verified": true,
                "locked": false,
                "worktree_name": name,
                "message": "No lock exists - worktree is available"
            }))
            .print();
        } else {
            println!("{} Worktree '{}' is not locked", "✅".bright_green(), name);
        }
        std::process::exit(0);
    }

    // Read and parse lock file
    let lock_content = match std::fs::read_to_string(&lock_file) {
        Ok(content) => content,
        Err(e) => {
            let error_msg = format!("Failed to read lock file: {}", e);
            if json_mode {
                JsonResponse::error(error_msg.clone()).print();
            } else {
                eprintln!("{} {}", "❌".bright_red(), error_msg);
            }
            std::process::exit(2);
        }
    };

    let lock_data: serde_json::Value = match serde_json::from_str(&lock_content) {
        Ok(data) => data,
        Err(e) => {
            let error_msg = format!("Failed to parse lock file: {}", e);
            if json_mode {
                JsonResponse::error(error_msg.clone()).print();
            } else {
                eprintln!("{} {}", "❌".bright_red(), error_msg);
            }
            std::process::exit(2);
        }
    };

    // Extract agent_id from lock file
    let lock_owner = lock_data["agent_id"].as_str().unwrap_or("");

    // Verify ownership
    if lock_owner == yi_id {
        // Owned by this agent - verification succeeds
        if json_mode {
            JsonResponse::success(serde_json::json!({
                "verified": true,
                "locked": true,
                "owner": lock_owner,
                "worktree_name": name,
                "message": "Lock verified - owned by this agent"
            }))
            .print();
        } else {
            println!(
                "{} Worktree '{}' is locked by you ({})",
                "✅".bright_green(),
                name,
                yi_id.bright_cyan()
            );
        }
        std::process::exit(0);
    } else {
        // Locked by different agent - verification fails
        let claimed_at = lock_data["claimed_at"].as_str().unwrap_or("unknown");
        let hostname = lock_data["hostname"].as_str().unwrap_or("unknown");

        if json_mode {
            JsonResponse::error(format!(
                "Worktree locked by different agent: {}",
                lock_owner
            ))
            .print();
            // Also output lock details in data field
            println!(
                "{}",
                serde_json::json!({
                    "verified": false,
                    "locked": true,
                    "owner": lock_owner,
                    "claimed_at": claimed_at,
                    "hostname": hostname,
                    "worktree_name": name,
                })
            );
        } else {
            eprintln!(
                "{} Worktree '{}' is locked by {}",
                "❌".bright_red(),
                name,
                lock_owner.bright_yellow()
            );
            eprintln!("   {} Claimed at: {}", "🕒".bright_black(), claimed_at);
            eprintln!("   {} Hostname: {}", "🖥️".bright_black(), hostname);
            eprintln!(
                "\n   Use 'imi claim {} --yi-id {} --force' to override",
                name, lock_owner
            );
        }
        std::process::exit(1);
    }
}

async fn handle_release_command(
    manager: &WorktreeManager,
    name: &str,
    yi_id: &str,
    repo: Option<&str>,
    json_mode: bool,
) -> Result<()> {
    // Resolve worktree by name
    let worktree = match manager.get_worktree_by_name(name, repo).await? {
        Some(wt) => wt,
        None => {
            let error_msg = format!("Worktree '{}' not found", name);
            if json_mode {
                JsonResponse::error(error_msg.clone()).print();
            } else {
                eprintln!("{}", error_msg.red());
            }
            return Err(anyhow::anyhow!(error_msg));
        }
    };

    // Check ownership - must be claimed by this agent
    match &worktree.agent_id {
        Some(owner) if owner == yi_id => {
            // Ownership verified
        }
        Some(owner) => {
            let error_msg = format!(
                "Cannot release: worktree '{}' is owned by agent '{}', not '{}'",
                name, owner, yi_id
            );
            if json_mode {
                JsonResponse::error(error_msg.clone()).print();
            } else {
                eprintln!("{}", error_msg.red());
                eprintln!("   Only the owning agent can release this worktree");
            }
            return Err(anyhow::anyhow!(error_msg));
        }
        None => {
            let error_msg = format!(
                "Cannot release: worktree '{}' is not claimed by any agent",
                name
            );
            if json_mode {
                JsonResponse::error(error_msg.clone()).print();
            } else {
                eprintln!("{}", error_msg.yellow());
                eprintln!(
                    "   Use 'imi claim {} --yi-id {}' to claim it first",
                    name, yi_id
                );
            }
            return Err(anyhow::anyhow!(error_msg));
        }
    }

    // Check git status - must be clean (no uncommitted changes)
    let worktree_path = PathBuf::from(&worktree.path);
    let git_status = manager.git.get_worktree_status(&worktree_path)?;

    if !git_status.clean {
        let error_msg = format!(
            "Cannot release: worktree '{}' has uncommitted changes",
            name
        );

        if json_mode {
            JsonResponse::error(error_msg.clone()).print();
            println!(
                "{}",
                serde_json::json!({
                    "modified_files": git_status.modified_files,
                    "new_files": git_status.new_files,
                    "deleted_files": git_status.deleted_files,
                })
            );
        } else {
            eprintln!("{}", error_msg.red());
            eprintln!("\n   Modified files:");
            for file in &git_status.modified_files {
                eprintln!("      M {}", file.yellow());
            }
            for file in &git_status.new_files {
                eprintln!("      A {}", file.green());
            }
            for file in &git_status.deleted_files {
                eprintln!("      D {}", file.red());
            }
            eprintln!("\n   Commit or discard changes before releasing");
        }
        return Err(anyhow::anyhow!(error_msg));
    }

    // Release worktree in database
    manager.db.release_worktree(&worktree.id, yi_id).await?;

    // Remove lock file
    let repo_root = worktree_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Invalid worktree path"))?;

    let local_ctx = LocalContext::new(repo_root);
    let imi_dir = repo_root.join(".iMi");
    local_ctx.remove_lock_file(&imi_dir, name).await?;

    // Log activity
    manager
        .db
        .log_agent_activity(
            yi_id,
            &worktree.id,
            "released",
            None,
            "Agent released worktree",
        )
        .await?;

    // Output success
    if json_mode {
        JsonResponse::success(serde_json::json!({
            "worktree_id": worktree.id,
            "worktree_name": name,
            "yi_id": yi_id,
            "path": worktree.path,
            "released_at": chrono::Utc::now().to_rfc3339(),
        }))
        .print();
    } else {
        println!(
            "{} Successfully released worktree '{}'",
            "✅".bright_green(),
            name
        );
        println!(
            "   {} Agent ID: {}",
            "🔓".bright_black(),
            yi_id.bright_cyan()
        );
        println!(
            "   {} Worktree ID: {}",
            "🆔".bright_black(),
            worktree.id.to_string().bright_black()
        );
        println!("   {} Path: {}", "📂".bright_black(), worktree.path);
        println!("\n   Worktree is now available for other agents to claim");
    }

    Ok(())
}
