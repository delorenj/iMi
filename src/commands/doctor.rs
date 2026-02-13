use anyhow::{Context, Result};
use colored::*;
use sqlx::PgPool;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct HealthCheck {
    pub category: String,
    pub status: CheckStatus,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub info: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
}

impl HealthCheck {
    fn new(category: impl Into<String>) -> Self {
        Self {
            category: category.into(),
            status: CheckStatus::Pass,
            warnings: Vec::new(),
            errors: Vec::new(),
            info: Vec::new(),
        }
    }

    fn warn(&mut self, msg: impl Into<String>) {
        self.warnings.push(msg.into());
        if self.status == CheckStatus::Pass {
            self.status = CheckStatus::Warn;
        }
    }

    fn error(&mut self, msg: impl Into<String>) {
        self.errors.push(msg.into());
        self.status = CheckStatus::Fail;
    }

    fn info(&mut self, msg: impl Into<String>) {
        self.info.push(msg.into());
    }
}

pub struct DoctorOpts {
    pub network: bool,
    pub verbose: bool,
}

impl Default for DoctorOpts {
    fn default() -> Self {
        Self {
            network: false,
            verbose: false,
        }
    }
}

/// Main entry point for health checks
pub async fn run_doctor(pool: &PgPool, opts: DoctorOpts) -> Result<Vec<HealthCheck>> {
    let mut checks = vec![];

    checks.push(check_database(pool).await?);
    checks.push(check_filesystem(pool).await?);
    checks.push(check_data_integrity(pool).await?);

    if opts.network {
        checks.push(check_git_remotes(pool).await?);
    }

    Ok(checks)
}

/// Check database connectivity and schema health
async fn check_database(pool: &PgPool) -> Result<HealthCheck> {
    let mut check = HealthCheck::new("Database Connectivity");

    // Test basic connectivity
    match sqlx::query("SELECT 1").fetch_one(pool).await {
        Ok(_) => {
            check.info(format!("Connected to PostgreSQL"));
        }
        Err(e) => {
            check.error(format!("Cannot connect to database: {}", e));
            return Ok(check);
        }
    }

    // Get registry stats
    let (total_projects, total_worktrees, active_worktrees) =
        sqlx::query_as::<_, (Option<i64>, Option<i64>, Option<i64>)>(
            r#"
            SELECT total_projects, total_worktrees, active_worktrees
            FROM get_registry_stats()
            "#,
        )
        .fetch_one(pool)
        .await
        .context("Failed to query registry stats")?;

    check.info(format!(
        "{} projects, {} worktrees ({} active)",
        total_projects.unwrap_or(0),
        total_worktrees.unwrap_or(0),
        active_worktrees.unwrap_or(0)
    ));

    // Check for orphaned worktrees (worktrees referencing deleted projects)
    let orphaned_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM worktrees w
        LEFT JOIN projects p ON w.project_id = p.id
        WHERE p.id IS NULL
        "#,
    )
    .fetch_one(pool)
    .await?;

    if orphaned_count > 0 {
        check.warn(format!("{} orphaned worktrees found", orphaned_count));
    }

    Ok(check)
}

