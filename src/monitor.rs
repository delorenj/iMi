use anyhow::Result;
use colored::*;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::{signal, time};

use crate::database::Worktree;
use crate::worktree::WorktreeManager;

#[derive(Debug, Clone)]
pub struct MonitorManager {
    worktree_manager: WorktreeManager,
}

#[derive(Debug, Clone)]
pub struct ActivityEvent {
    pub worktree_id: String,
    pub event_type: String,
    pub file_path: Option<String>,
    pub timestamp: Instant,
}

impl MonitorManager {
    pub fn new(worktree_manager: WorktreeManager) -> Self {
        Self { worktree_manager }
    }

    /// Start real-time monitoring of worktree activities
    pub async fn start(&self, repo: Option<&str>) -> Result<()> {
        println!(
            "{} Starting iMi Real-time Monitor",
            "👁️".bright_purple().bold()
        );
        println!("{}", "─".repeat(60).bright_black());

        // Get active worktrees to monitor
        let worktrees = self.worktree_manager.db.list_worktrees(repo).await?;

        if worktrees.is_empty() {
            println!("{} No active worktrees to monitor", "ℹ️".bright_blue());
            return Ok(());
        }

        println!(
            "{} Monitoring {} worktrees",
            "📊".bright_cyan(),
            worktrees.len()
        );
        for wt in &worktrees {
            println!(
                "  {} {}/{}",
                self.get_type_icon(&wt.worktree_type),
                wt.repo_name.bright_blue(),
                wt.worktree_name.bright_green()
            );
        }
        println!();

        // Set up file watchers
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let mut _watchers = Vec::new();
        let mut path_to_worktree = HashMap::new();

        for worktree in &worktrees {
            let path = PathBuf::from(&worktree.path);
            if path.exists() {
                let tx_clone = tx.clone();
                let mut watcher = RecommendedWatcher::new(
                    move |res: Result<Event, _>| {
                        if let Ok(event) = res {
                            let _ = tx_clone.try_send(event);
                        }
                    },
                    Config::default(),
                )?;
                watcher.watch(&path, RecursiveMode::Recursive)?;
                _watchers.push(watcher);
                path_to_worktree.insert(path, worktree.clone());
            }
        }

        // Start monitoring loop
        let monitor_task = self.monitor_loop(rx, path_to_worktree);
        let status_task = self.periodic_status_update(repo, worktrees.clone());

        // Wait for Ctrl+C
        println!("{} Press Ctrl+C to stop monitoring", "💡".bright_yellow());

        tokio::select! {
            _ = monitor_task => {},
            _ = status_task => {},
            _ = signal::ctrl_c() => {
                println!("\n{} Monitoring stopped", "🛑".bright_red());
            }
        }

        Ok(())
    }

