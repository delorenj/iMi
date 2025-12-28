use anyhow::{Context, Result};
use colored::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::config::{ProjectConfig, StackType};
use super::templates::TemplateEngine;
use crate::github::GitHubClient;

pub struct ProjectCreator {
    template_engine: TemplateEngine,
}

impl ProjectCreator {
    pub fn new() -> Result<Self> {
        Ok(Self {
            template_engine: TemplateEngine::new()?,
        })
    }

    /// Create a new project with full scaffolding
    pub async fn create_project(&self, config: ProjectConfig) -> Result<PathBuf> {
        println!(
            "{} Creating project: {}",
            "🚀".bright_cyan(),
            config.name.bright_green()
        );

        // Step 1: Create GitHub repository
        println!("  {} Creating GitHub repository...", "→".bright_blue());
        let github_client = GitHubClient::new().await?;
        let repo_url = github_client.create_repository(&config).await?;
        println!(
            "  {} Repository created: {}",
            "✓".bright_green(),
            repo_url.bright_cyan()
        );

        // Step 2: Create local project directory
        let project_path = self.create_local_directory(&config)?;
        println!(
            "  {} Project directory: {}",
            "✓".bright_green(),
            project_path.display()
        );

        // Step 3: Generate boilerplate files
        println!("  {} Generating boilerplate files...", "→".bright_blue());
        self.scaffold_project(&project_path, &config)?;
        println!("  {} Boilerplate generated", "✓".bright_green());

        // Step 4: Initialize git repository
        println!("  {} Initializing git repository...", "→".bright_blue());
        self.initialize_git(&project_path, &config, &github_client)?;
        println!("  {} Git initialized and pushed", "✓".bright_green());

        // Step 5: Output success message
        self.print_success_message(&config, &repo_url, &project_path);

        Ok(project_path)
    }

    /// Create local project directory
    fn create_local_directory(&self, config: &ProjectConfig) -> Result<PathBuf> {
        let code_dir = dirs::home_dir()
            .context("Could not determine home directory")?
            .join("code");

        let project_path = code_dir.join(&config.name);

        if project_path.exists() {
            return Err(anyhow::anyhow!(
                "Directory '{}' already exists. Please choose a different name or remove the existing directory.",
                project_path.display()
            ));
        }

        fs::create_dir_all(&project_path).context("Failed to create project directory")?;

        Ok(project_path)
    }

    /// Scaffold project with boilerplate files
    fn scaffold_project(&self, project_path: &Path, config: &ProjectConfig) -> Result<()> {
        // Generate mise.toml
        let mise_content = self.template_engine.render_mise_toml(config)?;
        fs::write(project_path.join("mise.toml"), mise_content)?;

        // Create .mise/tasks directory
        let tasks_dir = project_path.join(".mise").join("tasks");
        fs::create_dir_all(&tasks_dir)?;

        // Generate stack-specific files
        match &config.stack {
            StackType::PythonFastAPI => self.scaffold_python_project(project_path, config)?,
            StackType::ReactVite => self.scaffold_react_project(project_path, config)?,
            StackType::Generic { .. } => self.scaffold_generic_project(project_path, config)?,
        }

        // Generate docker compose if databases are required
        if !config.databases.is_empty() {
            let compose_content = self.template_engine.render_docker_compose(config)?;
            fs::write(project_path.join("compose.yml"), compose_content)?;
        }

        // Generate README
        let readme_content = self.template_engine.render_readme(config)?;
        fs::write(project_path.join("README.md"), readme_content)?;

        // Create .gitignore
        self.create_gitignore(project_path, &config.stack)?;

        Ok(())
    }

