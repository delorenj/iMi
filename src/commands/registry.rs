use anyhow::{Context, Result};
use colored::*;
use sqlx::PgPool;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Sync filesystem with database - discover and register all iMi cluster hubs
pub async fn sync_filesystem(pool: &PgPool, scan_root: Option<&Path>) -> Result<SyncStats> {
    let default_scan = PathBuf::from(std::env::var("HOME").unwrap()).join("code");
    let scan_path = scan_root.unwrap_or(&default_scan);

    println!("{}", "━".repeat(60).bright_black());
    println!(
        "{}",
        "iMi Registry Sync - Filesystem Discovery"
            .bold()
            .bright_white()
    );
    println!("{}\n", "━".repeat(60).bright_black());

    let mut stats = SyncStats::default();

    // Discover all cluster hubs
    let hubs = discover_cluster_hubs(&scan_path, 4)?;

    println!("{} Found {} cluster hubs\n", "🔍".bright_cyan(), hubs.len());

    for hub in hubs {
        println!("{} Processing: {}", "📦".bright_black(), hub.name.bold());
        println!(
            "   {} Trunk: {}",
            "📂".bright_black(),
            hub.trunk_path.display()
        );
        println!("   {} Remote: {}", "🔗".bright_black(), hub.remote_url);

        // Register project (idempotent via ON CONFLICT)
        match register_project(pool, &hub).await {
            Ok(project_id) => {
                println!(
                    "   {} Registered: {}",
                    "✅".green(),
                    project_id.to_string().bright_black()
                );
                stats.projects_registered += 1;

                // Discover and register worktrees
                if let Ok(worktrees) = discover_worktrees(&hub.project_path, &project_id).await {
                    for wt in worktrees {
                        match register_worktree(pool, &wt).await {
                            Ok(_) => {
                                println!("      {} Worktree: {}", "✓".green(), wt.name);
                                stats.worktrees_registered += 1;
                            }
                            Err(e) => {
                                println!(
                                    "      {} Worktree '{}' skipped: {}",
                                    "⚠".yellow(),
                                    wt.name,
                                    e
                                );
                            }
                        }
                    }
                }
            }
            Err(e) => {
                println!("   {} Skipped: {}", "⚠".yellow(), e);
                stats.projects_skipped += 1;
            }
        }

        println!();
    }

    println!("{}", "━".repeat(60).bright_black());
    println!("{}", "Summary".bold());
    println!("{}", "━".repeat(60).bright_black());
    println!(
        "Projects registered: {}",
        stats.projects_registered.to_string().green()
    );
    println!(
        "Projects skipped: {}",
        stats.projects_skipped.to_string().yellow()
    );
    println!(
        "Worktrees registered: {}",
        stats.worktrees_registered.to_string().green()
    );
    println!();

    Ok(stats)
}

#[derive(Default, Debug)]
pub struct SyncStats {
    pub projects_registered: usize,
    pub projects_skipped: usize,
    pub worktrees_registered: usize,
}

#[derive(Debug)]
struct ClusterHub {
    name: String,
    project_path: PathBuf,
    trunk_path: PathBuf,
    remote_url: String,
    default_branch: String,
}

#[derive(Debug)]
struct WorktreeInfo {
    project_id: Uuid,
    name: String,
    branch_name: String,
    worktree_type: String,
    path: PathBuf,
}

/// Discover all iMi cluster hubs in the filesystem
fn discover_cluster_hubs(scan_root: &Path, max_depth: usize) -> Result<Vec<ClusterHub>> {
    use walkdir::WalkDir;

    let mut hubs = Vec::new();

    for entry in WalkDir::new(scan_root)
        .max_depth(max_depth)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // Look for .iMi directories
        if path.file_name() == Some(std::ffi::OsStr::new(".iMi")) && path.is_dir() {
            let project_path = path.parent().unwrap().to_path_buf();

            // Check if this has a trunk-* subdirectory (cluster hub indicator)
            let trunk_dirs: Vec<_> = std::fs::read_dir(&project_path)
                .ok()
                .into_iter()
                .flatten()
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.file_name().to_string_lossy().starts_with("trunk-") && e.path().is_dir()
                })
                .collect();

            if let Some(trunk_entry) = trunk_dirs.first() {
                let trunk_path = trunk_entry.path();

                // Get git remote from trunk
                if let Ok(remote_url) = get_git_remote(&trunk_path) {
                    let name = project_path
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .to_string();

                    let default_branch = trunk_path
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .strip_prefix("trunk-")
                        .unwrap_or("main")
                        .to_string();

                    // Normalize remote URL
                    let normalized_url = normalize_remote_url(&remote_url);

                    hubs.push(ClusterHub {
                        name,
                        project_path,
                        trunk_path,
                        remote_url: normalized_url,
                        default_branch,
                    });
                }
            }
        }
    }

    Ok(hubs)
}

