"""FastMCP server initialization and lifecycle management."""

import logging

from mcp.server.fastmcp import FastMCP

from .config import MCPConfig
from .tools import register_tools, set_config

# Initialize logger
logger = logging.getLogger(__name__)


def create_server(config: MCPConfig) -> FastMCP:
    """Create and configure the FastMCP server.

    Args:
        config: MCP configuration

    Returns:
        Configured FastMCP server instance
    """
    # Configure logging
    logging.basicConfig(
        level=getattr(logging, config.log_level.upper()),
        format="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
    )

    # Create FastMCP instance
    mcp = FastMCP(
        "iMi Worktree Manager",
        dependencies=["fastmcp>=0.2.0", "pydantic>=2.0", "pydantic-settings>=2.0"],
    )

    # Set global config for tools
    set_config(config)

    # Register all tools
    register_tools(mcp)

    logger.info(
        f"iMi MCP server initialized (iMi binary: {config.imi_binary_path}, "
        f"timeout: {config.timeout_seconds}s)"
    )

    return mcp


def serve(config: MCPConfig) -> None:
    """Start the MCP server with stdio transport.

    Args:
        config: MCP configuration
    """
    mcp = create_server(config)

    logger.info("Starting iMi MCP server on stdio transport...")

    # Run server (blocks until interrupted)
    mcp.run(transport="stdio")