    fn scaffold_python_project(&self, project_path: &Path, config: &ProjectConfig) -> Result<()> {
        // Generate pyproject.toml
        let pyproject_content = self.template_engine.render_python_pyproject(config)?;
        fs::write(project_path.join("pyproject.toml"), pyproject_content)?;

        // Create src directory structure
        let package_name = config.name.to_lowercase().replace('-', "_");
        let src_dir = project_path.join("src").join(&package_name);
        fs::create_dir_all(&src_dir)?;

        // Create __init__.py
        fs::write(src_dir.join("__init__.py"), "")?;

        // Create main.py with basic FastAPI app
        let main_content = format!(
            r#"from fastapi import FastAPI

app = FastAPI(title="{}")

@app.get("/")
async def root():
    return {{"message": "Hello from {}"}}

@app.get("/health")
async def health():
    return {{"status": "healthy"}}
"#,
            config.name, config.name
        );
        fs::write(src_dir.join("main.py"), main_content)?;

        // Create tests directory
        let tests_dir = project_path.join("tests");
        fs::create_dir_all(&tests_dir)?;
        fs::write(tests_dir.join("__init__.py"), "")?;

        Ok(())
    }

    fn scaffold_react_project(&self, project_path: &Path, config: &ProjectConfig) -> Result<()> {
        // Generate package.json
        let package_json_content = self.template_engine.render_react_package_json(config)?;
        fs::write(project_path.join("package.json"), package_json_content)?;

        // Generate tsconfig.json
        let tsconfig_content = self.template_engine.render_react_tsconfig(config)?;
        fs::write(project_path.join("tsconfig.json"), tsconfig_content)?;

        // Generate vite.config.ts
        let vite_config_content = self.template_engine.render_react_vite_config(config)?;
        fs::write(project_path.join("vite.config.ts"), vite_config_content)?;

        // Generate tailwind.config.js
        let tailwind_config_content = self.template_engine.render_react_tailwind_config(config)?;
        fs::write(project_path.join("tailwind.config.js"), tailwind_config_content)?;

        // Create src directory structure
        let src_dir = project_path.join("src");
        fs::create_dir_all(&src_dir)?;

        // Create App.tsx
        let app_content = r#"import { useState } from 'react'
import './App.css'

function App() {
  const [count, setCount] = useState(0)

  return (
    <div className="min-h-screen bg-gray-100 flex items-center justify-center">
      <div className="bg-white p-8 rounded-lg shadow-lg">
        <h1 className="text-4xl font-bold mb-4">Welcome to Your Project</h1>
        <button
          className="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded"
          onClick={() => setCount((count) => count + 1)}
        >
          count is {count}
        </button>
      </div>
    </div>
  )
}

export default App
"#;
        fs::write(src_dir.join("App.tsx"), app_content)?;

        // Create main.tsx
        let main_content = r#"import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App.tsx'
import './index.css'

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
)
"#;
        fs::write(src_dir.join("main.tsx"), main_content)?;

        // Create index.css with Tailwind directives
        let index_css_content = r#"@tailwind base;
@tailwind components;
@tailwind utilities;

body {
  margin: 0;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', 'Oxygen',
    'Ubuntu', 'Cantarell', 'Fira Sans', 'Droid Sans', 'Helvetica Neue',
    sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}
"#;
        fs::write(src_dir.join("index.css"), index_css_content)?;

        fs::write(src_dir.join("App.css"), "")?;

        // Create public directory
        let public_dir = project_path.join("public");
        fs::create_dir_all(&public_dir)?;

        // Create index.html
        let index_html_content = format!(
            r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>{}</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
"#,
            config.name
        );
        fs::write(project_path.join("index.html"), index_html_content)?;

