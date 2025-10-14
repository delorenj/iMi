//! Common Test Utilities and Helpers
//!
//! This module provides reusable patterns, utilities, and helpers for testing
//! the iMi application. These utilities follow best practices for test isolation,
//! setup/cleanup, and consistent assertion patterns.

use anyhow::Result;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use tokio::fs;
use uuid::Uuid;

use imi::config::Config;
use imi::database::{AgentActivity, Database, Repository, Worktree};
use imi::git::GitManager;

/// Standardized test result for tracking test outcomes
#[derive(Debug, Clone)]
pub struct TestResult<T> {
    pub success: bool,
    pub result: Option<T>,
    pub error: Option<String>,
    pub duration_ms: u128,
}

impl<T> TestResult<T> {
    pub fn success(result: T, duration_ms: u128) -> Self {
        Self {
            success: true,
            result: Some(result),
            error: None,
            duration_ms,
        }
    }

    pub fn failure(error: String, duration_ms: u128) -> Self {
        Self {
            success: false,
            result: None,
            error: Some(error),
            duration_ms,
        }
    }

    pub fn is_success(&self) -> bool {
        self.success
    }

    pub fn unwrap(self) -> T {
        self.result.expect("TestResult contained no success result")
    }

    pub fn unwrap_error(self) -> String {
        self.error.expect("TestResult contained no error")
    }
}

/// Macro for timing test operations
#[macro_export]
macro_rules! time_operation {
    ($operation:expr) => {{
        let start = std::time::Instant::now();
        let result = $operation;
        let duration = start.elapsed().as_millis();
        (result, duration)
    }};
}

/// Comprehensive test environment that combines all necessary components
pub struct TestEnvironment {
    pub temp_dir: TempDir,
    pub config: Config,
    pub database: Database,
    pub git_manager: GitManager,
    pub test_data: TestDataBuilder,
    cleanup_tasks: Vec<Box<dyn FnOnce() -> Result<()> + Send>>,
}

impl TestEnvironment {
    /// Create a new comprehensive test environment
    pub async fn new() -> Result<Self> {
        let temp_dir = TempDir::new()?;

        // Create test config with temporary paths
        let mut config = Config::default();
        config.database_path = temp_dir.path().join("test.db");
        config.root_path = temp_dir.path().to_path_buf();

        // Initialize database
        let database = Database::new(&config.database_path).await?;

        // Initialize git manager
        let git_manager = GitManager::new();

        // Create test data builder
        let test_data = TestDataBuilder::new();

        Ok(Self {
            temp_dir,
            config,
            database,
            git_manager,
            test_data,
            cleanup_tasks: Vec::new(),
        })
    }

    /// Add a cleanup task to run when the environment is dropped
    pub fn add_cleanup_task<F>(&mut self, task: F)
    where
        F: FnOnce() -> Result<()> + Send + 'static,
    {
        self.cleanup_tasks.push(Box::new(task));
    }

    /// Get the root path for test operations
    pub fn root_path(&self) -> &Path {
        self.temp_dir.path()
    }

    /// Create a test repository directory structure
    pub async fn create_test_repo(&self, name: &str) -> Result<PathBuf> {
        let repo_path = self.root_path().join(name);
        fs::create_dir_all(&repo_path).await?;

        // Create mock .git directory
        let git_dir = repo_path.join(".git");
        fs::create_dir_all(&git_dir).await?;
        fs::write(git_dir.join("HEAD"), "ref: refs/heads/main").await?;

        Ok(repo_path)
    }

    /// Create a test worktree structure
    pub async fn create_test_worktree(
        &self,
        repo_name: &str,
        worktree_name: &str,
    ) -> Result<PathBuf> {
        let worktree_path = self.root_path().join(repo_name).join(worktree_name);
        fs::create_dir_all(&worktree_path).await?;

        // Create mock .git file (worktree reference)
        let git_file = worktree_path.join(".git");
        fs::write(
            git_file,
            format!(
                "gitdir: {}/.git/worktrees/{}",
                self.root_path().join(repo_name).display(),
                worktree_name
            ),
        )
        .await?;

        Ok(worktree_path)
    }

