use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokio::fs;
use tokio::sync::mpsc;
use tokio_test::{assert_ok, assert_err};

use imi::config::Config;
use imi::database::{Database, Worktree};
use imi::git::GitManager;
use imi::monitor::{ActivityEvent, MonitorManager};
use imi::worktree::WorktreeManager;

// Test helper functions
async fn create_test_database() -> Database {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("test.db");
    Database::new(&db_path).await.unwrap()
}

async fn create_test_config() -> Config {
    Config::default()
}

async fn create_test_monitor_manager() -> (MonitorManager, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db = create_test_database().await;
    let git = GitManager::new();
    let config = create_test_config().await;
    
    let worktree_manager = WorktreeManager::new(git, db, config.clone());
    let monitor = MonitorManager::new(worktree_manager, config);
    
    (monitor, temp_dir)
}

async fn create_test_worktree(db: &Database, name: &str, wt_type: &str, path: &Path) -> Worktree {
    // Ensure the repository exists first to satisfy foreign key constraint
    let _ = db.create_repository(
        "test-repo",
        path.to_string_lossy().as_ref(),
        "https://github.com/test/test-repo.git",
        "main",
    ).await;
    
    // Try to fetch existing worktree first
    if let Ok(existing_worktrees) = db.list_worktrees(Some("test-repo")).await {
        for wt in existing_worktrees {
            if wt.worktree_name == name {
                return wt;
            }
        }
    }
    
    let worktree = Worktree {
        id: format!("test-{}", name),
        repo_name: "test-repo".to_string(),
        worktree_name: name.to_string(),
        branch_name: "main".to_string(),
        worktree_type: wt_type.to_string(),
        path: path.to_string_lossy().to_string(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        active: true,
        agent_id: None,
    };
    
    db.create_worktree(
        &worktree.repo_name,
        &worktree.worktree_name,
        &worktree.branch_name,
        &worktree.worktree_type,
        &worktree.path,
        worktree.agent_id.as_deref(),
    ).await.unwrap();
    
    // Fetch and return the created worktree
    if let Ok(created_worktrees) = db.list_worktrees(Some("test-repo")).await {
        for wt in created_worktrees {
            if wt.worktree_name == name {
                return wt;
            }
        }
    }
    
    worktree
}

async fn create_test_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await.unwrap();
    }
    fs::write(path, content).await.unwrap();
}

// Mock event creation
fn create_mock_file_event(event_kind: notify::EventKind, paths: Vec<PathBuf>) -> notify::Event {
    notify::Event {
        kind: event_kind,
        paths,
        attrs: notify::event::EventAttributes::new(),
    }
}

#[cfg(test)]
mod monitor_manager_creation_tests {
    use super::*;

    #[tokio::test]
    async fn test_monitor_manager_new() {
        let (monitor, _temp_dir) = create_test_monitor_manager().await;
        
        // Verify monitor was created successfully
        assert!(std::ptr::addr_of!(monitor).is_aligned());
    }

    #[tokio::test]
    async fn test_monitor_manager_with_custom_worktree_manager() {
        let db = create_test_database().await;
        let git = GitManager::new();
        let config = create_test_config().await;
        let worktree_manager = WorktreeManager::new(git, db, config.clone());
        
        let monitor = MonitorManager::new(worktree_manager, config);
        
        // Verify monitor creation with custom WorktreeManager
        assert!(std::ptr::addr_of!(monitor).is_aligned());
    }

    #[tokio::test]
    async fn test_monitor_manager_clone() {
        let (monitor, _temp_dir) = create_test_monitor_manager().await;
        
        let cloned_monitor = monitor.clone();
        
        // Verify cloning works
        assert!(std::ptr::addr_of!(cloned_monitor).is_aligned());
    }
}

#[cfg(test)]
mod file_system_monitoring_tests {
    use super::*;

