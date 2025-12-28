use anyhow::{anyhow, Context, Result};

use crate::commands::project::config::{ProjectConfig, RepoVisibility};

pub struct GitHubClient {
    client: reqwest::Client,
    token: String,
    username: String,
}

impl GitHubClient {
    /// Create new GitHub client from environment token
    pub async fn new() -> Result<Self> {
        let token = std::env::var("GITHUB_TOKEN").context(
            "GITHUB_TOKEN environment variable not set. \
             Run: export GITHUB_TOKEN=<your-token> \
             or use: gh auth login",
        )?;

        let client = reqwest::Client::builder()
            .user_agent("iMi-Project-Creator/0.1.0")
            .build()
            .context("Failed to create HTTP client")?;

        // Get authenticated user
        let user_response = client
            .get("https://api.github.com/user")
            .header("Authorization", format!("Bearer {}", token))
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .context("Failed to fetch authenticated user")?;

        let user: serde_json::Value = user_response
            .json()
            .await
            .context("Failed to parse user response")?;

        let username = user["login"]
            .as_str()
            .context("Failed to get username from response")?
            .to_string();

        Ok(Self {
            client,
            token,
            username,
        })
    }

    /// Create a new GitHub repository
    pub async fn create_repository(&self, config: &ProjectConfig) -> Result<String> {
        let repo_name = &config.name;

        // Check if repository already exists
        if self.repository_exists(repo_name).await? {
            return Err(anyhow!(
                "Repository '{}' already exists on GitHub. \
                 Please choose a different name or delete the existing repository.",
                repo_name
            ));
        }

        let is_private = matches!(config.visibility, RepoVisibility::Private);

        // Build the create repository request body
        let create_request = serde_json::json!({
            "name": repo_name,
            "description": config.description,
            "private": is_private,
            "auto_init": false,
        });

        // Create repository using GitHub REST API
        let response = self
            .client
            .post("https://api.github.com/user/repos")
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .json(&create_request)
            .send()
            .await
            .context("Failed to send create repository request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!(
                "Failed to create repository (status {}): {}",
                status,
                error_body
            ));
        }

        let repo: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse repository response")?;

        // Extract HTML URL from response
        let html_url = repo["html_url"]
            .as_str()
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("https://github.com/{}/{}", self.username, repo_name));

        Ok(html_url)
    }

    /// Check if repository exists
    async fn repository_exists(&self, repo_name: &str) -> Result<bool> {
        let response = self
            .client
            .get(&format!(
                "https://api.github.com/repos/{}/{}",
                self.username, repo_name
            ))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .send()
            .await?;

        Ok(response.status().is_success())
    }

    /// Get repository clone URL
    pub fn get_clone_url(&self, repo_name: &str) -> String {
        format!("git@github.com:{}/{}.git", self.username, repo_name)
    }

    /// Get repository HTTPS URL
    pub fn get_https_url(&self, repo_name: &str) -> String {
        format!("https://github.com/{}/{}", self.username, repo_name)
    }
}

/// Check GitHub authentication status
pub fn check_auth() -> Result<()> {
    if std::env::var("GITHUB_TOKEN").is_ok() {
        return Ok(());
    }

    // Check if gh CLI is authenticated
    let gh_status = std::process::Command::new("gh")
        .args(&["auth", "status"])
        .output();

    match gh_status {
        Ok(output) if output.status.success() => Ok(()),
        _ => Err(anyhow!(
            "GitHub authentication not configured. \
             \n\nOption 1: Use GitHub CLI\n  gh auth login\n\n\
             Option 2: Set environment variable\n  export GITHUB_TOKEN=<your-token>"
        )),
    }
}

/// Show authentication help message
pub fn show_auth_help() {
    eprintln!("\n🔐 GitHub Authentication Required\n");
    eprintln!("To authenticate with GitHub, choose one of the following:\n");
    eprintln!("Option 1: GitHub CLI (Recommended)");
    eprintln!("  gh auth login");
    eprintln!("  gh auth setup-git\n");
    eprintln!("Option 2: Personal Access Token");
    eprintln!("  1. Create token at: https://github.com/settings/tokens/new");
    eprintln!("  2. Select scopes: 'repo', 'workflow'");
    eprintln!("  3. Export token: export GITHUB_TOKEN=<your-token>\n");
}
