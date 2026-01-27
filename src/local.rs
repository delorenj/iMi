use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

/// Manages the "Data Plane" (.iMi directory) for a specific project.
/// Optimized for speed and shell consumption (Starship).
pub struct LocalContext {
    /// Path to .iMi/
    imi_dir: PathBuf,
    /// Path to .iMi/presence/ (Lock files)
    presence_dir: PathBuf,
    /// Path to .iMi/links/ (Source of truth for symlinks)
    links_dir: PathBuf,
    /// Path to .iMi/registry.toml (Fast metadata cache)
    registry_file: PathBuf,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct LocalRegistry {
    /// Maps worktree names (e.g., "feat-auth") to metadata
    worktrees: HashMap<String, WorktreeMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
struct WorktreeMetadata {
    #[serde(rename = "type")]
    worktree_type: String,
    created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    agent_owner: Option<String>,
}

impl LocalContext {
    /// Initialize a new LocalContext for the given project root
    pub fn new(project_root: &Path) -> Self {
        let imi_dir = project_root.join(".iMi");
        Self {
            presence_dir: imi_dir.join("presence"),
            links_dir: imi_dir.join("links"),
            registry_file: imi_dir.join("registry.toml"),
            imi_dir,
        }
    }

    /// Ensure the .iMi directory structure exists (Idempotent)
    pub fn init(&self) -> Result<()> {
        if !self.imi_dir.exists() {
            fs::create_dir_all(&self.imi_dir).context("Failed to create .iMi directory")?;
        }

        fs::create_dir_all(&self.presence_dir)
            .context("Failed to create .iMi/presence directory")?;

        fs::create_dir_all(&self.links_dir).context("Failed to create .iMi/links directory")?;

        // Initialize empty registry if missing
        if !self.registry_file.exists() {
            let registry = LocalRegistry::default();
            let toml = toml::to_string_pretty(&registry)?;
            fs::write(&self.registry_file, toml)?;
        }

        Ok(())
    }

    /// Try to acquire a lock on the registry file
    fn lock_registry(&self) -> Result<()> {
        let lock_path = self.imi_dir.join("registry.lock");
        let mut retries = 0;

        while retries < 10 {
            // Try to create the lock file exclusively
            // This is atomic on most filesystems
            if fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&lock_path)
                .is_ok()
            {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(50));
            retries += 1;
        }

        anyhow::bail!("Failed to acquire registry lock after 500ms")
    }

    /// Release the registry lock
    fn unlock_registry(&self) {
        let lock_path = self.imi_dir.join("registry.lock");
        let _ = fs::remove_file(lock_path);
    }

    /// Lock a worktree to signal active agent work
    /// Used by agents or long-running tasks to turn the prompt "Purple"
    pub fn lock_worktree(&self, worktree_name: &str, agent_id: &str) -> Result<()> {
        self.init()?;
        let lock_file = self.presence_dir.join(format!("{}.lock", worktree_name));
        fs::write(&lock_file, agent_id)
            .with_context(|| format!("Failed to create lock file for {}", worktree_name))?;
        Ok(())
    }

    /// Remove a lock file
    pub fn unlock_worktree(&self, worktree_name: &str) -> Result<()> {
        let lock_file = self.presence_dir.join(format!("{}.lock", worktree_name));
        if lock_file.exists() {
            fs::remove_file(&lock_file)
                .with_context(|| format!("Failed to remove lock file for {}", worktree_name))?;
        }
        Ok(())
    }

    /// Check if a worktree is currently locked
    pub fn is_locked(&self, worktree_name: &str) -> bool {
        self.presence_dir
            .join(format!("{}.lock", worktree_name))
            .exists()
    }

    /// Register a new worktree in the local cache (Dual-Write)
    /// This allows Starship to look up types without guessing from folder names
    pub fn register_worktree(
        &self,
        name: &str,
        worktree_type: &str,
        agent: Option<&str>,
    ) -> Result<()> {
        self.init()?;
        self.lock_registry()?;

        // Use a closure to ensure we always unlock even if errors occur
        let result = (|| -> Result<()> {
            // Read-Modify-Write the TOML registry
            let mut registry = if self.registry_file.exists() {
                let content = fs::read_to_string(&self.registry_file)
                    .context("Failed to read registry file")?;
                toml::from_str(&content).context("Failed to parse registry file")?
            } else {
                LocalRegistry::default()
            };

            registry.worktrees.insert(
                name.to_string(),
                WorktreeMetadata {
                    worktree_type: worktree_type.to_string(),
                    created_at: chrono::Utc::now().to_rfc3339(),
                    agent_owner: agent.map(|s| s.to_string()),
                },
            );

            let new_content = toml::to_string_pretty(&registry)?;
            fs::write(&self.registry_file, new_content)?;
            Ok(())
        })();

        self.unlock_registry();
        result
    }

    /// Remove a worktree from the local cache
    pub fn unregister_worktree(&self, name: &str) -> Result<()> {
        if !self.registry_file.exists() {
            return Ok(());
        }

        self.init()?;
        self.lock_registry()?;

        let result = (|| -> Result<()> {
            let content =
                fs::read_to_string(&self.registry_file).context("Failed to read registry file")?;
            let mut registry: LocalRegistry =
                toml::from_str(&content).context("Failed to parse registry file")?;

            if registry.worktrees.remove(name).is_some() {
                let new_content = toml::to_string_pretty(&registry)?;
                fs::write(&self.registry_file, new_content)?;
            }
            Ok(())
        })();

        self.unlock_registry();

        // Also ensure lock is cleaned up (doesn't need registry lock)
        self.unlock_worktree(name)?;

        result
    }

    /// Get the path to the 'links' directory for storing shared env files
    pub fn links_path(&self) -> &Path {
        &self.links_dir
    }

    /// Create a lock file with full metadata (for agent claim operations)
    /// Format: JSON with agent_id, claimed_at, hostname, worktree_id
    pub async fn create_lock_file(
        &self,
        imi_dir: &Path,
        worktree_name: &str,
        agent_id: &str,
    ) -> Result<()> {
        let presence_dir = imi_dir.join("presence");
        fs::create_dir_all(&presence_dir).context("Failed to create presence directory")?;

        let lock_file = presence_dir.join(format!("{}.lock", worktree_name));

        let lock_data = serde_json::json!({
            "agent_id": agent_id,
            "claimed_at": chrono::Utc::now().to_rfc3339(),
            "hostname": hostname::get()
                .ok()
                .and_then(|h| h.into_string().ok())
                .unwrap_or_else(|| "unknown".to_string()),
        });

        let lock_content = serde_json::to_string_pretty(&lock_data)
            .context("Failed to serialize lock file data")?;

        fs::write(&lock_file, lock_content)
            .with_context(|| format!("Failed to create lock file for {}", worktree_name))?;

        Ok(())
    }

    /// Remove a lock file (for agent release operations)
    pub async fn remove_lock_file(&self, imi_dir: &Path, worktree_name: &str) -> Result<()> {
        let lock_file = imi_dir.join("presence").join(format!("{}.lock", worktree_name));
        if lock_file.exists() {
            fs::remove_file(&lock_file)
                .with_context(|| format!("Failed to remove lock file for {}", worktree_name))?;
        }
        Ok(())
    }

    /// Read lock file data
    pub async fn read_lock_file(
        &self,
        imi_dir: &Path,
        worktree_name: &str,
    ) -> Result<serde_json::Value> {
        let lock_file = imi_dir.join("presence").join(format!("{}.lock", worktree_name));
        let content = fs::read_to_string(&lock_file)
            .with_context(|| format!("Failed to read lock file for {}", worktree_name))?;
        let data: serde_json::Value = serde_json::from_str(&content)
            .context("Failed to parse lock file data")?;
        Ok(data)
    }
}
