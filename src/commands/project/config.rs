use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Project configuration for scaffolding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    pub description: String,
    pub stack: StackType,
    pub databases: Vec<DatabaseType>,
    pub mise_tasks: Vec<String>,
    pub visibility: RepoVisibility,
}

/// Supported stack types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StackType {
    PythonFastAPI,
    ReactVite,
    Generic { language: Option<String> },
}

/// Database service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatabaseType {
    Postgres {
        host: String,
        port: u16,
        user: String,
        password: String,
    },
    Redis {
        host: String,
        port: u16,
    },
    Qdrant {
        url: String,
    },
}

/// Repository visibility
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RepoVisibility {
    Public,
    Private,
}

impl Default for RepoVisibility {
    fn default() -> Self {
        RepoVisibility::Public
    }
}

impl ProjectConfig {
    pub fn new(name: String, description: String) -> Self {
        Self {
            name,
            description,
            stack: StackType::Generic { language: None },
            databases: Vec::new(),
            mise_tasks: Vec::new(),
            visibility: RepoVisibility::default(),
        }
    }

    /// Build ProjectConfig from natural language concept
    pub fn from_concept(concept: &str, name: Option<String>) -> anyhow::Result<Self> {
        let stack = detect_stack(concept);
        let databases = detect_databases(concept);
        let project_name = name.unwrap_or_else(|| extract_or_generate_name(concept));

        let mise_tasks = default_mise_tasks_for_stack(&stack);

        Ok(Self {
            name: project_name,
            description: concept.to_string(),
            stack,
            databases,
            mise_tasks,
            visibility: RepoVisibility::Public,
        })
    }

    /// Build ProjectConfig from PRD markdown file
    pub fn from_prd(prd_path: &str, name: Option<String>) -> anyhow::Result<Self> {
        let prd_content = std::fs::read_to_string(prd_path)?;

        // Extract project name from PRD or use provided name
        let project_name = name.unwrap_or_else(|| {
            extract_name_from_prd(&prd_content).unwrap_or_else(|| "MyProject".to_string())
        });

        let stack = detect_stack(&prd_content);
        let databases = detect_databases(&prd_content);
        let mise_tasks = default_mise_tasks_for_stack(&stack);

        Ok(Self {
            name: project_name,
            description: extract_description_from_prd(&prd_content),
            stack,
            databases,
            mise_tasks,
            visibility: RepoVisibility::Public,
        })
    }

