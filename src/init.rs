use anyhow::{anyhow, Context, Result};
use colored::*;
use dialoguer::{Confirm, Select};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};
use tokio::fs;
use uuid::Uuid;

use crate::config::Config;
use crate::database::Database;
use crate::git::GitManager;

#[derive(Debug, Clone)]
pub struct InitResult {
    pub success: bool,
    pub message: String,
}

impl InitResult {
    pub fn success(message: String) -> Self {
        Self {
            success: true,
            message,
        }
    }

    pub fn failure(message: String) -> Self {
        Self {
            success: false,
            message,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfficeMigrationResult {
    pub repo_name: String,
    pub source_trunk: String,
    pub target_trunk: String,
    pub moved_worktrees: usize,
    pub updated_worktrees: usize,
    pub status: String,
    pub message: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfficeMigrationSummary {
    pub dry_run: bool,
    pub processed: usize,
    pub migrated: usize,
    pub skipped: usize,
    pub failed: usize,
    pub results: Vec<OfficeMigrationResult>,
}

/// Project metadata written to .iMi/project.json
/// Provides fast filesystem access for shell integrations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetadata {
    pub project_id: Uuid,
    pub name: String,
    pub remote_origin: String,
    pub default_branch: String,
    pub trunk_path: String,
    pub description: Option<String>,
}

#[derive(Clone)]
pub struct InitCommand {
    pub force: bool,
    config: Config,
    db: Database,
}

impl InitCommand {
    pub fn new(force: bool, config: Config, db: Database) -> Self {
        Self { force, config, db }
    }

    pub async fn execute(&self, path: Option<&Path>) -> Result<InitResult> {
        let git_manager = GitManager::new();
        let current_dir = match path {
            Some(p) => p.to_path_buf(),
            None => env::current_dir().context("Failed to get current directory")?,
        };

        if git_manager.is_in_repository(&current_dir) {
            self.handle_inside_repo(&current_dir).await
        } else {
            // Outside any repo - check if we should show TUI selector
            if path.is_none() {
                // User didn't specify a path, so check if we have registered repos
                let repos = self.db.list_repositories().await?;

                if !repos.is_empty() && !self.force {
                    // Show TUI selector
                    println!();
                    println!(
                        "{} {}",
                        "📦".bright_cyan(),
                        "Available Repositories:".bright_cyan().bold()
                    );
                    println!();

                    let repo_names: Vec<String> = repos
                        .iter()
                        .map(|r| format!("{} ({})", r.name.bright_green(), r.path.dimmed()))
                        .collect();

                    let selection = Select::new()
                        .with_prompt("Select a repository to initialize")
                        .items(&repo_names)
                        .default(0)
                        .interact_opt()?;

                    if let Some(idx) = selection {
                        let selected_repo = &repos[idx];
                        let repo_path = PathBuf::from(&selected_repo.path);

                        // Change to that directory and initialize
                        return self.handle_inside_repo(&repo_path).await;
                    } else {
                        return Ok(InitResult::failure(
                            "Repository selection cancelled.".to_string(),
                        ));
                    }
                }
            }

            self.handle_outside_repo().await
        }
    }

    pub async fn migrate_office_layout(
        &self,
        repo_filter: Option<&str>,
        dry_run: bool,
    ) -> Result<OfficeMigrationSummary> {
        let repositories = if let Some(repo_name) = repo_filter {
            match self.db.get_repository(repo_name).await? {
                Some(repo) => vec![repo],
                None => {
                    return Err(anyhow!(
                        "Repository '{}' is not registered in iMi",
                        repo_name
                    ));
                }
            }
        } else {
            self.db.list_repositories().await?
        };

        let mut summary = OfficeMigrationSummary {
            dry_run,
            processed: 0,
            migrated: 0,
            skipped: 0,
            failed: 0,
            results: Vec::new(),
        };

        for repo in repositories {
            summary.processed += 1;
            match self.migrate_single_repository(&repo, dry_run).await {
                Ok(result) => {
                    match result.status.as_str() {
                        "migrated" => summary.migrated += 1,
                        "skipped" => summary.skipped += 1,
                        _ => {}
                    }
                    summary.results.push(result);
                }
                Err(err) => {
                    summary.failed += 1;
                    summary.results.push(OfficeMigrationResult {
                        repo_name: repo.name.clone(),
                        source_trunk: repo.path.clone(),
                        target_trunk: self.config.get_trunk_path(&repo.name).display().to_string(),
                        moved_worktrees: 0,
                        updated_worktrees: 0,
                        status: "failed".to_string(),
                        message: err.to_string(),
                        warnings: Vec::new(),
                    });
                }
            }
        }

        Ok(summary)
    }

    async fn migrate_single_repository(
        &self,
        repo: &crate::database::Project,
        dry_run: bool,
    ) -> Result<OfficeMigrationResult> {
        let source_trunk = PathBuf::from(&repo.path);
        let target_container = self.config.get_repo_path(&repo.name);
        let target_trunk = self.config.get_trunk_path(&repo.name);
        let worktrees = self.db.list_worktrees(Some(&repo.name)).await?;
        let tracked_worktree_names: Vec<String> =
            worktrees.iter().map(|wt| wt.name.clone()).collect();

        let trunk_needs_move = !Self::paths_match(&source_trunk, &target_trunk);
        let mut planned_worktree_moves = 0usize;

        for worktree in &worktrees {
            let source = PathBuf::from(&worktree.path);
            let target = target_container.join(&worktree.name);
            if !Self::paths_match(&source, &target) {
                planned_worktree_moves += 1;
            }
        }

        if !trunk_needs_move && planned_worktree_moves == 0 {
            return Ok(OfficeMigrationResult {
                repo_name: repo.name.clone(),
                source_trunk: source_trunk.display().to_string(),
                target_trunk: target_trunk.display().to_string(),
                moved_worktrees: 0,
                updated_worktrees: 0,
                status: "skipped".to_string(),
                message: "Already in office layout".to_string(),
                warnings: Vec::new(),
            });
        }

        if dry_run {
            return Ok(OfficeMigrationResult {
                repo_name: repo.name.clone(),
                source_trunk: source_trunk.display().to_string(),
                target_trunk: target_trunk.display().to_string(),
                moved_worktrees: planned_worktree_moves,
                updated_worktrees: 0,
                status: "migrated".to_string(),
                message: format!(
                    "Dry run: trunk move={} planned worktree moves={}",
                    trunk_needs_move, planned_worktree_moves
                ),
                warnings: Vec::new(),
            });
        }

        fs::create_dir_all(&target_container)
            .await
            .context("Failed to create office container directory")?;

        let mut warnings = Vec::new();
        let mut moved_worktrees = 0usize;
        let mut updated_worktrees = 0usize;

        if trunk_needs_move {
            let source_exists = source_trunk.exists();
            let target_exists = target_trunk.exists();

            match (source_exists, target_exists) {
                (true, true) => {
                    if self.force {
                        warnings.push(format!(
                            "Target trunk already exists at '{}'; keeping target path",
                            target_trunk.display()
                        ));
                    } else {
                        return Err(anyhow!(
                            "Target trunk already exists at '{}'. Re-run with --force to keep the target path.",
                            target_trunk.display()
                        ));
                    }
                }
                (true, false) => {
                    if Self::paths_match(&source_trunk, &target_container) {
                        self.move_container_contents_to_trunk(
                            &target_container,
                            &target_trunk,
                            &tracked_worktree_names,
                        )
                        .await?;
                    } else {
                        if let Some(parent) = target_trunk.parent() {
                            fs::create_dir_all(parent).await?;
                        }

                        std::fs::rename(&source_trunk, &target_trunk).with_context(|| {
                            format!(
                                "Failed to move trunk from '{}' to '{}'",
                                source_trunk.display(),
                                target_trunk.display()
                            )
                        })?;
                    }
                }
                (false, true) => {
                    warnings.push(format!(
                        "Source trunk missing at '{}'; using existing target '{}'",
                        source_trunk.display(),
                        target_trunk.display()
                    ));
                }
                (false, false) => {
                    return Err(anyhow!(
                        "Neither source trunk '{}' nor target trunk '{}' exists",
                        source_trunk.display(),
                        target_trunk.display()
                    ));
                }
            }

            let target_trunk_str = target_trunk.to_string_lossy().to_string();
            self.db
                .update_repository_path(&repo.name, &target_trunk_str)
                .await?;
        }

        for worktree in &worktrees {
            let source = PathBuf::from(&worktree.path);
            let target = target_container.join(&worktree.name);

            if Self::paths_match(&source, &target) {
                continue;
            }

            let source_exists = source.exists();
            let target_exists = target.exists();

            match (source_exists, target_exists) {
                (true, true) => {
                    if self.force {
                        warnings.push(format!(
                            "Worktree '{}' exists at both '{}' and '{}'; keeping target path",
                            worktree.name,
                            source.display(),
                            target.display()
                        ));
                    } else {
                        return Err(anyhow!(
                            "Worktree '{}' exists at both '{}' and '{}'. Re-run with --force to keep target path.",
                            worktree.name,
                            source.display(),
                            target.display()
                        ));
                    }
                }
                (true, false) => {
                    if let Some(parent) = target.parent() {
                        fs::create_dir_all(parent).await?;
                    }

                    std::fs::rename(&source, &target).with_context(|| {
                        format!(
                            "Failed to move worktree '{}' from '{}' to '{}'",
                            worktree.name,
                            source.display(),
                            target.display()
                        )
                    })?;
                    moved_worktrees += 1;
                }
                (false, true) => {
                    warnings.push(format!(
                        "Worktree '{}' source missing at '{}'; using existing target '{}'",
                        worktree.name,
                        source.display(),
                        target.display()
                    ));
                }
                (false, false) => {
                    let message = format!(
                        "Worktree '{}' not found at '{}' or '{}'",
                        worktree.name,
                        source.display(),
                        target.display()
                    );
                    if self.force {
                        warnings.push(message);
                        continue;
                    }
                    return Err(anyhow!(message));
                }
            }

            if target.exists() {
                let target_str = target.to_string_lossy().to_string();
                self.db
                    .update_worktree_path(&repo.name, &worktree.name, &target_str)
                    .await?;
                updated_worktrees += 1;
            }
        }

        self.write_project_metadata_file(repo, &target_trunk)
            .await?;

        let mut message = format!(
            "Office layout migration completed (moved {} worktrees, updated {} worktree path entries)",
            moved_worktrees, updated_worktrees
        );
        if !warnings.is_empty() {
            message.push_str(&format!(" with {} warning(s)", warnings.len()));
        }

        Ok(OfficeMigrationResult {
            repo_name: repo.name.clone(),
            source_trunk: source_trunk.display().to_string(),
            target_trunk: target_trunk.display().to_string(),
            moved_worktrees,
            updated_worktrees,
            status: "migrated".to_string(),
            message,
            warnings,
        })
    }

    async fn move_container_contents_to_trunk(
        &self,
        container: &Path,
        trunk_path: &Path,
        tracked_worktrees: &[String],
    ) -> Result<()> {
        if trunk_path.exists() {
            return Err(anyhow!(
                "Target trunk directory already exists: {}",
                trunk_path.display()
            ));
        }

        fs::create_dir_all(trunk_path)
            .await
            .context("Failed to create trunk directory")?;

        let mut reserved_names: HashSet<String> = tracked_worktrees.iter().cloned().collect();
        reserved_names.insert(".iMi".to_string());
        reserved_names.insert("sync".to_string());
        if let Some(name) = trunk_path.file_name().and_then(|n| n.to_str()) {
            reserved_names.insert(name.to_string());
        }

        let mut entries = fs::read_dir(container).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy().to_string();

            if path == trunk_path || reserved_names.contains(&file_name_str) {
                continue;
            }

            let looks_like_worktree = file_name_str.starts_with("feat-")
                || file_name_str.starts_with("fix-")
                || file_name_str.starts_with("aiops-")
                || file_name_str.starts_with("devops-")
                || file_name_str.starts_with("pr-")
                || file_name_str.starts_with("review-");

            if looks_like_worktree {
                continue;
            }

            let target = trunk_path.join(&file_name);
            std::fs::rename(&path, &target).with_context(|| {
                format!(
                    "Failed to move '{}' into trunk directory '{}'",
                    path.display(),
                    trunk_path.display()
                )
            })?;
        }

        Ok(())
    }

    async fn write_project_metadata_file(
        &self,
        project: &crate::database::Project,
        trunk_path: &Path,
    ) -> Result<()> {
        let container = trunk_path.parent().ok_or_else(|| {
            anyhow!(
                "Invalid trunk path (missing parent directory): {}",
                trunk_path.display()
            )
        })?;
        let imi_dir = container.join(".iMi");
        fs::create_dir_all(&imi_dir)
            .await
            .context("Failed to create .iMi directory in office container")?;

        let metadata = ProjectMetadata {
            project_id: project.id,
            name: project.name.clone(),
            remote_origin: project.remote_url.clone(),
            default_branch: project.default_branch.clone(),
            trunk_path: trunk_path.display().to_string(),
            description: project.description.clone(),
        };

        let json_content = serde_json::to_string_pretty(&metadata)
            .context("Failed to serialize project metadata")?;
        fs::write(imi_dir.join("project.json"), json_content)
            .await
            .context("Failed to write office project metadata")?;

        Ok(())
    }

    fn paths_match(left: &Path, right: &Path) -> bool {
        if left == right {
            return true;
        }

        match (std::fs::canonicalize(left), std::fs::canonicalize(right)) {
            (Ok(a), Ok(b)) => a == b,
            _ => false,
        }
    }

    async fn handle_outside_repo(&self) -> Result<InitResult> {
        let config_path = Config::get_global_config_path()?;
        let db_path = &self.config.database_path;

        let config_exists = config_path.exists();
        let db_exists = db_path.exists();

        // Only show the "Running outside" message if we're actually creating something new
        let needs_setup = !config_exists || !db_exists || self.force;

        if needs_setup {
            println!(
                "{} Running outside of a git repository. Setting up global iMi configuration...",
                "🌍".bright_blue()
            );
        }

        if !config_exists || self.force {
            self.config
                .save_to(&config_path)
                .await
                .context("Failed to save default configuration")?;
            println!(
                "{} Created default configuration at {}",
                "✅".bright_green(),
                config_path.display()
            );
        } else if !needs_setup {
            // Silently skip - config already exists and we're not forcing
        } else {
            println!(
                "{} Configuration already exists at {}. Use --force to overwrite.",
                "ℹ️".bright_yellow(),
                config_path.display()
            );
        }

        if !db_exists || self.force {
            self.db
                .ensure_tables()
                .await
                .context("Failed to create database tables")?;
            println!(
                "{} Created database at {}",
                "✅".bright_green(),
                db_path.display()
            );
        } else if !needs_setup {
            // Silently skip - database already exists and we're not forcing
        } else {
            println!(
                "{} Database already exists at {}. Use --force to overwrite.",
                "ℹ️".bright_yellow(),
                db_path.display()
            );
        }

        if needs_setup {
            Ok(InitResult::success(
                "Global iMi configuration setup complete.".to_string(),
            ))
        } else {
            // Silent success - everything already exists
            Ok(InitResult::success(
                "iMi is already configured.".to_string(),
            ))
        }
    }

    async fn handle_inside_repo(&self, current_dir: &Path) -> Result<InitResult> {
        println!(
            "{} Running inside a git repository. Initializing...",
            "🚀".bright_cyan()
        );

        let git_manager = GitManager::new();
        let repo = git_manager.find_repository(Some(current_dir))?;
        let repo_path = repo
            .workdir()
            .context("Repository has no working directory")?
            .to_path_buf();
        let repo_name = git_manager.get_repository_name(&repo)?;

        let trunk_dir = format!("trunk-{}", self.config.git_settings.default_branch);
        let repo_container = self.config.get_repo_path(&repo_name);
        let trunk_path = repo_container.join(&trunk_dir);

        if repo_path != trunk_path {
            println!();
            println!(
                "{} {}",
                "⚠️".bright_yellow(),
                "Current repository path does not match office layout:".bright_yellow()
            );
            println!("   {}", repo_path.display().to_string().bright_white());
            println!();
            println!(
                "{}",
                "Target agent office layout (Anthropic-style isolation):".bright_cyan()
            );
            println!(
                "   {}/",
                repo_container.display().to_string().bright_white()
            );
            println!(
                "     ├── {}/ {}",
                trunk_dir.bright_green(),
                "(trunk clone for this entity)".dimmed()
            );
            println!(
                "     ├── {}/ {}",
                "feat-feature1".bright_blue(),
                "(feature worktrees)".dimmed()
            );
            println!(
                "     └── {}/ {}",
                "fix-bugfix".bright_red(),
                "(fix worktrees)".dimmed()
            );
            println!();

            // Check if target structure already exists and is not the current source
            if repo_container.exists() && repo_container != repo_path {
                return Err(anyhow!(
                    "Target directory already exists: {}\nPlease manually resolve the conflict before migration.",
                    repo_container.display()
                ));
            }

            if !self.force {
                println!("{}", "This will:".bright_cyan());
                println!(
                    "  1. Create office directory: {}",
                    repo_container.display().to_string().bright_white()
                );
                println!(
                    "  2. Move current repo to: {}",
                    trunk_path.display().to_string().bright_green()
                );
                println!("  3. Register with iMi");
                println!();

                let should_restructure = Confirm::new()
                    .with_prompt("Would you like to migrate to office layout now?")
                    .default(false)
                    .interact()?;

                if !should_restructure {
                    return Ok(InitResult::failure(
                        "Initialization cancelled. Re-run 'iMi init' to migrate into office layout."
                            .to_string(),
                    ));
                }
            }

            println!();
            println!(
                "{} Migrating repository to office layout...",
                "🔄".bright_cyan()
            );
            let temp_backup = std::env::temp_dir().join(format!("imi_backup_{}", repo_name));

            match self
                .restructure_directory(&repo_path, &repo_container, &trunk_path, &temp_backup)
                .await
            {
                Ok(_) => {
                    println!("{} Office migration completed", "✅".bright_green());

                    if temp_backup.exists() {
                        let _ = fs::remove_dir_all(&temp_backup).await;
                    }

                    return self.register_repository(&trunk_path, &repo_name).await;
                }
                Err(e) => {
                    println!("{} Migration failed: {}", "❌".bright_red(), e);

                    if temp_backup.exists() {
                        println!("{} Attempting rollback...", "🔄".bright_yellow());
                        if let Err(rollback_err) =
                            self.rollback_restructure(&temp_backup, &repo_path).await
                        {
                            println!("{} Rollback failed: {}", "❌".bright_red(), rollback_err);
                            println!(
                                "{} Manual intervention required. Backup at: {}",
                                "⚠️".bright_yellow(),
                                temp_backup.display()
                            );
                        } else {
                            println!("{} Rollback successful", "✅".bright_green());
                            let _ = fs::remove_dir_all(&temp_backup).await;
                        }
                    }

                    return Err(e);
                }
            }
        }

        self.register_repository(&repo_path, &repo_name).await
    }

    async fn restructure_directory(
        &self,
        source: &Path,
        container: &Path,
        trunk_path: &Path,
        backup: &Path,
    ) -> Result<()> {
        // Step 1: Create backup
        fs::create_dir_all(backup.parent().unwrap()).await?;

        // Copy source to backup (using tokio::fs for async operations)
        self.copy_dir_recursive(source, backup).await?;

        // Step 2: Create container directory
        fs::create_dir_all(container)
            .await
            .context("Failed to create container directory")?;

        // Step 3: Move source to trunk_path inside container.
        // Handle in-place migration when source already equals container.
        if source == container {
            if trunk_path.exists() {
                return Err(anyhow!(
                    "Target trunk directory already exists: {}",
                    trunk_path.display()
                ));
            }

            fs::create_dir_all(trunk_path)
                .await
                .context("Failed to create trunk directory during in-place migration")?;

            let mut entries = fs::read_dir(source).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                let file_name = entry.file_name();
                let file_name_str = file_name.to_string_lossy();

                // Keep existing iMi metadata at container root and skip new trunk dir.
                if path == trunk_path || file_name_str == ".iMi" {
                    continue;
                }

                let target = trunk_path.join(&file_name);
                std::fs::rename(&path, &target)
                    .context("Failed to move repository content into trunk directory")?;
            }
        } else {
            std::fs::rename(source, trunk_path)
                .context("Failed to move repository to trunk directory")?;
        }

        Ok(())
    }

    fn copy_dir_recursive<'a>(
        &'a self,
        src: &'a Path,
        dst: &'a Path,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            fs::create_dir_all(dst).await?;

            let mut entries = fs::read_dir(src).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                let file_name = entry.file_name();
                let dst_path = dst.join(&file_name);

                if path.is_dir() {
                    self.copy_dir_recursive(&path, &dst_path).await?;
                } else {
                    fs::copy(&path, &dst_path).await?;
                }
            }