/// Check filesystem state vs database state
async fn check_filesystem(pool: &PgPool) -> Result<HealthCheck> {
    let mut check = HealthCheck::new("Filesystem State");

    // Get all projects from database
    let projects = sqlx::query_as::<_, (String, String)>(
        r#"
        SELECT name, trunk_path
        FROM projects
        WHERE active = true
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut missing_trunks = 0;
    let mut invalid_paths = 0;

    for (project_name, project_trunk_path) in projects {
        let trunk_path = PathBuf::from(&project_trunk_path);

        // Check if trunk directory exists
        if !trunk_path.exists() {
            missing_trunks += 1;
            check.warn(format!(
                "Project '{}' trunk not found: {}",
                project_name, project_trunk_path
            ));
            continue;
        }

        // Check if it's actually a directory
        if !trunk_path.is_dir() {
            invalid_paths += 1;
            check.error(format!(
                "Project '{}' trunk is not a directory: {}",
                project_name, project_trunk_path
            ));
            continue;
        }

        // Check if .git exists (basic git repo validation)
        let git_dir = trunk_path.join(".git");
        if !git_dir.exists() {
            check.warn(format!(
                "Project '{}' trunk missing .git: {}",
                project_name, project_trunk_path
            ));
        }
    }

    if missing_trunks == 0 && invalid_paths == 0 {
        check.info("All registered projects have valid trunk directories");
    }

    Ok(check)
}

/// Check data integrity constraints
async fn check_data_integrity(pool: &PgPool) -> Result<HealthCheck> {
    let mut check = HealthCheck::new("Data Integrity");

    // Check for duplicate remote_origin (should be prevented by unique constraint)
    let duplicates = sqlx::query_as::<_, (String, i64)>(
        r#"
        SELECT remote_origin, COUNT(*) as count
        FROM projects
        WHERE active = true
        GROUP BY remote_origin
        HAVING COUNT(*) > 1
        "#,
    )
    .fetch_all(pool)
    .await?;

    if !duplicates.is_empty() {
        for (remote_origin, count) in duplicates {
            check.error(format!(
                "Duplicate remote_origin: {} ({} occurrences)",
                remote_origin, count
            ));
        }
    } else {
        check.info("No duplicate remote origins");
    }

    // Check for worktrees with invalid project references
    let invalid_worktrees = sqlx::query_as::<_, (String, String)>(
        r#"
        SELECT w.id::text as id, w.name
        FROM worktrees w
        LEFT JOIN projects p ON w.project_id = p.id
        WHERE p.id IS NULL AND w.active = true
        "#,
    )
    .fetch_all(pool)
    .await?;

    if !invalid_worktrees.is_empty() {
        for (worktree_id, worktree_name) in invalid_worktrees {
            check.error(format!(
                "Worktree '{}' references deleted project (id: {})",
                worktree_name, worktree_id
            ));
        }
    }

    // Check for invalid trunk_path values (should be absolute)
    let relative_paths = sqlx::query_as::<_, (String, String)>(
        r#"
        SELECT name, trunk_path
        FROM projects
        WHERE active = true AND trunk_path NOT LIKE '/%'
        "#,
    )
    .fetch_all(pool)
    .await?;

    if !relative_paths.is_empty() {
        for (project_name, trunk_path) in relative_paths {
            check.warn(format!(
                "Project '{}' has relative trunk_path: {}",
                project_name, trunk_path
            ));
        }
    }

    Ok(check)
}

/// Check git remote accessibility (requires network)
async fn check_git_remotes(pool: &PgPool) -> Result<HealthCheck> {
    let mut check = HealthCheck::new("Git Remote Access");

    let projects = sqlx::query_as::<_, (String, String, String)>(
        r#"
        SELECT name, trunk_path, remote_origin
        FROM projects
        WHERE active = true
        "#,
    )
    .fetch_all(pool)
    .await?;

    for (project_name, project_trunk_path, project_remote_origin) in projects {
        let trunk_path = PathBuf::from(&project_trunk_path);
        if !trunk_path.exists() {
            continue;
        }

        // Attempt to run git remote -v in the trunk directory
        let output = tokio::process::Command::new("git")
            .arg("remote")
            .arg("-v")
            .current_dir(&trunk_path)
            .output()
            .await;

        match output {
            Ok(out) if out.status.success() => {
                // Git remote command succeeded
                let stdout = String::from_utf8_lossy(&out.stdout);
                if !stdout.contains(&project_remote_origin) {
                    check.warn(format!(
                        "Project '{}' remote mismatch - DB: {}, Git: {}",
                        project_name,
                        project_remote_origin,
                        stdout.trim()
                    ));
                }
            }
            Ok(out) => {
                check.error(format!(
                    "Project '{}' git remote failed: {}",
                    project_name,
                    String::from_utf8_lossy(&out.stderr)
                ));
            }
            Err(e) => {
                check.error(format!(
                    "Project '{}' git command error: {}",
                    project_name, e
                ));
            }
        }
    }

    Ok(check)
}

/// Print health check results
pub fn print_report(checks: &[HealthCheck]) {
    println!("\n{}", "━".repeat(60).bright_black());
    println!("{}", "iMi System Health Check".bold().bright_white());
    println!("{}\n", "━".repeat(60).bright_black());

    let mut total_warnings = 0;
    let mut total_errors = 0;

    for check in checks {
        let status_icon = match check.status {
            CheckStatus::Pass => "✅".green(),
            CheckStatus::Warn => "⚠️ ".yellow(),
            CheckStatus::Fail => "❌".red(),
        };

        println!("{} {}", status_icon, check.category.bold());

        for info in &check.info {
            println!("   • {}", info.bright_black());
        }

        for warn in &check.warnings {
            println!("   {} {}", "⚠".yellow(), warn.yellow());
            total_warnings += 1;
        }

        for err in &check.errors {
            println!("   {} {}", "✗".red(), err.red());
            total_errors += 1;
        }

        println!();
    }

    println!("{}", "━".repeat(60).bright_black());

    let health_score = calculate_health_score(checks);
    let score_color = if health_score >= 90 {
        health_score.to_string().green()
    } else if health_score >= 70 {
        health_score.to_string().yellow()
    } else {
        health_score.to_string().red()
    };

    println!("Health Score: {}/100", score_color.bold());

    if total_warnings > 0 || total_errors > 0 {
        println!("\n{}", "Recommendations:".bold());
        if total_errors > 0 {
            println!("  1. Review and fix {} critical errors", total_errors);
        }
        if total_warnings > 0 {
            println!(
                "  2. Address {} warnings for optimal health",
                total_warnings
            );
        }
        println!("  3. Run `imi registry sync` to register new projects");
        println!("  4. Run `imi worktree prune` to clean up stale entries");
    } else {
        println!("{}", "All systems healthy! 🎉".green().bold());
    }

    println!();
}

fn calculate_health_score(checks: &[HealthCheck]) -> u32 {
    if checks.is_empty() {
        return 0;
    }

    let total_items: usize = checks
        .iter()
        .map(|c| 1 + c.warnings.len() + c.errors.len())
        .sum();

    let failures: usize = checks
        .iter()
        .map(|c| c.warnings.len() + (c.errors.len() * 2))
        .sum();

    let score = ((total_items.saturating_sub(failures)) as f64 / total_items as f64) * 100.0;
    score.round() as u32
}