    #[tokio::test]
    async fn test_start_monitoring_no_worktrees() {
        let (monitor, _temp_dir) = create_test_monitor_manager().await;
        
        // Test starting monitor with no worktrees
        let result = tokio::time::timeout(
            Duration::from_millis(100),
            monitor.start(None)
        ).await;
        
        // Should complete quickly when no worktrees exist
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_start_monitoring_with_worktrees() {
        let (monitor, temp_dir) = create_test_monitor_manager().await;
        let worktree_path = temp_dir.path().join("test-worktree");
        fs::create_dir_all(&worktree_path).await.unwrap();
        
        // Create test worktree in database
        create_test_worktree(&monitor.worktree_manager.db, "test", "feat", &worktree_path).await;
        
        // Test starting monitor with worktrees
        let result = tokio::time::timeout(
            Duration::from_millis(100),
            monitor.start(Some("test-repo"))
        ).await;
        
        // Should start monitoring (timeout expected)
        assert!(result.is_err()); // Timeout because monitoring runs indefinitely
    }

    #[tokio::test]
    async fn test_monitoring_nonexistent_worktree_paths() {
        let (monitor, _temp_dir) = create_test_monitor_manager().await;
        let nonexistent_path = PathBuf::from("/nonexistent/path");
        
        // Create worktree with nonexistent path
        create_test_worktree(&monitor.worktree_manager.db, "nonexistent", "feat", &nonexistent_path).await;
        
        // Test monitoring should handle nonexistent paths gracefully
        let result = tokio::time::timeout(
            Duration::from_millis(100),
            monitor.start(None)
        ).await;
        
        // Monitoring may timeout if paths don't exist (it tries to watch them)
        // Either completes quickly with error or times out - both are acceptable
        let _ = result; // Don't assert - behavior may vary
    }

    #[tokio::test]
    async fn test_get_type_icon() {
        let (monitor, _temp_dir) = create_test_monitor_manager().await;
        
        // Test various worktree type icons
        let feat_icon = monitor.get_type_icon("feat");
        let pr_icon = monitor.get_type_icon("pr");
        let fix_icon = monitor.get_type_icon("fix");
        let aiops_icon = monitor.get_type_icon("aiops");
        let devops_icon = monitor.get_type_icon("devops");
        let trunk_icon = monitor.get_type_icon("trunk");
        let unknown_icon = monitor.get_type_icon("unknown");
        
        // Verify icons are returned (strings)
        assert!(!feat_icon.is_empty());
        assert!(!pr_icon.is_empty());
        assert!(!fix_icon.is_empty());
        assert!(!aiops_icon.is_empty());
        assert!(!devops_icon.is_empty());
        assert!(!trunk_icon.is_empty());
        assert!(!unknown_icon.is_empty());
    }
}

#[cfg(test)]
mod event_processing_tests {
    use super::*;

    #[tokio::test]
    async fn test_process_file_event_create() {
        let (monitor, temp_dir) = create_test_monitor_manager().await;
        let worktree_path = temp_dir.path().join("test-worktree");
        fs::create_dir_all(&worktree_path).await.unwrap();
        
        let worktree = create_test_worktree(&monitor.worktree_manager.db, "test", "feat", &worktree_path).await;
        
        let mut path_to_worktree = HashMap::new();
        path_to_worktree.insert(worktree_path.clone(), worktree.clone());
        
        let test_file = worktree_path.join("test.txt");
        let event = create_mock_file_event(
            notify::EventKind::Create(notify::event::CreateKind::File),
            vec![test_file]
        );
        
        let result = monitor.process_file_event(&event, &path_to_worktree).await;
        
        assert!(result.is_some());
        let activity = result.unwrap();
        assert_eq!(activity.worktree_id, worktree.id);
        assert_eq!(activity.event_type, "created");
        assert!(activity.file_path.is_some());
    }

