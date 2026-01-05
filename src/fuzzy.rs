use anyhow::Result;
use std::path::PathBuf;

use crate::database::{Database, Repository, Worktree};

/// Represents a searchable target (worktree or repository)
#[derive(Debug, Clone)]
pub enum SearchTarget {
    Worktree { worktree: Worktree, score: f64 },
    Repository { repository: Repository, score: f64 },
}

impl SearchTarget {
    pub fn path(&self) -> PathBuf {
        match self {
            SearchTarget::Worktree { worktree, .. } => PathBuf::from(&worktree.path),
            SearchTarget::Repository { repository, .. } => PathBuf::from(&repository.path),
        }
    }

    pub fn display_name(&self) -> String {
        match self {
            SearchTarget::Worktree { worktree, .. } => {
                format!("{} [{}]", worktree.worktree_name, worktree.branch_name)
            }
            SearchTarget::Repository { repository, .. } => {
                format!("{} (repo)", repository.name)
            }
        }
    }

    pub fn score(&self) -> f64 {
        match self {
            SearchTarget::Worktree { score, .. } => *score,
            SearchTarget::Repository { score, .. } => *score,
        }
    }

    pub fn repo_name(&self) -> &str {
        match self {
            SearchTarget::Worktree { worktree, .. } => &worktree.repo_name,
            SearchTarget::Repository { repository, .. } => &repository.name,
        }
    }

    pub fn worktree_type(&self) -> Option<&str> {
        match self {
            SearchTarget::Worktree { worktree, .. } => Some(&worktree.worktree_type),
            SearchTarget::Repository { .. } => None,
        }
    }
}

pub struct FuzzyMatcher {
    db: Database,
}

impl FuzzyMatcher {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Fuzzy search across worktrees and repositories
    pub async fn search(
        &self,
        query: &str,
        repo_filter: Option<&str>,
        worktrees_only: bool,
        include_inactive: bool,
    ) -> Result<Vec<SearchTarget>> {
        let mut targets = Vec::new();

        // Search worktrees
        let worktrees = if include_inactive {
            self.db.list_all_worktrees(repo_filter).await?
        } else {
            self.db.list_worktrees(repo_filter).await?
        };

        for worktree in worktrees {
            let score = self.calculate_score(query, &worktree);
            if score > 0.0 {
                targets.push(SearchTarget::Worktree { worktree, score });
            }
        }

        // Search repositories (unless worktrees_only flag is set)
        if !worktrees_only && repo_filter.is_none() {
            let repositories = self.db.list_repositories().await?;
            for repository in repositories {
                let score = self.calculate_repo_score(query, &repository);
                if score > 0.0 {
                    targets.push(SearchTarget::Repository { repository, score });
                }
            }
        }

        // Sort by score (highest first)
        targets.sort_by(|a, b| {
            b.score()
                .partial_cmp(&a.score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(targets)
    }

    /// Show interactive picker when no query is provided
    pub async fn interactive_select(
        &self,
        repo_filter: Option<&str>,
        worktrees_only: bool,
    ) -> Result<Option<SearchTarget>> {
        use dialoguer::{theme::ColorfulTheme, Select};

        let mut targets = Vec::new();

        // Get all worktrees
        let worktrees = self.db.list_worktrees(repo_filter).await?;
        for worktree in worktrees {
            targets.push(SearchTarget::Worktree {
                worktree,
                score: 1.0, // All targets have equal score in interactive mode
            });
        }

        // Get all repositories (unless worktrees_only)
        if !worktrees_only && repo_filter.is_none() {
            let repositories = self.db.list_repositories().await?;
            for repository in repositories {
                targets.push(SearchTarget::Repository {
                    repository,
                    score: 1.0,
                });
            }
        }

        if targets.is_empty() {
            return Ok(None);
        }

        // Format display names with icons
        let display_items: Vec<String> = targets
            .iter()
            .map(|target| match target {
                SearchTarget::Worktree { worktree, .. } => {
                    let icon = match worktree.worktree_type.as_str() {
                        "feat" => "🚀",
                        "pr" => "🔍",
                        "fix" => "🔧",
                        "aiops" => "🤖",
                        "devops" => "⚙️",
                        "trunk" => "🌳",
                        _ => "📁",
                    };
                    format!(
                        "{} {} [{}] ({})",
                        icon, worktree.worktree_name, worktree.branch_name, worktree.repo_name
                    )
                }
                SearchTarget::Repository { repository, .. } => {
                    format!("📦 {} (repository)", repository.name)
                }
            })
            .collect();

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select a worktree or repository")
            .items(&display_items)
            .default(0)
            .interact_opt()?;

        Ok(selection.map(|idx| targets[idx].clone()))
    }

    /// Calculate fuzzy match score for a worktree (0.0 to 1.0)
    fn calculate_score(&self, query: &str, worktree: &Worktree) -> f64 {
        let query_lower = query.to_lowercase();

        // Exact match gets highest score
        if worktree.worktree_name.to_lowercase() == query_lower {
            return 1.0;
        }

        // Contains match in worktree name gets high score
        if worktree.worktree_name.to_lowercase().contains(&query_lower) {
            return 0.8;
        }

        // Branch name match
        if worktree.branch_name.to_lowercase().contains(&query_lower) {
            return 0.7;
        }

        // Repository name match (lower priority)
        if worktree.repo_name.to_lowercase().contains(&query_lower) {
            return 0.5;
        }

        // Worktree type match (e.g., searching "feat" finds all feature branches)
        if worktree.worktree_type.to_lowercase().contains(&query_lower) {
            return 0.4;
        }

        // No match
        0.0
    }

    fn calculate_repo_score(&self, query: &str, repository: &Repository) -> f64 {
        let query_lower = query.to_lowercase();

        if repository.name.to_lowercase() == query_lower {
            return 1.0;
        }

        if repository.name.to_lowercase().contains(&query_lower) {
            return 0.6; // Lower than worktree matches to prioritize worktrees
        }

        0.0
    }
}
