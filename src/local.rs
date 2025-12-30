use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Manages the "Data Plane" (.iMi directory) for a specific project.
/// Optimized for speed and shell consumption (Starship).
pub struct LocalContext {
    /// The root of the project (where .iMi lives)
    root: PathBuf,
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
            root: project_root.to_path_buf(),
            presence_dir: imi_dir.join("presence"),
            links_dir: imi_dir.join("links"),
            registry_file: imi_dir.join("registry.toml"),
            imi_dir,
        }
    }

    /// Ensure the .iMi directory structure exists (Idempotent)
    pub fn init(&self) -> Result<()> {
        if !self.imi_dir.exists() {
            fs::create_dir_all(&self.imi_dir)
                .context("Failed to create .iMi directory")?;
        }
        
        fs::create_dir_all(&self.presence_dir)
            .context("Failed to create .iMi/presence directory")?;
            
        fs::create_dir_all(&self.links_dir)
            .context("Failed to create .iMi/links directory")?;

        // Initialize empty registry if missing
        if !self.registry_file.exists() {
            let registry = LocalRegistry::default();
            let toml = toml::to_string_pretty(&registry)?;
            fs::write(&self.registry_file, toml)?;
        }

        Ok(())
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
        self.presence_dir.join(format!("{}.lock", worktree_name)).exists()
    }

    /// Register a new worktree in the local cache (Dual-Write)
    /// This allows Starship to look up types without guessing from folder names
    pub fn register_worktree(&self, name: &str, worktree_type: &str, agent: Option<&str>) -> Result<()> {
        self.init()?;

        // Read-Modify-Write the TOML registry
        // We use std::fs because this file is small and high-frequency read/write
        let content = fs::read_to_string(&self.registry_file).unwrap_or_default();
        let mut registry: LocalRegistry = toml::from_str(&content).unwrap_or_default();

        registry.worktrees.insert(name.to_string(), WorktreeMetadata {
            worktree_type: worktree_type.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            agent_owner: agent.map(|s| s.to_string()),
        });

        let new_content = toml::to_string_pretty(&registry)?;
        fs::write(&self.registry_file, new_content)?;

        Ok(())
    }

    /// Remove a worktree from the local cache
    pub fn unregister_worktree(&self, name: &str) -> Result<()> {
        if !self.registry_file.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(&self.registry_file)?;
        let mut registry: LocalRegistry = toml::from_str(&content)?;

        if registry.worktrees.remove(name).is_some() {
            let new_content = toml::to_string_pretty(&registry)?;
            fs::write(&self.registry_file, new_content)?;
        }

        // Also ensure lock is cleaned up
        self.unlock_worktree(name)?;

        Ok(())
    }
    
    /// Get the path to the 'links' directory for storing shared env files
    pub fn links_path(&self) -> &Path {
        &self.links_dir
    }
}
