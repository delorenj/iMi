"""Entry point for iMi MCP server."""

import sys
from pathlib import Path

from .config import MCPConfig
from .server import serve


def main() -> None:
    """Main entry point for imi-mcp command."""
    try:
        # Load configuration from environment and optional file
        config_file = Path.home() / ".config" / "iMi" / "mcp-server.toml"
        config = MCPConfig.load(config_file if config_file.exists() else None)

        # Start server
        serve(config)

    except KeyboardInterrupt:
        print("\niMi MCP server stopped by user", file=sys.stderr)
        sys.exit(0)

    except Exception as e:
        print(f"Fatal error: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