    /// Main monitoring loop for file system events
    async fn monitor_loop(
        &self,
        mut rx: tokio::sync::mpsc::Receiver<Event>,
        path_to_worktree: HashMap<PathBuf, Worktree>,
    ) -> Result<()> {
        let mut last_events: HashMap<String, Instant> = HashMap::new();
        let debounce_duration = Duration::from_secs(1);

        while let Some(event) = rx.recv().await {
            if let Some(activity) = self.process_file_event(&event, &path_to_worktree).await {
                // Debounce rapid events
                let key = format!(
                    "{}:{}",
                    activity.worktree_id,
                    activity.file_path.as_deref().unwrap_or("")
                );

                if let Some(last_time) = last_events.get(&key) {
                    if activity.timestamp.duration_since(*last_time) < debounce_duration {
                        continue;
                    }
                }

                last_events.insert(key, activity.timestamp);
                self.display_activity(&activity).await;

                // Log to database
                if let Err(e) = self.log_activity_to_db(&activity).await {
                    eprintln!("Failed to log activity: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Periodic status updates
    async fn periodic_status_update(
        &self,
        _repo: Option<&str>,
        worktrees: Vec<Worktree>,
    ) -> Result<()> {
        let mut interval = time::interval(Duration::from_secs(30));
        let mut last_status_check = Instant::now();

        loop {
            interval.tick().await;

            // Every 30 seconds, show a summary
            if last_status_check.elapsed() >= Duration::from_secs(30) {
                let _ = self.display_status_summary(&worktrees).await;
                last_status_check = Instant::now();
            }
        }
    }

    /// Process a file system event into an activity event
    async fn process_file_event(
        &self,
        event: &Event,
        path_to_worktree: &HashMap<PathBuf, Worktree>,
    ) -> Option<ActivityEvent> {
        let (event_type, file_path) = match &event.kind {
            notify::EventKind::Create(_) => ("created", event.paths.first().cloned()),
            notify::EventKind::Modify(_) => ("modified", event.paths.first().cloned()),
            notify::EventKind::Remove(_) => ("deleted", event.paths.first().cloned()),
            _ => return None,
        };

        // Find which worktree this file belongs to
        let file_path = file_path?;
        for (worktree_path, worktree) in path_to_worktree {
            if file_path.starts_with(worktree_path) {
                // Skip .git and other system files
                if let Some(file_name) = file_path.file_name() {
                    let file_str = file_name.to_string_lossy();
                    if file_str.starts_with('.') && !file_str.starts_with(".env") {
                        continue;
                    }
                }

                let relative_path = file_path.strip_prefix(worktree_path).ok()?;

                return Some(ActivityEvent {
                    worktree_id: worktree.id.clone(),
                    event_type: event_type.to_string(),
                    file_path: Some(relative_path.to_string_lossy().to_string()),
                    timestamp: Instant::now(),
                });
            }
        }

        None
    }

    /// Display activity event
    async fn display_activity(&self, activity: &ActivityEvent) {
        let timestamp = chrono::Utc::now().format("%H:%M:%S");
        let icon = match activity.event_type.as_str() {
            "created" => "➕".bright_green(),
            "modified" => "📝".bright_yellow(),
            "deleted" => "➖".bright_red(),
            "renamed" => "🔄".bright_blue(),
            _ => "📄".bright_white(),
        };

        if let Some(file_path) = &activity.file_path {
            println!(
                "{} {} {} {}",
                timestamp.to_string().bright_black(),
                icon,
                activity.event_type.bright_cyan(),
                file_path.bright_white()
            );
        }
    }

    /// Log activity to database
    async fn log_activity_to_db(&self, activity: &ActivityEvent) -> Result<()> {
        let description = if let Some(file_path) = &activity.file_path {
            format!("File {}: {}", activity.event_type, file_path)
        } else {
            format!("Worktree {}", activity.event_type)
        };

        self.worktree_manager
            .db
            .log_agent_activity(
                "file-monitor", // agent_id
                &activity.worktree_id,
                &activity.event_type,
                activity.file_path.as_deref(),
                &description,
            )
            .await?;

        Ok(())
    }

    /// Display periodic status summary
    async fn display_status_summary(&self, worktrees: &[Worktree]) -> Result<()> {
        let timestamp = chrono::Utc::now().format("%H:%M:%S");

        println!(
            "\n{} {} Status Summary",
            timestamp.to_string().bright_black(),
            "📊".bright_cyan()
        );
        println!("{}", "─".repeat(50).bright_black());

        let mut active_count = 0;
        let mut type_counts = HashMap::new();

        for worktree in worktrees {
            let path = PathBuf::from(&worktree.path);
            if path.exists() {
                active_count += 1;
                *type_counts
                    .entry(worktree.worktree_type.clone())
                    .or_insert(0) += 1;

                // Check for recent Git activity
                if let Ok(status) = self.worktree_manager.git.get_worktree_status(&path) {
                    if !status.clean {
                        println!(
                            "  {} {}/{} - {} changes",
                            self.get_type_icon(&worktree.worktree_type),
                            worktree.repo_name.bright_blue(),
                            worktree.worktree_name.bright_green(),
                            (status.modified_files.len()
                                + status.new_files.len()
                                + status.deleted_files.len())
                            .to_string()
                            .bright_yellow()
                        );
                    }
                }
            }
        }

        println!(
            "  {} {} active worktrees",
            "📈".bright_green(),
            active_count
        );
        for (wt_type, count) in type_counts {
            println!(
                "    {} {}: {}",
                self.get_type_icon(&wt_type),
                wt_type,
                count
            );
        }

        // Show recent agent activities
        if let Ok(activities) = self
            .worktree_manager
            .db
            .get_recent_activities(None, 5)
            .await
        {
            if !activities.is_empty() {
                println!("  {} Recent activities:", "🕒".bright_cyan());
                for activity in activities.iter().take(3) {
                    let time_ago = chrono::Utc::now().signed_duration_since(activity.created_at);
                    println!(
                        "    {} {} ({})",
                        "⚡".bright_yellow(),
                        activity.description.bright_white(),
                        format!("{}m ago", time_ago.num_minutes()).bright_black()
                    );
                }
            }
        }

        println!();
        Ok(())
    }

    /// Get icon for worktree type
    fn get_type_icon(&self, worktree_type: &str) -> colored::ColoredString {
        match worktree_type {
            "feat" => "🚀".bright_cyan(),
            "pr" => "🔍".bright_yellow(),
            "fix" => "🔧".bright_red(),
            "aiops" => "🤖".bright_magenta(),
            "devops" => "⚙️".bright_blue(),
            "trunk" => "🌳".bright_green(),
            _ => "📁".bright_white(),
        }
    }

    /// Show real-time Git statistics
    #[allow(dead_code)]
    pub async fn show_git_stats(&self, repo: Option<&str>) -> Result<()> {
        let worktrees = self.worktree_manager.db.list_worktrees(repo).await?;

        println!("{} Git Activity Summary", "📊".bright_cyan().bold());
        println!("{}", "─".repeat(60).bright_black());

        let mut total_changes = 0;
        let mut total_commits_ahead = 0;
        let mut total_commits_behind = 0;

        for worktree in &worktrees {
            let path = PathBuf::from(&worktree.path);
            if path.exists() {
                if let Ok(status) = self.worktree_manager.git.get_worktree_status(&path) {
                    let changes = status.modified_files.len()
                        + status.new_files.len()
                        + status.deleted_files.len();
                    total_changes += changes;
                    total_commits_ahead += status.commits_ahead;
                    total_commits_behind += status.commits_behind;

                    if changes > 0 || status.commits_ahead > 0 || status.commits_behind > 0 {
                        println!(
                            "{} {}/{}",
                            self.get_type_icon(&worktree.worktree_type),
                            worktree.repo_name.bright_blue(),
                            worktree.worktree_name.bright_green()
                        );

                        if changes > 0 {
                            println!("  {} {} local changes", "📝".bright_yellow(), changes);
                        }
                        if status.commits_ahead > 0 {
                            println!(
                                "  {} {} commits ahead",
                                "⬆️".bright_green(),
                                status.commits_ahead
                            );
                        }
                        if status.commits_behind > 0 {
                            println!(
                                "  {} {} commits behind",
                                "⬇️".bright_red(),
                                status.commits_behind
                            );
                        }
                    }
                }
            }
        }

        println!("\n{} Totals:", "🎯".bright_cyan());
        println!("  {} {} total changes", "📝".bright_yellow(), total_changes);
        println!(
            "  {} {} commits ahead",
            "⬆️".bright_green(),
            total_commits_ahead
        );
        println!(
            "  {} {} commits behind",
            "⬇️".bright_red(),
            total_commits_behind
        );

        Ok(())
    }
}