    /// Run a test with automatic timing and error capture
    pub async fn run_test<F, T>(&self, test_name: &str, test_fn: F) -> TestResult<T>
    where
        F: std::future::Future<Output = Result<T>>,
    {
        let start = std::time::Instant::now();
        match test_fn.await {
            Ok(result) => {
                let duration = start.elapsed().as_millis();
                TestResult::success(result, duration)
            }
            Err(error) => {
                let duration = start.elapsed().as_millis();
                TestResult::failure(format!("{}: {}", test_name, error), duration)
            }
        }
    }
}

impl Drop for TestEnvironment {
    fn drop(&mut self) {
        // Run cleanup tasks
        for task in self.cleanup_tasks.drain(..) {
            if let Err(e) = task() {
                eprintln!("Cleanup task failed: {}", e);
            }
        }
    }
}

/// Builder pattern for creating test data with realistic values
#[derive(Debug, Clone)]
pub struct TestDataBuilder {
    repositories: HashMap<String, Repository>,
    worktrees: HashMap<String, Worktree>,
    activities: HashMap<String, AgentActivity>,
}

impl TestDataBuilder {
    pub fn new() -> Self {
        Self {
            repositories: HashMap::new(),
            worktrees: HashMap::new(),
            activities: HashMap::new(),
        }
    }

    /// Create a realistic test repository
    pub fn repository(&mut self, name: &str) -> RepositoryBuilder {
        RepositoryBuilder::new(name, self)
    }

    /// Create a realistic test worktree
    pub fn worktree(&mut self, repo_name: &str, name: &str) -> WorktreeBuilder {
        WorktreeBuilder::new(repo_name, name, self)
    }

    /// Create a realistic test activity
    pub fn activity(&mut self, worktree_id: &str) -> ActivityBuilder {
        ActivityBuilder::new(worktree_id, self)
    }

    /// Get all created repositories
    pub fn repositories(&self) -> &HashMap<String, Repository> {
        &self.repositories
    }

    /// Get all created worktrees
    pub fn worktrees(&self) -> &HashMap<String, Worktree> {
        &self.worktrees
    }

    /// Get all created activities
    pub fn activities(&self) -> &HashMap<String, AgentActivity> {
        &self.activities
    }
}

/// Builder for creating test repositories with realistic data
pub struct RepositoryBuilder<'a> {
    name: String,
    path: Option<String>,
    remote_url: Option<String>,
    default_branch: String,
    active: bool,
    builder: &'a mut TestDataBuilder,
}

impl<'a> RepositoryBuilder<'a> {
    fn new(name: &str, builder: &'a mut TestDataBuilder) -> Self {
        Self {
            name: name.to_string(),
            path: None,
            remote_url: None,
            default_branch: "main".to_string(),
            active: true,
            builder,
        }
    }

    pub fn path<P: AsRef<str>>(mut self, path: P) -> Self {
        self.path = Some(path.as_ref().to_string());
        self
    }

    pub fn remote_url<U: AsRef<str>>(mut self, url: U) -> Self {
        self.remote_url = Some(url.as_ref().to_string());
        self
    }

    pub fn default_branch<B: AsRef<str>>(mut self, branch: B) -> Self {
        self.default_branch = branch.as_ref().to_string();
        self
    }

    pub fn inactive(mut self) -> Self {
        self.active = false;
        self
    }

