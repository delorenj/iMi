"""Configuration management for iMi MCP server."""

from pathlib import Path
from typing import Optional

from pydantic import Field
from pydantic_settings import BaseSettings, SettingsConfigDict


class MCPConfig(BaseSettings):
    """Configuration for iMi MCP server.

    Loads from environment variables (IMI_MCP_*) and optional TOML file.
    """

    model_config = SettingsConfigDict(
        env_prefix="IMI_MCP_",
        env_file=".env",
        env_file_encoding="utf-8",
        extra="ignore",
    )

    # iMi CLI configuration
    imi_binary_path: str = Field(
        default="imi",
        description="Path to iMi CLI binary (default: searches PATH)",
    )

    timeout_seconds: int = Field(
        default=30,
        description="Timeout for iMi CLI commands in seconds",
        ge=1,
        le=600,
    )

    # MCP server configuration
    log_level: str = Field(
        default="INFO",
        description="Logging level (DEBUG, INFO, WARNING, ERROR)",
    )

    # Optional TOML config file path
    config_file: Optional[Path] = Field(
        default=None,
        description="Path to TOML config file (overrides env vars)",
    )

    @classmethod
    def load(cls, config_file: Optional[Path] = None) -> "MCPConfig":
        """Load configuration from environment and optional TOML file.

        Args:
            config_file: Optional path to TOML config file

        Returns:
            Configured MCPConfig instance
        """
        if config_file and config_file.exists():
            # TODO: Load TOML config when needed
            # For now, just use environment variables
            pass

        return cls()
