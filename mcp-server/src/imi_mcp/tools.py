"""MCP tools for iMi worktree management."""

from typing import Optional

from mcp.server.fastmcp import FastMCP

from .cli_wrapper import run_imi_command
from .config import MCPConfig
from .schemas import (
    CreateWorktreeInput,
    ListWorktreesInput,
    NavigateInput,
    ProjectCreateInput,
    PruneInput,
    RemoveWorktreeInput,
    ReviewWorktreeInput,
    SyncInput,
    WorktreeResult,
)

# Global config instance (loaded on server startup)
_config: Optional[MCPConfig] = None


def set_config(config: MCPConfig) -> None:
    """Set the global config instance."""
    global _config
    _config = config


def get_config() -> MCPConfig:
    """Get the global config instance."""
    if _config is None:
        raise RuntimeError("Config not initialized. Call set_config() first.")
    return _config


def register_tools(mcp: FastMCP) -> None:
    """Register all iMi tools with the FastMCP server.

    Args:
        mcp: FastMCP server instance
    """

    @mcp.tool()
    async def create_worktree(
        name: str,
        worktree_type: str = "feat",
        repo: Optional[str] = None,
    ) -> WorktreeResult:
        """Create a new worktree of specified type.

        Creates a git worktree for isolated development work. Supports built-in types
        (feat, fix, aiops, devops, review) and custom user-defined types.

        Args:
            name: Descriptive name for the worktree (e.g., "user-authentication")
            worktree_type: Type of worktree (default: "feat")
            repo: Repository name (optional, uses current repo if not specified)

        Returns:
            WorktreeResult with worktree path and creation status
        """
        config = get_config()
        args = ["add", worktree_type, name]
        if repo:
            args.extend(["--repo", repo])

        result = await run_imi_command(args, config)

        if result.success and result.data:
            return WorktreeResult(
                success=True,
                message=f"Worktree '{name}' created successfully",
                data=result.data,
            )
        else:
            return WorktreeResult(
                success=False,
                message=f"Failed to create worktree '{name}'",
                error=result.error or "Unknown error",
            )

    @mcp.tool()
    async def create_review_worktree(
        pr_number: int,
        repo: Optional[str] = None,
    ) -> WorktreeResult:
        """Create a worktree for reviewing a pull request.

        Fetches the PR branch from GitHub and creates a dedicated review worktree.
        Requires gh CLI authentication.

        Args:
            pr_number: Pull request number to review
            repo: Repository name (optional, uses current repo if not specified)

        Returns:
            WorktreeResult with review worktree path and status
        """
        config = get_config()
        args = ["review", str(pr_number)]
        if repo:
            args.append(repo)

        result = await run_imi_command(args, config)

        if result.success and result.data:
            return WorktreeResult(
                success=True,
                message=f"Review worktree for PR #{pr_number} created successfully",
                data=result.data,
            )
        else:
            return WorktreeResult(
                success=False,
                message=f"Failed to create review worktree for PR #{pr_number}",
                error=result.error or "Unknown error",
            )

    @mcp.tool()
    async def list_worktrees(
        repo: Optional[str] = None,
        worktrees_only: bool = False,
        projects_only: bool = False,
    ) -> WorktreeResult:
        """List all active worktrees and repositories.

        Returns comprehensive worktree information including paths, branches,
        and activity status.

        Args:
            repo: Repository name (optional, shows all repos if not specified)
            worktrees_only: List only worktrees (exclude projects)
            projects_only: List only projects/repositories (exclude worktrees)

        Returns:
            WorktreeResult with list of worktrees/projects
        """
        config = get_config()
        args = ["list"]
        if repo:
            args.extend(["--repo", repo])
        if worktrees_only:
            args.append("--worktrees")
        if projects_only:
            args.append("--projects")

        result = await run_imi_command(args, config)

        if result.success:
            return WorktreeResult(
                success=True,
                message="Worktrees listed successfully",
                data=result.data or {"output": result.stdout},
            )
        else:
            return WorktreeResult(
                success=False,
                message="Failed to list worktrees",
                error=result.error or "Unknown error",
            )

    @mcp.tool()
    async def show_status(repo: Optional[str] = None) -> WorktreeResult:
        """Show status of all worktrees.

        Displays git status, uncommitted changes, and activity for each worktree.

        Args:
            repo: Repository name (optional, shows all repos if not specified)

        Returns:
            WorktreeResult with worktree status information
        """
        config = get_config()
        args = ["status"]
        if repo:
            args.extend(["--repo", repo])

        result = await run_imi_command(args, config)

        if result.success:
            return WorktreeResult(
                success=True,
                message="Status retrieved successfully",
                data=result.data or {"output": result.stdout},
            )
        else:
            return WorktreeResult(
                success=False,
                message="Failed to retrieve status",
                error=result.error or "Unknown error",
            )

    @mcp.tool()
    async def create_project(
        concept: Optional[str] = None,
        prd: Optional[str] = None,
        name: Optional[str] = None,
        payload: Optional[str] = None,
    ) -> WorktreeResult:
        """Create a new project with GitHub repository and boilerplate.

        Bootstraps a complete project with:
        - GitHub repository creation
        - Stack detection (Python/FastAPI, React/Vite, Generic)
        - Boilerplate files (mise.toml, pyproject.toml, package.json)
        - Git initialization and remote push

        Requires GitHub authentication (GITHUB_TOKEN environment variable).

        Args:
            concept: Project concept description (natural language)
            prd: Path to PRD markdown file
            name: Explicit project name (optional, inferred from concept/prd)
            payload: JSON payload for structured project definition

        Returns:
            WorktreeResult with project path and GitHub URL
        """
        config = get_config()
        args = ["project", "create"]

        if concept:
            args.extend(["--concept", concept])
        if prd:
            args.extend(["--prd", prd])
        if name:
            args.extend(["--name", name])
        if payload:
            args.extend(["--payload", payload])

        result = await run_imi_command(args, config)

        if result.success and result.data:
            return WorktreeResult(
                success=True,
                message="Project created successfully",
                data=result.data,
            )
        else:
            return WorktreeResult(
                success=False,
                message="Failed to create project",
                error=result.error or "Unknown error",
            )

    @mcp.tool()
    async def navigate_worktree(
        query: Optional[str] = None,
        repo: Optional[str] = None,
    ) -> WorktreeResult:
        """Navigate to a worktree using fuzzy search.

        Returns the absolute path to the selected worktree. The LLM can use this
        path for subsequent file operations.

        Args:
            query: Fuzzy search query (worktree name, branch name, or repo name)
            repo: Exact repository name (skip fuzzy search within this repo)

        Returns:
            WorktreeResult with target_path for navigation
        """
        config = get_config()
        args = ["go"]
        if query:
            args.append(query)
        if repo:
            args.extend(["--repo", repo])

        result = await run_imi_command(args, config)

        if result.success and result.data:
            return WorktreeResult(
                success=True,
                message="Worktree located successfully",
                data=result.data,
            )
        else:
            return WorktreeResult(
                success=False,
                message="Failed to locate worktree",
                error=result.error or "Unknown error",
            )

    @mcp.tool()
    async def remove_worktree(
        name: str,
        repo: Optional[str] = None,
        keep_branch: bool = False,
        keep_remote: bool = False,
    ) -> WorktreeResult:
        """Remove a worktree and optionally its branches.

        Cleans up the worktree directory and associated git references.

        Args:
            name: Name of the worktree to remove
            repo: Repository name (optional, uses current repo if not specified)
            keep_branch: Keep local branch after removing worktree
            keep_remote: Keep remote branch after removing worktree (requires keep_branch)

        Returns:
            WorktreeResult with removal status
        """
        config = get_config()
        args = ["remove", name]
        if repo:
            args.extend(["--repo", repo])
        if keep_branch:
            args.append("--keep-branch")
        if keep_remote:
            args.append("--keep-remote")

        result = await run_imi_command(args, config)

        if result.success and result.data:
            return WorktreeResult(
                success=True,
                message=f"Worktree '{name}' removed successfully",
                data=result.data,
            )
        else:
            return WorktreeResult(
                success=False,
                message=f"Failed to remove worktree '{name}'",
                error=result.error or "Unknown error",
            )

    @mcp.tool()
    async def sync_worktrees(repo: Optional[str] = None) -> WorktreeResult:
        """Sync database with actual Git worktrees.

        Reconciles the iMi database with the actual git worktree state on disk.
        Useful after manual git operations or external worktree modifications.

        Args:
            repo: Repository name (optional, syncs all repos if not specified)

        Returns:
            WorktreeResult with sync status
        """
        config = get_config()
        args = ["sync"]
        if repo:
            args.extend(["--repo", repo])

        result = await run_imi_command(args, config)

        if result.success and result.data:
            return WorktreeResult(
                success=True,
                message="Worktrees synced successfully",
                data=result.data,
            )
        else:
            return WorktreeResult(
                success=False,
                message="Failed to sync worktrees",
                error=result.error or "Unknown error",
            )

    @mcp.tool()
    async def prune_worktrees(
        repo: Optional[str] = None,
        dry_run: bool = False,
    ) -> WorktreeResult:
        """Clean up stale worktree references from Git.

        Removes orphaned worktree references that no longer exist on disk.

        Args:
            repo: Repository name (optional, uses current repo if not specified)
            dry_run: Show what would be removed without actually removing

        Returns:
            WorktreeResult with prune status
        """
        config = get_config()
        args = ["prune"]
        if repo:
            args.extend(["--repo", repo])
        if dry_run:
            args.append("--dry-run")

        result = await run_imi_command(args, config)

        if result.success and result.data:
            return WorktreeResult(
                success=True,
                message="Prune completed successfully",
                data=result.data,
            )
        else:
            return WorktreeResult(
                success=False,
                message="Failed to prune worktrees",
                error=result.error or "Unknown error",
            )

    @mcp.tool()
    async def list_types() -> WorktreeResult:
        """List all available worktree types.

        Returns both built-in types (feat, fix, aiops, devops, review) and
        user-defined custom types.

        Returns:
            WorktreeResult with list of worktree types and their metadata
        """
        config = get_config()
        args = ["types", "list"]

        result = await run_imi_command(args, config)

        if result.success and result.data:
            return WorktreeResult(
                success=True,
                message="Worktree types listed successfully",
                data=result.data,
            )
        else:
            return WorktreeResult(
                success=False,
                message="Failed to list worktree types",
                error=result.error or "Unknown error",
            )