/// Get git remote URL from a directory
fn get_git_remote(path: &Path) -> Result<String> {
    let output = std::process::Command::new("git")
        .arg("remote")
        .arg("get-url")
        .arg("origin")
        .current_dir(path)
        .output()
        .context("Failed to run git remote")?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(anyhow::anyhow!("No git remote configured"))
    }
}

/// Normalize remote URL to SSH format with .git suffix
fn normalize_remote_url(url: &str) -> String {
    let mut normalized = url.to_string();

    // Convert HTTPS to SSH
    if let Some(caps) = regex::Regex::new(r"^https://github\.com/(.+)/(.+?)(?:\.git)?$")
        .unwrap()
        .captures(url)
    {
        normalized = format!("git@github.com:{}/{}.git", &caps[1], &caps[2]);
    }

    // Ensure .git suffix
    if !normalized.ends_with(".git") {
        normalized.push_str(".git");
    }

    normalized
}

/// Register a project in PostgreSQL (idempotent)
async fn register_project(pool: &PgPool, hub: &ClusterHub) -> Result<Uuid> {
    let project_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT register_project($1, $2, $3, $4, '{}'::jsonb)
        "#,
    )
    .bind(&hub.name)
    .bind(&hub.remote_url)
    .bind(&hub.default_branch)
    .bind(hub.trunk_path.to_string_lossy().as_ref())
    .fetch_one(pool)
    .await
    .context("Failed to register project")?;

    Ok(project_id)
}

/// Discover worktrees in a cluster hub
async fn discover_worktrees(project_path: &Path, project_id: &Uuid) -> Result<Vec<WorktreeInfo>> {
    let mut worktrees = Vec::new();

    for entry in std::fs::read_dir(project_path)? {
        let entry = entry?;
        let path = entry.path();

        // Skip hidden, trunk, and non-directories
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || name.starts_with("trunk-") || !path.is_dir() {
            continue;
        }

        // Must have .git (file or directory)
        let git_path = path.join(".git");
        if !git_path.exists() {
            continue;
        }

        // Get branch name
        let output = std::process::Command::new("git")
            .arg("rev-parse")
            .arg("--abbrev-ref")
            .arg("HEAD")
            .current_dir(&path)
            .output()?;

        if !output.status.success() {
            continue;
        }

        let branch_name = String::from_utf8_lossy(&output.stdout).trim().to_string();

        // Determine worktree type from name
        let worktree_type = if name.starts_with("feat-") {
            "feat"
        } else if name.starts_with("fix-") {
            "fix"
        } else if name.starts_with("aiops-") {
            "aiops"
        } else if name.starts_with("devops-") {
            "devops"
        } else if name.starts_with("review-") {
            "review"
        } else {
            continue; // Skip unknown types
        };

        worktrees.push(WorktreeInfo {
            project_id: *project_id,
            name,
            branch_name,
            worktree_type: worktree_type.to_string(),
            path,
        });
    }

    Ok(worktrees)
}

/// Register a worktree in PostgreSQL (idempotent)
async fn register_worktree(pool: &PgPool, wt: &WorktreeInfo) -> Result<Uuid> {
    let worktree_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT register_worktree($1, $2, $3, $4, $5, NULL, '{}'::jsonb)
        "#,
    )
    .bind(&wt.project_id)
    .bind(&wt.worktree_type)
    .bind(&wt.name)
    .bind(&wt.branch_name)
    .bind(wt.path.to_string_lossy().as_ref())
    .fetch_one(pool)
    .await
    .context("Failed to register worktree")?;

    Ok(worktree_id)
}