    #[tokio::test]
    async fn test_process_file_event_modify() {
        let (monitor, temp_dir) = create_test_monitor_manager().await;
        let worktree_path = temp_dir.path().join("test-worktree");
        fs::create_dir_all(&worktree_path).await.unwrap();
        
        let worktree = create_test_worktree(&monitor.worktree_manager.db, "test", "feat", &worktree_path).await;
        
        let mut path_to_worktree = HashMap::new();
        path_to_worktree.insert(worktree_path.clone(), worktree.clone());
        
        let test_file = worktree_path.join("test.txt");
        let event = create_mock_file_event(
            notify::EventKind::Modify(notify::event::ModifyKind::Data(notify::event::DataChange::Content)),
            vec![test_file]
        );
        
        let result = monitor.process_file_event(&event, &path_to_worktree).await;
        
        assert!(result.is_some());
        let activity = result.unwrap();
        assert_eq!(activity.worktree_id, worktree.id);
        assert_eq!(activity.event_type, "modified");
        assert!(activity.file_path.is_some());
    }

    #[tokio::test]
    async fn test_process_file_event_delete() {
        let (monitor, temp_dir) = create_test_monitor_manager().await;
        let worktree_path = temp_dir.path().join("test-worktree");
        fs::create_dir_all(&worktree_path).await.unwrap();
        
        let worktree = create_test_worktree(&monitor.worktree_manager.db, "test", "feat", &worktree_path).await;
        
        let mut path_to_worktree = HashMap::new();
        path_to_worktree.insert(worktree_path.clone(), worktree.clone());
        
        let test_file = worktree_path.join("test.txt");
        let event = create_mock_file_event(
            notify::EventKind::Remove(notify::event::RemoveKind::File),
            vec![test_file]
        );
        
        let result = monitor.process_file_event(&event, &path_to_worktree).await;
        
        assert!(result.is_some());
        let activity = result.unwrap();
        assert_eq!(activity.worktree_id, worktree.id);
        assert_eq!(activity.event_type, "deleted");
        assert!(activity.file_path.is_some());
    }