        Ok(())
    }

    fn scaffold_generic_project(&self, project_path: &Path, _config: &ProjectConfig) -> Result<()> {
        // Create basic directory structure
        let src_dir = project_path.join("src");
        fs::create_dir_all(&src_dir)?;

        Ok(())
    }

    fn create_gitignore(&self, project_path: &Path, stack: &StackType) -> Result<()> {
        let mut gitignore_content = String::new();

        // Common ignores
        gitignore_content.push_str("# IDE\n.vscode/\n.idea/\n\n");
        gitignore_content.push_str("# OS\n.DS_Store\nThumbs.db\n\n");

        // Stack-specific ignores
        match stack {
            StackType::PythonFastAPI => {
                gitignore_content.push_str("# Python\n");
                gitignore_content.push_str("__pycache__/\n*.py[cod]\n");
                gitignore_content.push_str(".venv/\n.env\n");
                gitignore_content.push_str("*.egg-info/\ndist/\nbuild/\n\n");
            }
            StackType::ReactVite => {
                gitignore_content.push_str("# Node\n");
                gitignore_content.push_str("node_modules/\n");
                gitignore_content.push_str("dist/\n.env\n");
                gitignore_content.push_str("*.log\n\n");
            }
            StackType::Generic { .. } => {
                gitignore_content.push_str("# Build\ndist/\nbuild/\n\n");
            }
        }

        fs::write(project_path.join(".gitignore"), gitignore_content)?;

        Ok(())
    }

    /// Initialize git repository and push to GitHub
    fn initialize_git(
        &self,
        project_path: &Path,
        config: &ProjectConfig,
        github_client: &GitHubClient,
    ) -> Result<()> {
        // Initialize git
        Command::new("git")
            .args(&["init"])
            .current_dir(project_path)
            .output()
            .context("Failed to initialize git repository")?;

        // Add all files
        Command::new("git")
            .args(&["add", "."])
            .current_dir(project_path)
            .output()
            .context("Failed to add files to git")?;

        // Create initial commit
        Command::new("git")
            .args(&["commit", "-m", "Initial commit: Project scaffolding"])
            .current_dir(project_path)
            .output()
            .context("Failed to create initial commit")?;

        // Rename branch to main
        Command::new("git")
            .args(&["branch", "-M", "main"])
            .current_dir(project_path)
            .output()
            .context("Failed to rename branch to main")?;

        // Add remote
        let remote_url = github_client.get_clone_url(&config.name);
        Command::new("git")
            .args(&["remote", "add", "origin", &remote_url])
            .current_dir(project_path)
            .output()
            .context("Failed to add git remote")?;

        // Push to GitHub
        let push_result = Command::new("git")
            .args(&["push", "-u", "origin", "main"])
            .current_dir(project_path)
            .output()
            .context("Failed to push to GitHub")?;

        if !push_result.status.success() {
            let error = String::from_utf8_lossy(&push_result.stderr);
            return Err(anyhow::anyhow!("Failed to push to GitHub: {}", error));
        }

        Ok(())
    }

    fn print_success_message(&self, config: &ProjectConfig, repo_url: &str, project_path: &Path) {
        println!("\n{}", "✨ Success! Your project is ready.".bright_green().bold());
        println!();
        println!("  {} {}", "Repository:".bright_cyan(), repo_url.bright_white());
        println!(
            "  {} {}",
            "Local path:".bright_cyan(),
            project_path.display().to_string().bright_white()
        );
        println!();
        println!("{}", "Next steps:".bright_yellow().bold());
        println!("  {} {}", "1.".bright_blue(), format!("cd {}", config.name).bright_white());
        println!("  {} {}", "2.".bright_blue(), "mise install".bright_white());

        match &config.stack {
            StackType::PythonFastAPI => {
                println!("  {} {}", "3.".bright_blue(), "uv sync".bright_white());
                println!("  {} {}", "4.".bright_blue(), "mise run dev".bright_white());
            }
            StackType::ReactVite => {
                println!("  {} {}", "3.".bright_blue(), "bun install".bright_white());
                println!("  {} {}", "4.".bright_blue(), "mise run dev".bright_white());
            }
            StackType::Generic { .. } => {
                println!("  {} {}", "3.".bright_blue(), "mise run dev".bright_white());
            }
        }

        println!();
    }
}