    pub fn build(self) -> Repository {
        let now = Utc::now();
        let repository = Repository {
            id: Uuid::new_v4().to_string(),
            name: self.name.clone(),
            path: self.path.unwrap_or_else(|| format!("/tmp/{}", self.name)),
            remote_url: self
                .remote_url
                .unwrap_or_else(|| format!("https://github.com/test/{}.git", self.name)),
            default_branch: self.default_branch,
            created_at: now,
            updated_at: now,
            active: self.active,
        };

        self.builder
            .repositories
            .insert(self.name, repository.clone());
        repository
    }
}

/// Builder for creating test worktrees with realistic data
pub struct WorktreeBuilder<'a> {
    repo_name: String,
    worktree_name: String,
    branch_name: Option<String>,
    worktree_type: String,
    path: Option<String>,
    active: bool,
    agent_id: Option<String>,
    builder: &'a mut TestDataBuilder,
}

impl<'a> WorktreeBuilder<'a> {
    fn new(repo_name: &str, worktree_name: &str, builder: &'a mut TestDataBuilder) -> Self {
        Self {
            repo_name: repo_name.to_string(),
            worktree_name: worktree_name.to_string(),
            branch_name: None,
            worktree_type: "feat".to_string(),
            path: None,
            active: true,
            agent_id: None,
            builder,
        }
    }

    pub fn branch_name<B: AsRef<str>>(mut self, branch: B) -> Self {
        self.branch_name = Some(branch.as_ref().to_string());
        self
    }

    pub fn worktree_type<T: AsRef<str>>(mut self, wt_type: T) -> Self {
        self.worktree_type = wt_type.as_ref().to_string();
        self
    }

    pub fn path<P: AsRef<str>>(mut self, path: P) -> Self {
        self.path = Some(path.as_ref().to_string());
        self
    }

    pub fn inactive(mut self) -> Self {
        self.active = false;
        self
    }

    pub fn agent_id<A: AsRef<str>>(mut self, agent: A) -> Self {
        self.agent_id = Some(agent.as_ref().to_string());
        self
    }

    pub fn build(self) -> Worktree {
        let now = Utc::now();
        let worktree = Worktree {
            id: Uuid::new_v4().to_string(),
            repo_name: self.repo_name.clone(),
            worktree_name: self.worktree_name.clone(),
            branch_name: self
                .branch_name
                .unwrap_or_else(|| format!("feature/{}", self.worktree_name)),
            worktree_type: self.worktree_type,
            path: self
                .path
                .unwrap_or_else(|| format!("/tmp/{}/{}", self.repo_name, self.worktree_name)),
            created_at: now,
            updated_at: now,
            active: self.active,
            agent_id: self.agent_id,
        };

        let key = format!("{}:{}", self.repo_name, self.worktree_name);
        self.builder.worktrees.insert(key, worktree.clone());
        worktree
    }
}

/// Builder for creating test activities with realistic data
pub struct ActivityBuilder<'a> {
    worktree_id: String,
    agent_id: String,
    activity_type: String,
    file_path: Option<String>,
    description: String,
    builder: &'a mut TestDataBuilder,
}

impl<'a> ActivityBuilder<'a> {
    fn new(worktree_id: &str, builder: &'a mut TestDataBuilder) -> Self {
        Self {
            worktree_id: worktree_id.to_string(),
            agent_id: "test-agent".to_string(),
            activity_type: "created".to_string(),
            file_path: None,
            description: "Test activity".to_string(),
            builder,
        }
    }

    pub fn agent_id<A: AsRef<str>>(mut self, agent: A) -> Self {
        self.agent_id = agent.as_ref().to_string();
        self
    }

    pub fn activity_type<T: AsRef<str>>(mut self, activity: T) -> Self {
        self.activity_type = activity.as_ref().to_string();
        self
    }

    pub fn file_path<P: AsRef<str>>(mut self, path: P) -> Self {
        self.file_path = Some(path.as_ref().to_string());
        self
    }

    pub fn description<D: AsRef<str>>(mut self, desc: D) -> Self {
        self.description = desc.as_ref().to_string();
        self
    }

