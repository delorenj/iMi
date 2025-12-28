"""Pydantic models for iMi MCP tool inputs and outputs."""

from typing import Any, Dict, Optional

from pydantic import BaseModel, Field


# Tool Input Models


class CreateWorktreeInput(BaseModel):
    """Input for creating a worktree."""

    name: str = Field(description="Descriptive name for the worktree")
    worktree_type: str = Field(
        default="feat", description="Worktree type (feat, fix, aiops, devops, etc.)"
    )
    repo: Optional[str] = Field(
        default=None, description="Repository name (optional, uses current if not specified)"
    )


class ReviewWorktreeInput(BaseModel):
    """Input for creating a PR review worktree."""

    pr_number: int = Field(description="Pull request number", gt=0)
    repo: Optional[str] = Field(
        default=None, description="Repository name (optional, uses current if not specified)"
    )


class ListWorktreesInput(BaseModel):
    """Input for listing worktrees."""

    repo: Optional[str] = Field(
        default=None, description="Repository name (optional, shows all repos if not specified)"
    )
    worktrees_only: bool = Field(default=False, description="List only worktrees")
    projects_only: bool = Field(default=False, description="List only projects/repositories")


class ProjectCreateInput(BaseModel):
    """Input for creating a new project."""

    concept: Optional[str] = Field(
        default=None, description="Project concept description (natural language)"
    )
    prd: Optional[str] = Field(default=None, description="Path to PRD markdown file")
    name: Optional[str] = Field(
        default=None,
        description="Explicit project name (optional, will be inferred from concept/prd if not provided)",
    )
    payload: Optional[str] = Field(
        default=None, description="JSON payload for structured project definition"
    )


class RemoveWorktreeInput(BaseModel):
    """Input for removing a worktree."""

    name: str = Field(description="Name of the worktree to remove")
    repo: Optional[str] = Field(
        default=None, description="Repository name (optional, uses current if not specified)"
    )
    keep_branch: bool = Field(default=False, description="Keep local branch after removing worktree")
    keep_remote: bool = Field(
        default=False, description="Keep remote branch after removing worktree"
    )


class NavigateInput(BaseModel):
    """Input for navigating to a worktree."""

    query: Optional[str] = Field(
        default=None, description="Fuzzy search query (worktree name, branch name, or repo name)"
    )
    repo: Optional[str] = Field(
        default=None, description="Exact repository name (skip fuzzy search within this repo)"
    )


class SyncInput(BaseModel):
    """Input for syncing database with actual Git worktrees."""

    repo: Optional[str] = Field(
        default=None, description="Repository name (optional, syncs all repos if not specified)"
    )


class PruneInput(BaseModel):
    """Input for pruning stale worktrees."""

    repo: Optional[str] = Field(
        default=None, description="Repository name (optional, uses current if not specified)"
    )
    dry_run: bool = Field(
        default=False, description="Show what would be removed without actually removing"
    )


# Tool Output Models


class CLIResult(BaseModel):
    """Result from executing an iMi CLI command."""

    success: bool
    data: Optional[Dict[str, Any]] = None
    error: Optional[str] = None
    stdout: Optional[str] = None
    stderr: Optional[str] = None
    exit_code: int = 0


class WorktreeResult(BaseModel):
    """Standardized result for MCP tool responses."""

    success: bool
    message: str
    data: Optional[Dict[str, Any]] = None
    error: Optional[str] = None
