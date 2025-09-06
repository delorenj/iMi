use anyhow::{Context, Result};
use std::env;
use tempfile::TempDir;
use tokio::fs;
use git2::Repository;
use serial_test::serial;

use imi::config::Config;
use imi::database::Database;
use imi::init::InitCommand;

struct TestContext {
    temp_dir: TempDir,
    config: Config,
}

async fn setup_test_env() -> Result<TestContext> {
    let temp_dir = TempDir::new().context("Failed to create temp directory")?;
    env::set_var("HOME", temp_dir.path());
    env::set_current_dir(temp_dir.path())?;

    let mut config = Config::default();
    config.root_path = temp_dir.path().join("code");
    config.database_path = temp_dir.path().join("imi.db");
    config.save().await?;

    Ok(TestContext { temp_dir, config })
}

async fn setup_repo_env() -> Result<TestContext> {
    let ctx = setup_test_env().await?;
    let repo_path = ctx.temp_dir.path().join("my-repo");
    let trunk_path = repo_path.join("trunk-main");
    fs::create_dir_all(&trunk_path).await?;
    env::set_current_dir(&trunk_path)?;

    Repository::init(&trunk_path)?;

    Ok(ctx)
}


#[cfg(test)]
mod init_rules_tests {
    use super::*;

    #[tokio::test]
    #[serial]
    async fn test_init_creates_default_config_outside_repo() -> Result<()> {
        let _ctx = setup_test_env().await?;
        let config_path = Config::get_config_path()?;
        fs::remove_file(&config_path).await.ok();
        assert!(!config_path.exists(), "Config file should not exist initially");

        let init_cmd = InitCommand::new(false);
        init_cmd.execute().await?;

        assert!(config_path.exists(), "Config file should be created");
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_init_updates_config_with_force_flag() -> Result<()> {
        let _ctx = setup_test_env().await?;
        let config_path = Config::get_config_path()?;
        let mut config = Config::load().await?;
        let initial_content = fs::read_to_string(&config_path).await?;

        config.git_settings.default_branch = "develop".to_string();
        config.save().await?;

        let init_cmd = InitCommand::new(true);
        init_cmd.execute().await?;

        let updated_content = fs::read_to_string(&config_path).await?;
        assert_ne!(initial_content, updated_content, "Config file should be updated");
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_init_creates_database_outside_repo() -> Result<()> {
        let _ctx = setup_test_env().await?;
        let config = Config::load().await?;
        let db_path = &config.database_path;
        fs::remove_file(&db_path).await.ok();

        assert!(!db_path.exists(), "Database file should not exist initially");

        let init_cmd = InitCommand::new(false);
        init_cmd.execute().await?;
        
        assert!(db_path.exists(), "Database file should be created");
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_init_updates_database_with_force_flag() -> Result<()> {
        let _ctx = setup_test_env().await?;
        let config = Config::load().await?;
        let db_path = &config.database_path;
        let db = Database::new(db_path).await?;
        db.ensure_tables().await?;
        
        let init_cmd = InitCommand::new(true);
        init_cmd.execute().await?;

        assert!(db_path.exists(), "Database file should exist");
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_init_fails_in_invalid_repo_structure() -> Result<()> {
        let ctx = setup_test_env().await?;
        let invalid_repo_path = ctx.temp_dir.path().join("invalid-repo");
        fs::create_dir_all(&invalid_repo_path).await?;
        env::set_current_dir(&invalid_repo_path)?;
        Repository::init(&invalid_repo_path)?;

        let init_cmd = InitCommand::new(false);
        let result = init_cmd.execute().await;

        assert!(result.is_err(), "Init should fail in a directory not starting with trunk-");
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_init_registers_repository_in_database() -> Result<()> {
        let _ctx = setup_repo_env().await?;
        let config = Config::load().await?;
        let db = Database::new(&config.database_path).await?;

        let init_cmd = InitCommand::new(false);
        init_cmd.execute().await?;

        let repo = db.get_repository("my-repo").await?;
        assert!(repo.is_some(), "Repository should be registered in the database");
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_init_fails_if_repo_already_registered() -> Result<()> {
        let _ctx = setup_repo_env().await?;
        let init_cmd = InitCommand::new(false);
        init_cmd.execute().await?;

        // run again
        let result = init_cmd.execute().await;
        assert!(!result.unwrap().success, "Init should fail if repo is already registered");

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_init_registers_imi_path() -> Result<()> {
        let ctx = setup_repo_env().await?;
        let config = Config::load().await?;
        let db = Database::new(&config.database_path).await?;

        let init_cmd = InitCommand::new(false);
        init_cmd.execute().await?;

        let repo = db.get_repository("my-repo").await?.unwrap();
        let expected_imi_path = ctx.temp_dir.path().join("my-repo");
        assert_eq!(repo.path, expected_imi_path.to_str().unwrap());
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_init_creates_imi_dir_in_imi_path() -> Result<()> {
        let _ctx = setup_repo_env().await?;
        let init_cmd = InitCommand::new(false);
        init_cmd.execute().await?;

        let imi_dir = env::current_dir()?.parent().unwrap().join(".iMi");
        assert!(imi_dir.exists(), ".iMi directory should be created in the iMi path");
        Ok(())
    }
}