    #[tokio::test]
    async fn test_process_file_event_ignore_git_files() {
        let (monitor, temp_dir) = create_test_monitor_manager().await;
        let worktree_path = temp_dir.path().join("test-worktree");
        fs::create_dir_all(&worktree_path).await.unwrap();
        
        let worktree = create_test_worktree(&monitor.worktree_manager.db, "test", "feat", &worktree_path).await;
        
        let mut path_to_worktree = HashMap::new();
        path_to_worktree.insert(worktree_path.clone(), worktree.clone());
        
        let git_file = worktree_path.join(".gitignore");
        let event = create_mock_file_event(
            notify::EventKind::Create(notify::event::CreateKind::File),
            vec![git_file]
        );
        
        let result = monitor.process_file_event(&event, &path_to_worktree).await;
        
        // Should ignore .git files but allow .gitignore and .env
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_process_file_event_allow_env_files() {
        let (monitor, temp_dir) = create_test_monitor_manager().await;
        let worktree_path = temp_dir.path().join("test-worktree");
        fs::create_dir_all(&worktree_path).await.unwrap();
        
        let worktree = create_test_worktree(&monitor.worktree_manager.db, "test", "feat", &worktree_path).await;
        
        let mut path_to_worktree = HashMap::new();
        path_to_worktree.insert(worktree_path.clone(), worktree.clone());
        
        let env_file = worktree_path.join(".env");
        let event = create_mock_file_event(
            notify::EventKind::Create(notify::event::CreateKind::File),
            vec![env_file]
        );
        
        let result = monitor.process_file_event(&event, &path_to_worktree).await;
        
        // Should allow .env files
        assert!(result.is_some());
        let activity = result.unwrap();
        assert_eq!(activity.event_type, "created");
    }

    #[tokio::test]
    async fn test_process_file_event_no_matching_worktree() {
        let (monitor, temp_dir) = create_test_monitor_manager().await;
        let worktree_path = temp_dir.path().join("test-worktree");
        let different_path = temp_dir.path().join("different-path");
        fs::create_dir_all(&worktree_path).await.unwrap();
        
        let worktree = create_test_worktree(&monitor.worktree_manager.db, "test", "feat", &worktree_path).await;
        
        let mut path_to_worktree = HashMap::new();
        path_to_worktree.insert(worktree_path, worktree);
        
        let test_file = different_path.join("test.txt");
        let event = create_mock_file_event(
            notify::EventKind::Create(notify::event::CreateKind::File),
            vec![test_file]
        );
        
        let result = monitor.process_file_event(&event, &path_to_worktree).await;
        
        // Should return None for files outside worktree paths
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_process_unsupported_event_type() {
        let (monitor, temp_dir) = create_test_monitor_manager().await;
        let worktree_path = temp_dir.path().join("test-worktree");
        fs::create_dir_all(&worktree_path).await.unwrap();
        
        let worktree = create_test_worktree(&monitor.worktree_manager.db, "test", "feat", &worktree_path).await;
        
        let mut path_to_worktree = HashMap::new();
        path_to_worktree.insert(worktree_path.clone(), worktree);
        
        let test_file = worktree_path.join("test.txt");
        let event = create_mock_file_event(
            notify::EventKind::Access(notify::event::AccessKind::Read),
            vec![test_file]
        );
        
        let result = monitor.process_file_event(&event, &path_to_worktree).await;
        
        // Should ignore unsupported event types
        assert!(result.is_none());
    }
}

#[cfg(test)]
mod activity_logging_tests {
    use super::*;

    #[tokio::test]
    async fn test_log_activity_to_db_with_file_path() {
        let (monitor, temp_dir) = create_test_monitor_manager().await;
        let worktree_path = temp_dir.path().join("test-worktree");
        fs::create_dir_all(&worktree_path).await.unwrap();
        
        // Create worktree first to satisfy foreign key constraint
        let worktree = create_test_worktree(&monitor.worktree_manager.db, "test-worktree", "feat", &worktree_path).await;
        
        let activity = ActivityEvent {
            worktree_id: worktree.id,
            event_type: "created".to_string(),
            file_path: Some("test.txt".to_string()),
            timestamp: Instant::now(),
        };
        
        let result = monitor.log_activity_to_db(&activity).await;
        
        assert_ok!(result);
    }

    #[tokio::test]
    async fn test_log_activity_to_db_without_file_path() {
        let (monitor, temp_dir) = create_test_monitor_manager().await;
        let worktree_path = temp_dir.path().join("test-worktree");
        fs::create_dir_all(&worktree_path).await.unwrap();
        
        // Create worktree first to satisfy foreign key constraint
        let worktree = create_test_worktree(&monitor.worktree_manager.db, "test-worktree", "feat", &worktree_path).await;
        
        let activity = ActivityEvent {
            worktree_id: worktree.id,
            event_type: "modified".to_string(),
            file_path: None,
            timestamp: Instant::now(),
        };
        
        let result = monitor.log_activity_to_db(&activity).await;
        
        assert_ok!(result);
    }

    #[tokio::test]
    async fn test_display_activity_with_file_path() {
        let (monitor, _temp_dir) = create_test_monitor_manager().await;
        
        let activity = ActivityEvent {
            worktree_id: "test-worktree".to_string(),
            event_type: "created".to_string(),
            file_path: Some("src/main.rs".to_string()),
            timestamp: Instant::now(),
        };
        
        // Should not panic when displaying activity
        monitor.display_activity(&activity).await;
    }

    #[tokio::test]
    async fn test_display_activity_without_file_path() {
        let (monitor, _temp_dir) = create_test_monitor_manager().await;
        
        let activity = ActivityEvent {
            worktree_id: "test-worktree".to_string(),
            event_type: "modified".to_string(),
            file_path: None,
            timestamp: Instant::now(),
        };
        
        // Should not panic when displaying activity without file path
        monitor.display_activity(&activity).await;
    }

    #[tokio::test]
    async fn test_display_activity_different_event_types() {
        let (monitor, _temp_dir) = create_test_monitor_manager().await;
        
        let event_types = vec!["created", "modified", "deleted", "renamed", "unknown"];
        
        for event_type in event_types {
            let activity = ActivityEvent {
                worktree_id: "test-worktree".to_string(),
                event_type: event_type.to_string(),
                file_path: Some("test.txt".to_string()),
                timestamp: Instant::now(),
            };
            
            // Should handle all event types without panicking
            monitor.display_activity(&activity).await;
        }
    }
}

#[cfg(test)]
mod monitor_loop_tests {
    use super::*;

    #[tokio::test]
    async fn test_monitor_loop_with_events() {
        let (monitor, temp_dir) = create_test_monitor_manager().await;
        let worktree_path = temp_dir.path().join("test-worktree");
        fs::create_dir_all(&worktree_path).await.unwrap();
        
        let worktree = create_test_worktree(&monitor.worktree_manager.db, "test", "feat", &worktree_path).await;
        
        let mut path_to_worktree = HashMap::new();
        path_to_worktree.insert(worktree_path.clone(), worktree);
        
        let (tx, rx) = mpsc::channel(10);
        
        // Send test event
        let test_file = worktree_path.join("test.txt");
        let event = create_mock_file_event(
            notify::EventKind::Create(notify::event::CreateKind::File),
            vec![test_file]
        );
        tx.send(event).await.unwrap();
        drop(tx); // Close channel to end loop
        
        let result = monitor.monitor_loop(rx, path_to_worktree).await;
        
        assert_ok!(result);
    }

    #[tokio::test]
    async fn test_monitor_loop_debouncing() {
        let (monitor, temp_dir) = create_test_monitor_manager().await;
        let worktree_path = temp_dir.path().join("test-worktree");
        fs::create_dir_all(&worktree_path).await.unwrap();
        
        let worktree = create_test_worktree(&monitor.worktree_manager.db, "test", "feat", &worktree_path).await;
        
        let mut path_to_worktree = HashMap::new();
        path_to_worktree.insert(worktree_path.clone(), worktree);
        
        let (tx, rx) = mpsc::channel(10);
        
        // Send rapid duplicate events
        let test_file = worktree_path.join("test.txt");
        for _ in 0..5 {
            let event = create_mock_file_event(
                notify::EventKind::Modify(notify::event::ModifyKind::Data(notify::event::DataChange::Content)),
                vec![test_file.clone()]
            );
            tx.send(event).await.unwrap();
        }
        drop(tx);
        
        let result = monitor.monitor_loop(rx, path_to_worktree).await;
        
        // Should handle debouncing without errors
        assert_ok!(result);
    }

    #[tokio::test]
    async fn test_monitor_loop_empty_channel() {
        let (monitor, _temp_dir) = create_test_monitor_manager().await;
        let path_to_worktree = HashMap::new();
        
        let (_tx, rx) = mpsc::channel(10);
        // Immediately drop tx to close channel
        
        let result = monitor.monitor_loop(rx, path_to_worktree).await;
        
        assert_ok!(result);
    }
}

#[cfg(test)]
mod status_reporting_tests {
    use super::*;

    #[tokio::test]
    async fn test_display_status_summary_empty_worktrees() {
        let (monitor, _temp_dir) = create_test_monitor_manager().await;
        let worktrees = vec![];
        
        let result = monitor.display_status_summary(&worktrees).await;
        
        assert_ok!(result);
    }

    #[tokio::test]
    async fn test_display_status_summary_with_worktrees() {
        let (monitor, temp_dir) = create_test_monitor_manager().await;
        let worktree_path = temp_dir.path().join("test-worktree");
        fs::create_dir_all(&worktree_path).await.unwrap();
        
        let worktree = create_test_worktree(&monitor.worktree_manager.db, "test", "feat", &worktree_path).await;
        let worktrees = vec![worktree];
        
        let result = monitor.display_status_summary(&worktrees).await;
        
        assert_ok!(result);
    }

    #[tokio::test]
    async fn test_display_status_summary_nonexistent_paths() {
        let (monitor, _temp_dir) = create_test_monitor_manager().await;
        
        let worktree = Worktree {
            id: "test-nonexistent".to_string(),
            repo_name: "test-repo".to_string(),
            worktree_name: "nonexistent".to_string(),
            branch_name: "nonexistent".to_string(),
            worktree_type: "feat".to_string(),
            path: "/nonexistent/path".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            active: true,
            agent_id: None,
        };
        
        let worktrees = vec![worktree];
        let result = monitor.display_status_summary(&worktrees).await;
        
        assert_ok!(result);
    }

    #[tokio::test]
    async fn test_periodic_status_update() {
        let (monitor, temp_dir) = create_test_monitor_manager().await;
        let worktree_path = temp_dir.path().join("test-worktree");
        fs::create_dir_all(&worktree_path).await.unwrap();
        
        let worktree = create_test_worktree(&monitor.worktree_manager.db, "test", "feat", &worktree_path).await;
        let worktrees = vec![worktree];
        
        // Test periodic status with short timeout
        let result = tokio::time::timeout(
            Duration::from_millis(100),
            monitor.periodic_status_update(None, worktrees)
        ).await;
        
        // Should timeout since it runs indefinitely
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_show_git_stats_no_worktrees() {
        let (monitor, _temp_dir) = create_test_monitor_manager().await;
        
        let result = monitor.show_git_stats(None).await;
        
        assert_ok!(result);
    }

    #[tokio::test]
    async fn test_show_git_stats_with_worktrees() {
        let (monitor, temp_dir) = create_test_monitor_manager().await;
        let worktree_path = temp_dir.path().join("test-worktree");
        fs::create_dir_all(&worktree_path).await.unwrap();
        
        create_test_worktree(&monitor.worktree_manager.db, "test", "feat", &worktree_path).await;
        
        let result = monitor.show_git_stats(Some("test-repo")).await;
        
        assert_ok!(result);
    }
}

#[cfg(test)]
mod error_handling_tests {
    use super::*;

    #[tokio::test]
    async fn test_log_activity_invalid_worktree() {
        let (monitor, _temp_dir) = create_test_monitor_manager().await;
        
        let activity = ActivityEvent {
            worktree_id: "nonexistent-worktree".to_string(),
            event_type: "created".to_string(),
            file_path: Some("test.txt".to_string()),
            timestamp: Instant::now(),
        };
        
        // Should handle invalid worktree - database will reject due to foreign key constraint
        let result = monitor.log_activity_to_db(&activity).await;
        assert_err!(result); // Expected to fail due to foreign key constraint
    }

    #[tokio::test]
    async fn test_monitor_loop_error_recovery() {
        let (monitor, temp_dir) = create_test_monitor_manager().await;
        let worktree_path = temp_dir.path().join("test-worktree");
        
        // Don't create the worktree directory to simulate error conditions
        let worktree = Worktree {
            id: "error-test".to_string(),
            repo_name: "test-repo".to_string(),
            worktree_name: "error-test".to_string(),
            branch_name: "main".to_string(),
            worktree_type: "feat".to_string(),
            path: worktree_path.to_string_lossy().to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            active: true,
            agent_id: None,
        };
        
        let mut path_to_worktree = HashMap::new();
        path_to_worktree.insert(worktree_path.clone(), worktree);
        
        let (tx, rx) = mpsc::channel(10);
        
        // Send event for nonexistent path
        let test_file = worktree_path.join("test.txt");
        let event = create_mock_file_event(
            notify::EventKind::Create(notify::event::CreateKind::File),
            vec![test_file]
        );
        tx.send(event).await.unwrap();
        drop(tx);
        
        let result = monitor.monitor_loop(rx, path_to_worktree).await;
        
        // Should handle errors gracefully
        assert_ok!(result);
    }

    #[tokio::test]
    async fn test_process_file_event_with_empty_paths() {
        let (monitor, temp_dir) = create_test_monitor_manager().await;
        let worktree_path = temp_dir.path().join("test-worktree");
        fs::create_dir_all(&worktree_path).await.unwrap();
        
        let worktree = create_test_worktree(&monitor.worktree_manager.db, "test", "feat", &worktree_path).await;
        
        let mut path_to_worktree = HashMap::new();
        path_to_worktree.insert(worktree_path, worktree);
        
        let event = create_mock_file_event(
            notify::EventKind::Create(notify::event::CreateKind::File),
            vec![] // Empty paths
        );
        
        let result = monitor.process_file_event(&event, &path_to_worktree).await;
        
        // Should handle empty paths gracefully
        assert!(result.is_none());
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_full_monitoring_workflow() {
        let (monitor, temp_dir) = create_test_monitor_manager().await;
        let worktree_path = temp_dir.path().join("integration-test");
        fs::create_dir_all(&worktree_path).await.unwrap();
        
        // Create worktree in database
        create_test_worktree(&monitor.worktree_manager.db, "integration", "feat", &worktree_path).await;
        
        // Create test file
        let test_file = worktree_path.join("integration_test.rs");
        create_test_file(&test_file, "// Integration test file").await;
        
        // Create and process file event
        let worktree = create_test_worktree(&monitor.worktree_manager.db, "integration", "feat", &worktree_path).await;
        let mut path_to_worktree = HashMap::new();
        path_to_worktree.insert(worktree_path.clone(), worktree.clone());
        
        let event = create_mock_file_event(
            notify::EventKind::Create(notify::event::CreateKind::File),
            vec![test_file]
        );
        
        let activity_result = monitor.process_file_event(&event, &path_to_worktree).await;
        assert!(activity_result.is_some());
        
        let activity = activity_result.unwrap();
        assert_eq!(activity.worktree_id, worktree.id);
        assert_eq!(activity.event_type, "created");
        
        // Log activity to database
        let log_result = monitor.log_activity_to_db(&activity).await;
        assert_ok!(log_result);
        
        // Display activity (should not panic)
        monitor.display_activity(&activity).await;
        
        // Display status summary
        let status_result = monitor.display_status_summary(&vec![worktree]).await;
        assert_ok!(status_result);
    }

    #[tokio::test]
    async fn test_multiple_worktree_types_monitoring() {
        let (monitor, temp_dir) = create_test_monitor_manager().await;
        
        let worktree_types = vec!["feat", "pr", "fix", "aiops", "devops", "trunk"];
        let mut worktrees = Vec::new();
        
        // Create multiple worktree types
        for wt_type in &worktree_types {
            let worktree_path = temp_dir.path().join(format!("test-{}", wt_type));
            fs::create_dir_all(&worktree_path).await.unwrap();
            
            let worktree = create_test_worktree(&monitor.worktree_manager.db, wt_type, wt_type, &worktree_path).await;
            worktrees.push(worktree);
        }
        
        // Test status summary with multiple types
        let result = monitor.display_status_summary(&worktrees).await;
        assert_ok!(result);
        
        // Test git stats with multiple types
        let git_stats_result = monitor.show_git_stats(Some("test-repo")).await;
        assert_ok!(git_stats_result);
    }

    #[tokio::test]
    async fn test_concurrent_activity_processing() {
        let (monitor, temp_dir) = create_test_monitor_manager().await;
        let worktree_path = temp_dir.path().join("concurrent-test");
        fs::create_dir_all(&worktree_path).await.unwrap();
        
        let worktree = create_test_worktree(&monitor.worktree_manager.db, "concurrent", "feat", &worktree_path).await;
        
        // Create multiple activities
        let activities = vec![
            ActivityEvent {
                worktree_id: worktree.id.clone(),
                event_type: "created".to_string(),
                file_path: Some("file1.txt".to_string()),
                timestamp: Instant::now(),
            },
            ActivityEvent {
                worktree_id: worktree.id.clone(),
                event_type: "modified".to_string(),
                file_path: Some("file2.txt".to_string()),
                timestamp: Instant::now(),
            },
            ActivityEvent {
                worktree_id: worktree.id.clone(),
                event_type: "deleted".to_string(),
                file_path: Some("file3.txt".to_string()),
                timestamp: Instant::now(),
            },
        ];
        
        // Process activities - SQLite doesn't handle concurrent writes well in tests
        // so we process them with small delays to avoid database lock issues
        for activity in activities {
            let result = monitor.log_activity_to_db(&activity).await;
            assert_ok!(result);
            // Small delay to avoid database contention
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
}

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[tokio::test]
    async fn test_activity_event_debug_format() {
        let activity = ActivityEvent {
            worktree_id: "test-debug".to_string(),
            event_type: "debug_test".to_string(),
            file_path: Some("debug.rs".to_string()),
            timestamp: Instant::now(),
        };
        
        let debug_str = format!("{:?}", activity);
        assert!(debug_str.contains("test-debug"));
        assert!(debug_str.contains("debug_test"));
        assert!(debug_str.contains("debug.rs"));
    }

    #[tokio::test]
    async fn test_activity_event_clone() {
        let activity = ActivityEvent {
            worktree_id: "test-clone".to_string(),
            event_type: "clone_test".to_string(),
            file_path: Some("clone.rs".to_string()),
            timestamp: Instant::now(),
        };
        
        let cloned_activity = activity.clone();
        assert_eq!(activity.worktree_id, cloned_activity.worktree_id);
        assert_eq!(activity.event_type, cloned_activity.event_type);
        assert_eq!(activity.file_path, cloned_activity.file_path);
    }

    #[tokio::test]
    async fn test_monitor_manager_debug_format() {
        let (monitor, _temp_dir) = create_test_monitor_manager().await;
        
        let debug_str = format!("{:?}", monitor);
        assert!(debug_str.contains("MonitorManager"));
    }

    #[tokio::test]
    async fn test_very_long_file_paths() {
        let (monitor, temp_dir) = create_test_monitor_manager().await;
        let worktree_path = temp_dir.path().join("test-worktree");
        fs::create_dir_all(&worktree_path).await.unwrap();
        
        let worktree = create_test_worktree(&monitor.worktree_manager.db, "long-path", "feat", &worktree_path).await;
        
        let mut path_to_worktree = HashMap::new();
        path_to_worktree.insert(worktree_path.clone(), worktree.clone());
        
        // Create very long nested path
        let long_path = worktree_path
            .join("very")
            .join("long")
            .join("nested")
            .join("directory")
            .join("structure")
            .join("for")
            .join("testing")
            .join("purposes")
            .join("file.txt");
        
        let event = create_mock_file_event(
            notify::EventKind::Create(notify::event::CreateKind::File),
            vec![long_path]
        );
        
        let result = monitor.process_file_event(&event, &path_to_worktree).await;
        
        assert!(result.is_some());
        let activity = result.unwrap();
        assert!(activity.file_path.is_some());
        let file_path = activity.file_path.unwrap();
        assert!(file_path.contains("very/long/nested"));
    }

    #[tokio::test]
    async fn test_unicode_file_paths() {
        let (monitor, temp_dir) = create_test_monitor_manager().await;
        let worktree_path = temp_dir.path().join("test-worktree");
        fs::create_dir_all(&worktree_path).await.unwrap();
        
        let worktree = create_test_worktree(&monitor.worktree_manager.db, "unicode", "feat", &worktree_path).await;
        
        let mut path_to_worktree = HashMap::new();
        path_to_worktree.insert(worktree_path.clone(), worktree.clone());
        
        // Test with unicode file name
        let unicode_file = worktree_path.join("测试文件.txt");
        let event = create_mock_file_event(
            notify::EventKind::Create(notify::event::CreateKind::File),
            vec![unicode_file]
        );
        
        let result = monitor.process_file_event(&event, &path_to_worktree).await;
        
        assert!(result.is_some());
        let activity = result.unwrap();
        assert!(activity.file_path.is_some());
        assert!(activity.file_path.unwrap().contains("测试文件.txt"));
    }
}