    pub fn build(self) -> AgentActivity {
        let activity = AgentActivity {
            id: Uuid::new_v4().to_string(),
            agent_id: self.agent_id,
            worktree_id: self.worktree_id.clone(),
            activity_type: self.activity_type,
            file_path: self.file_path,
            description: self.description,
            created_at: Utc::now(),
        };

        self.builder
            .activities
            .insert(self.worktree_id, activity.clone());
        activity
    }
}

/// Assertion utilities for common test patterns
pub struct AssertionUtils;

impl AssertionUtils {
    /// Assert that a timestamp is recent (within the last minute)
    pub fn assert_recent_timestamp(timestamp: &DateTime<Utc>, context: &str) {
        let now = Utc::now();
        let diff = now.signed_duration_since(*timestamp);
        assert!(
            diff.num_seconds() < 60 && diff.num_seconds() >= 0,
            "{}: timestamp should be recent. Got: {}, Now: {}, Diff: {}s",
            context,
            timestamp,
            now,
            diff.num_seconds()
        );
    }

    /// Assert that a UUID string is valid
    pub fn assert_valid_uuid(uuid_str: &str, context: &str) {
        let parsed = Uuid::parse_str(uuid_str);
        assert!(
            parsed.is_ok(),
            "{}: should be a valid UUID: {}",
            context,
            uuid_str
        );
    }

    /// Assert that a path exists and is accessible
    pub fn assert_path_accessible<P: AsRef<Path>>(path: P, context: &str) {
        let path = path.as_ref();
        assert!(
            path.exists(),
            "{}: path should exist: {}",
            context,
            path.display()
        );

        // Check if readable
        let metadata = std::fs::metadata(path);
        assert!(
            metadata.is_ok(),
            "{}: path should be readable: {}",
            context,
            path.display()
        );
    }

    /// Assert that two collections have the same length
    pub fn assert_same_length<T, U>(collection1: &[T], collection2: &[U], context: &str) {
        assert_eq!(
            collection1.len(),
            collection2.len(),
            "{}: collections should have same length. Got {} vs {}",
            context,
            collection1.len(),
            collection2.len()
        );
    }

    /// Assert that a collection is not empty
    pub fn assert_not_empty<T>(collection: &[T], context: &str) {
        assert!(
            !collection.is_empty(),
            "{}: collection should not be empty",
            context
        );
    }

    /// Assert that a string matches a pattern (contains substring)
    pub fn assert_contains(haystack: &str, needle: &str, context: &str) {
        assert!(
            haystack.contains(needle),
            "{}: '{}' should contain '{}'",
            context,
            haystack,
            needle
        );
    }

    /// Assert that an operation completed within expected time
    pub fn assert_performance(duration_ms: u128, max_expected_ms: u128, context: &str) {
        assert!(
            duration_ms <= max_expected_ms,
            "{}: operation took too long. Expected: {}ms, Actual: {}ms",
            context,
            max_expected_ms,
            duration_ms
        );
    }
}

/// Mock data generators for consistent test data
pub struct MockDataGenerator;

impl MockDataGenerator {
    /// Generate realistic repository names
    pub fn repository_names(count: usize) -> Vec<String> {
        let prefixes = vec![
            "web", "api", "core", "ui", "data", "auth", "admin", "mobile",
        ];
        let suffixes = vec![
            "app", "service", "lib", "tool", "engine", "client", "server", "worker",
        ];

        (0..count)
            .map(|i| {
                if i < prefixes.len() * suffixes.len() {
                    format!(
                        "{}-{}",
                        prefixes[i % prefixes.len()],
                        suffixes[i / prefixes.len()]
                    )
                } else {
                    format!("repo-{}", i)
                }
            })
            .collect()
    }