            Ok(())
        })
    }

    async fn rollback_restructure(&self, backup: &Path, original: &Path) -> Result<()> {
        // Remove any partially created structure
        if let Some(parent) = original.parent() {
            // Only remove if it was newly created (empty or only contains our failed attempt)
            if parent.exists() {
                let mut entries = fs::read_dir(parent).await?;
                let mut count = 0;
                while let Some(_) = entries.next_entry().await? {
                    count += 1;
                    if count > 1 {
                        break;
                    }
                }

                // If parent only has one entry (our failed attempt), safe to remove
                if count <= 1 {
                    fs::remove_dir_all(parent).await?;
                }
            }
        }

        // Restore from backup
        self.copy_dir_recursive(backup, original).await?;

        Ok(())
    }

    async fn register_repository(&self, repo_path: &Path, repo_name: &str) -> Result<InitResult> {
        let git_manager = GitManager::new();

        self.validate_office_layout(repo_path, repo_name)?;

        if let Some(existing_repo) = self.db.get_repository(repo_name).await? {
            if !self.force {
                return Ok(InitResult::failure(format!(
                    "Repository '{}' is already registered at {}. Use --force to re-initialize.",
                    repo_name, existing_repo.path
                )));
            }
        }

        let remote_url = git_manager
            .get_remote_url(repo_path)
            .await
            .unwrap_or_default();
        let default_branch = git_manager
            .get_default_branch(repo_path)
            .await
            .unwrap_or_else(|_| "main".to_string());

        let project = self
            .db
            .create_repository(
                repo_name,
                repo_path.to_str().unwrap(),
                &remote_url,
                &default_branch,
            )
            .await?;
        println!(
            "{} Registered repository '{}' in the database.",
            "✅".bright_green(),
            repo_name
        );
        println!(
            "   {} Project ID: {}",
            "🔑".bright_black(),
            project.id.to_string().bright_cyan()
        );

        let imi_dir = repo_path.parent().unwrap().join(".iMi");
        fs::create_dir_all(&imi_dir)
            .await
            .context("Failed to create .iMi directory")?;
        println!(
            "{} Created .iMi directory at {}",
            "✅".bright_green(),
            imi_dir.display()
        );

        // Write project metadata to .iMi/project.json for fast filesystem access
        let project_metadata = ProjectMetadata {
            project_id: project.id,
            name: project.name.clone(),
            remote_origin: project.remote_url.clone(),
            default_branch: project.default_branch.clone(),
            trunk_path: project.path.clone(),
            description: project.description.clone(),
        };
        let project_json_path = imi_dir.join("project.json");
        let json_content = serde_json::to_string_pretty(&project_metadata)
            .context("Failed to serialize project metadata")?;
        fs::write(&project_json_path, json_content)
            .await
            .context("Failed to write project.json")?;
        println!(
            "{} Created project.json with UUID {}",
            "✅".bright_green(),
            project.id.to_string().bright_cyan()
        );

        Ok(InitResult::success(format!(
            "Successfully initialized iMi for repository '{}'.",
            repo_name
        )))
    }

    fn validate_office_layout(&self, repo_path: &Path, repo_name: &str) -> Result<()> {
        let expected_container = self.config.get_repo_path(repo_name);
        let expected_trunk_name = format!("trunk-{}", self.config.git_settings.default_branch);
        let expected_trunk_path = expected_container.join(&expected_trunk_name);

        let actual_trunk_name = repo_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();

        let in_expected_container = repo_path
            .parent()
            .map(|p| p == expected_container.as_path())
            .unwrap_or(false);

        if !in_expected_container || actual_trunk_name != expected_trunk_name {
            return Err(anyhow!(
                "Repository path does not match office layout.\nExpected: {}\nActual: {}\nEach entity must operate from its own office clone before creating worktrees.",
                expected_trunk_path.display(),
                repo_path.display()
            ));
        }

        Ok(())
    }

    fn detect_paths(&self, current_dir: &Path) -> Result<(PathBuf, String)> {
        let git_manager = GitManager::new();
        let repo = git_manager.find_repository(Some(current_dir))?;
        let repo_path = repo
            .workdir()
            .context("Repository has no working directory")?
            .to_path_buf();

        let repo_name = git_manager.get_repository_name(&repo)?;

        // The "imi_path" is the parent of the repository directory.
        let imi_path = repo_path.parent().unwrap_or(&repo_path).to_path_buf();

        Ok((imi_path, repo_name))
    }

    /// Clone a repository from GitHub and set up iMi structure
    pub async fn clone_from_github(&self, github_repo: &str) -> Result<InitResult> {
        println!(
            "{} Cloning {} from GitHub...",
            "🔍".bright_cyan(),
            github_repo.bright_white()
        );

        // Extract repo name from owner/repo format
        let parts: Vec<&str> = github_repo.split('/').collect();
        if parts.len() != 2 {
            return Err(anyhow!(
                "Invalid GitHub repository format. Expected: owner/repo"
            ));
        }

        let repo_name = parts[1];

        // Determine clone location in the entity office
        let entity_workspace = self.config.get_entity_workspace_path();
        fs::create_dir_all(&entity_workspace)
            .await
            .context("Failed to create entity workspace directory")?;

        let trunk_dir = format!("trunk-{}", self.config.git_settings.default_branch);
        let repo_container = entity_workspace.join(repo_name);
        let trunk_path = repo_container.join(&trunk_dir);

        // Check if already exists
        if trunk_path.exists() {
            return Err(anyhow!(
                "Repository already exists at: {}\nUse 'iMi init {}' to initialize it.",
                trunk_path.display(),
                trunk_path.display()
            ));
        }

        // Create container directory
        fs::create_dir_all(&repo_container)
            .await
            .context("Failed to create repository container")?;

        // Clone the repository using git command
        let git_url = format!("https://github.com/{}.git", github_repo);

        println!(
            "{} Cloning into {}...",
            "📁".bright_blue(),
            trunk_path.display().to_string().bright_white()
        );

        let output = tokio::process::Command::new("git")
            .args(&["clone", &git_url, trunk_path.to_str().unwrap()])
            .output()
            .await
            .context("Failed to execute git clone")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!(
                "Git clone failed: {}\n\nThis might be a private repository. Try:\n  1. Check that {} exists on GitHub\n  2. Ensure you're authenticated (gh auth login or SSH keys)",
                stderr,
                github_repo
            ));
        }

        println!("{} Clone complete!", "✅".bright_green());

        // Now initialize iMi in the cloned repository
        self.register_repository(&trunk_path, repo_name).await
    }
}