    /// Build ProjectConfig from JSON payload
    pub fn from_json(json_str: &str) -> anyhow::Result<Self> {
        let payload: HashMap<String, serde_json::Value> = serde_json::from_str(json_str)?;

        let name = payload
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("MyProject")
            .to_string();

        let description = payload
            .get("concept")
            .or_else(|| payload.get("description"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Detect stack from explicit fields or description
        let stack = if let Some(api) = payload.get("api").and_then(|v| v.as_str()) {
            if api.to_lowercase().contains("fastapi") {
                StackType::PythonFastAPI
            } else {
                StackType::Generic {
                    language: Some(api.to_string()),
                }
            }
        } else if let Some(frontend) = payload.get("frontend").and_then(|v| v.as_str()) {
            if frontend.to_lowercase().contains("react") {
                StackType::ReactVite
            } else {
                StackType::Generic {
                    language: Some(frontend.to_string()),
                }
            }
        } else {
            detect_stack(&description)
        };

        let databases = detect_databases(&description);

        let mise_tasks = payload
            .get("mise-tasks")
            .or_else(|| payload.get("mise_tasks"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_else(|| default_mise_tasks_for_stack(&stack));

        Ok(Self {
            name,
            description,
            stack,
            databases,
            mise_tasks,
            visibility: RepoVisibility::Public,
        })
    }
}

/// Detect stack from text content using keyword heuristics
fn detect_stack(content: &str) -> StackType {
    let content_lower = content.to_lowercase();

    // Check for Python/FastAPI indicators
    if content_lower.contains("fastapi")
        || (content_lower.contains("python") && content_lower.contains("api"))
    {
        return StackType::PythonFastAPI;
    }

    // Check for React indicators
    if content_lower.contains("react")
        || content_lower.contains("vite")
        || content_lower.contains("typescript")
            && (content_lower.contains("dashboard")
                || content_lower.contains("frontend")
                || content_lower.contains("ui"))
    {
        return StackType::ReactVite;
    }

    // Fallback to generic
    StackType::Generic { language: None }
}

/// Detect database requirements from text content
fn detect_databases(content: &str) -> Vec<DatabaseType> {
    let content_lower = content.to_lowercase();
    let mut databases = Vec::new();

    if content_lower.contains("postgres") || content_lower.contains("postgresql") {
        databases.push(DatabaseType::Postgres {
            host: "192.168.1.12".to_string(),
            port: 5432,
            user: std::env::var("DEFAULT_USERNAME").unwrap_or_else(|_| "postgres".to_string()),
            password: std::env::var("DEFAULT_PASSWORD").unwrap_or_else(|_| "password".to_string()),
        });
    }

    if content_lower.contains("redis") {
        databases.push(DatabaseType::Redis {
            host: "192.168.1.12".to_string(),
            port: 6743,
        });
    }

    if content_lower.contains("qdrant") || content_lower.contains("vector") {
        databases.push(DatabaseType::Qdrant {
            url: "qdrant.delo.sh".to_string(),
        });
    }

    databases
}

/// Extract or generate project name from concept
fn extract_or_generate_name(_concept: &str) -> String {
    // Simple heuristic: look for quoted names or capitalized words
    // For now, just generate a generic name
    // TODO: Could use LLM here for better name extraction
    "MyProject".to_string()
}

/// Extract project name from PRD markdown
fn extract_name_from_prd(prd_content: &str) -> Option<String> {
    // Look for "# ProjectName" or "Project: Name" patterns
    for line in prd_content.lines() {
        if line.starts_with("# ") {
            return Some(line.trim_start_matches("# ").trim().to_string());
        }
        if line.to_lowercase().starts_with("project:") {
            return Some(
                line.trim_start_matches("project:")
                    .trim_start_matches("Project:")
                    .trim()
                    .to_string(),
            );
        }
    }
    None
}

/// Extract description from PRD markdown
fn extract_description_from_prd(prd_content: &str) -> String {
    // Simple extraction: take first paragraph after title
    let lines: Vec<&str> = prd_content.lines().collect();
    let mut description = String::new();

    let mut started = false;
    for line in lines {
        if line.starts_with('#') && !started {
            started = true;
            continue;
        }
        if started && !line.is_empty() {
            description.push_str(line);
            description.push(' ');
            if description.len() > 200 {
                break;
            }
        }
    }

    if description.is_empty() {
        "A new project".to_string()
    } else {
        description.trim().to_string()
    }
}

/// Default mise tasks for each stack type
fn default_mise_tasks_for_stack(stack: &StackType) -> Vec<String> {
    match stack {
        StackType::PythonFastAPI => vec![
            "dev".to_string(),
            "test".to_string(),
            "lint".to_string(),
            "format".to_string(),
        ],
        StackType::ReactVite => vec![
            "dev".to_string(),
            "build".to_string(),
            "test".to_string(),
            "lint".to_string(),
        ],
        StackType::Generic { .. } => vec!["dev".to_string(), "test".to_string()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_python_fastapi_stack() {
        let result = detect_stack("Create a Python API using FastAPI");
        assert_eq!(result, StackType::PythonFastAPI);
    }

    #[test]
    fn test_detect_react_stack() {
        let result = detect_stack("Build a React dashboard with TypeScript");
        assert_eq!(result, StackType::ReactVite);
    }

    #[test]
    fn test_detect_postgres_database() {
        let dbs = detect_databases("We need PostgreSQL for data storage");
        assert_eq!(dbs.len(), 1);
        matches!(dbs[0], DatabaseType::Postgres { .. });
    }

    #[test]
    fn test_from_json_payload() {
        let json = r#"{
            "name": "TestProject",
            "api": "FastAPI",
            "frontend": "react dashboard",
            "mise-tasks": ["hello-world"]
        }"#;

        let config = ProjectConfig::from_json(json).unwrap();
        assert_eq!(config.name, "TestProject");
        assert_eq!(config.stack, StackType::PythonFastAPI);
        assert_eq!(config.mise_tasks, vec!["hello-world"]);
    }
}