    /// Generate realistic worktree names  
    pub fn worktree_names(count: usize) -> Vec<String> {
        let types = vec!["feat", "fix", "refactor", "docs", "test", "chore"];
        let features = vec![
            "user-auth",
            "api-endpoints",
            "data-validation",
            "error-handling",
            "performance",
            "ui-improvements",
            "integration",
            "monitoring",
        ];

        (0..count)
            .map(|i| {
                if i < types.len() * features.len() {
                    format!("{}-{}", types[i % types.len()], features[i / types.len()])
                } else {
                    format!("worktree-{}", i)
                }
            })
            .collect()
    }

    /// Generate realistic remote URLs
    pub fn remote_urls(repo_names: &[String]) -> Vec<String> {
        let hosts = vec!["github.com", "gitlab.com", "bitbucket.org"];
        let users = vec!["acme", "team", "org", "company"];

        repo_names
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let host = hosts[i % hosts.len()];
                let user = users[i % users.len()];
                format!("https://{}/{}/{}.git", host, user, name)
            })
            .collect()
    }

    /// Generate realistic file paths for activities
    pub fn file_paths(count: usize) -> Vec<String> {
        let extensions = vec!["rs", "toml", "md", "json", "txt", "yml", "js", "py"];
        let directories = vec!["src", "tests", "docs", "config", "scripts", "examples"];

        (0..count)
            .map(|i| {
                let dir = directories[i % directories.len()];
                let ext = extensions[i % extensions.len()];
                format!("{}/file_{}.{}", dir, i, ext)
            })
            .collect()
    }
}

/// Performance testing utilities
pub struct PerformanceTestUtils;

impl PerformanceTestUtils {
    /// Measure the execution time of an async operation
    pub async fn measure_async<F, T>(operation: F) -> (Result<T>, u128)
    where
        F: std::future::Future<Output = Result<T>>,
    {
        let start = std::time::Instant::now();
        let result = operation.await;
        let duration = start.elapsed().as_millis();
        (result, duration)
    }

    /// Run multiple iterations of an operation and collect timing statistics
    pub async fn benchmark_async<F, T>(
        operation: F,
        iterations: usize,
        name: &str,
    ) -> BenchmarkResult
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>>
            + Send
            + Sync,
        T: Send,
    {
        let mut durations = Vec::with_capacity(iterations);
        let mut successes = 0;

        for _ in 0..iterations {
            let (result, duration) = Self::measure_async(operation()).await;
            durations.push(duration);
            if result.is_ok() {
                successes += 1;
            }
        }

        BenchmarkResult {
            name: name.to_string(),
            iterations,
            successes,
            failures: iterations - successes,
            min_ms: *durations.iter().min().unwrap_or(&0),
            max_ms: *durations.iter().max().unwrap_or(&0),
            avg_ms: durations.iter().sum::<u128>() / iterations as u128,
            median_ms: Self::calculate_median(&mut durations),
        }
    }

    fn calculate_median(durations: &mut [u128]) -> u128 {
        durations.sort_unstable();
        let len = durations.len();
        if len == 0 {
            0
        } else if len % 2 == 0 {
            (durations[len / 2 - 1] + durations[len / 2]) / 2
        } else {
            durations[len / 2]
        }
    }
}

#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub name: String,
    pub iterations: usize,
    pub successes: usize,
    pub failures: usize,
    pub min_ms: u128,
    pub max_ms: u128,
    pub avg_ms: u128,
    pub median_ms: u128,
}

impl BenchmarkResult {
    pub fn success_rate(&self) -> f64 {
        self.successes as f64 / self.iterations as f64
    }

    pub fn print_summary(&self) {
        println!("Benchmark: {}", self.name);
        println!("  Iterations: {}", self.iterations);
        println!("  Success Rate: {:.2}%", self.success_rate() * 100.0);
        println!(
            "  Duration - Min: {}ms, Max: {}ms, Avg: {}ms, Median: {}ms",
            self.min_ms, self.max_ms, self.avg_ms, self.median_ms
        );
    }
}
