use anyhow::{Context, Result};

use serial_test::serial;
use std::env;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::fs;

use imi::config::Config;
use imi::database::Database;
use imi::init::InitCommand;

struct TestContext {
    temp_dir: TempDir,
    config: Config,
    db: Database,
}

async fn setup_test_env() -> Result<TestContext> {
    let temp_dir = TempDir::new().context("Failed to create temp directory")?;
    env::set_var("HOME", temp_dir.path());

    let config_path = Config::get_global_config_path()?;
    let db_path = temp_dir.path().join("imi.db");

    let mut config = Config::default();
    config.workspace_settings.root_path = temp_dir.path().join("code");
    config.database_path = db_path.clone();
    config.save_to(&config_path).await?;

    let db = Database::new(&db_path).await?;
    db.ensure_tables().await?;

    Ok(TestContext {
        temp_dir,
        config,
        db,
    })
}

async fn setup_repo_env() -> Result<(TestContext, PathBuf)> {
    let ctx = setup_test_env().await?;
    let repo_path = ctx.config.workspace_settings.root_path.join("my-repo");
    let trunk_path = repo_path.join("trunk-main");
    fs::create_dir_all(&trunk_path).await?;

    let output = std::process::Command::new("git")
        .args(["init"])
        .current_dir(&repo_path)
        .output()
        .context("Failed to initialize git repository")?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Git init failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    std::process::Command::new("git")
        .args([
            "remote",
            "add",
            "origin",
            "git@github.com:test/my-repo.git",
        ])
        .current_dir(&repo_path)
        .output()
        .context("Failed to add remote")?;

    Ok((ctx, trunk_path))
}

#[cfg(test)]
mod init_rules_tests {
    use super::*;

    #[tokio::test]
    #[serial]
    async fn test_init_creates_default_config_outside_repo() -> Result<()> {
        let ctx = setup_test_env().await?;
        let config_path = Config::get_global_config_path()?;
        fs::remove_file(&config_path).await.ok();
        assert!(
            !config_path.exists(),
            "Config file should not exist initially"
        );

        let init_cmd = InitCommand::new(false, ctx.config, ctx.db);
        init_cmd.execute(Some(ctx.temp_dir.path())).await?;

        assert!(config_path.exists(), "Config file should be created");
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_init_updates_config_with_force_flag() -> Result<()> {
        let ctx = setup_test_env().await?;
        let config_path = Config::get_global_config_path()?;
        let mut config = Config::load_from(&config_path).await?;

        config.git_settings.default_branch = "develop".to_string();
        config.save_to(&config_path).await?;

        let init_cmd = InitCommand::new(true, Config::default(), ctx.db);
        init_cmd.execute(Some(ctx.temp_dir.path())).await?;

        let updated_config = Config::load_from(&config_path).await?;
        assert_eq!(
            updated_config.git_settings.default_branch, "main",
            "Config file should be updated to default"
        );
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_init_creates_database_outside_repo() -> Result<()> {
        let ctx = setup_test_env().await?;
        let db_path = &ctx.config.database_path;
        let _ = fs::remove_file(&db_path).await;
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        assert!(
            !db_path.exists(),
            "Database file should not exist initially"
        );

        let db = Database::new(db_path).await?;
        let init_cmd = InitCommand::new(false, ctx.config.clone(), db);
        init_cmd.execute(Some(ctx.temp_dir.path())).await?;

        assert!(db_path.exists(), "Database file should be created");
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_init_updates_database_with_force_flag() -> Result<()> {
        let ctx = setup_test_env().await?;
        let db_path = &ctx.config.database_path;
        let db = Database::new(db_path).await?;
        db.ensure_tables().await?;

        let init_cmd = InitCommand::new(true, ctx.config.clone(), db);
        init_cmd.execute(Some(ctx.temp_dir.path())).await?;

        assert!(db_path.exists(), "Database file should exist");
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_init_succeeds_in_repo_root() -> Result<()> {
        let ctx = setup_test_env().await?;
        let repo_path = ctx.temp_dir.path().join("my-repo");
        fs::create_dir_all(&repo_path).await?;

        let output = std::process::Command::new("git")
            .args(["init"])
            .current_dir(&repo_path)
            .output()
            .context("Failed to initialize git repository")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Git init failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        std::process::Command::new("git")
            .args([
                "remote",
                "add",
                "origin",
                "git@github.com:test/my-repo.git",
            ])
            .current_dir(&repo_path)
            .output()
            .context("Failed to add remote")?;

        let init_cmd = InitCommand::new(false, ctx.config, ctx.db);
        let result = init_cmd.execute(Some(&repo_path)).await;

        assert!(
            result.is_ok(),
            "Init should succeed in a repo root directory"
        );
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_init_registers_repository_in_database() -> Result<()> {
        let (ctx, trunk_path): (TestContext, PathBuf) = setup_repo_env().await?;
        let db = ctx.db;

        let init_cmd = InitCommand::new(false, ctx.config, db.clone());
        init_cmd.execute(Some(&trunk_path)).await?;

        let repo = db.get_repository("my-repo").await?;
        assert!(
            repo.is_some(),
            "Repository should be registered in the database"
        );
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_init_fails_if_repo_already_registered() -> Result<()> {
        let (ctx, trunk_path): (TestContext, PathBuf) = setup_repo_env().await?;
        let init_cmd = InitCommand::new(false, ctx.config.clone(), ctx.db.clone());
        init_cmd.execute(Some(&trunk_path)).await?;

        // run again
        let result = init_cmd.execute(Some(&trunk_path)).await;
        assert!(
            !result.unwrap().success,
            "Init should fail if repo is already registered"
        );

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_init_registers_imi_path() -> Result<()> {
        let (ctx, trunk_path): (TestContext, PathBuf) = setup_repo_env().await?;
        let db = ctx.db;

        let init_cmd = InitCommand::new(false, ctx.config, db.clone());
        init_cmd.execute(Some(&trunk_path)).await?;

        let repo = db.get_repository("my-repo").await?.unwrap();
        let expected_imi_path = ctx.temp_dir.path().join("code").join("my-repo");
        assert_eq!(repo.path, expected_imi_path.to_str().unwrap());
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_init_creates_imi_dir_in_imi_path() -> Result<()> {
        let (ctx, trunk_path): (TestContext, PathBuf) = setup_repo_env().await?;
        let init_cmd = InitCommand::new(false, ctx.config, ctx.db);
        init_cmd.execute(Some(&trunk_path)).await?;

        let imi_dir = ctx
            .temp_dir
            .path()
            .join("code")
            .join("my-repo")
            .join(".iMi");
        assert!(
            imi_dir.exists(),
            ".iMi directory should be created in the iMi path"
        );
        Ok(())
    }
}
