#!/bin/bash

# ==============================================================================
# iMi Starship Module
# "The Heads-Up Display for your Agentic Workflow"
# ==============================================================================

# --- Configuration -----------------------------------------------------------
IMI_CONFIG="$HOME/.config/iMi/config.toml"
ICON_IMI="⛩️ " # The Cluster Root
ICON_AGENT="🤖" # Active Agent Presence
ICON_LOCK="🔒"  # Manual Lock
COLOR_ACTIVE="green"
COLOR_AGENT="purple bold"
COLOR_WARN="yellow"
COLOR_ERR="red bold"

# 1. FAILING FAST -------------------------------------------------------------
# If iMi isn't configured, or we aren't in a git repo, bail out immediately.
if [ ! -f "$IMI_CONFIG" ]; then exit 0; fi

# Get the top-level directory of the current git worktree
GIT_ROOT=$(git rev-parse --show-toplevel 2>/dev/null)
if [ -z "$GIT_ROOT" ]; then exit 0; fi

# Get the "Cluster Root" (The directory ABOVE the worktree)
# Standard iMi layout: ~/code/project/trunk-main -> Project Root is ~/code/project
PROJECT_ROOT=$(dirname "$GIT_ROOT")
IMI_DIR="$PROJECT_ROOT/.iMi"

# If this repo isn't managed by iMi (no .iMi folder), show nothing (or a subtle nudger)
if [ ! -d "$IMI_DIR" ]; then
	# Optional: Show a subtle "unregistered" icon if you want to be nagged
	# echo "{\"text\": \"💤\", \"style\": \"dimmed white\"}"
	exit 0
fi

# 2. IDENTIFY CONTEXT ---------------------------------------------------------
PROJECT_NAME=$(basename "$PROJECT_ROOT")
WORKTREE_NAME=$(basename "$GIT_ROOT")
PRESENCE_FILE="$IMI_DIR/presence/$WORKTREE_NAME.lock"

# 3. CHECK FOR AGENT ACTIVITY (The "Purple" State) ----------------------------
# This is the High-Priority Alert. If an agent is working, we override everything.
if [ -f "$PRESENCE_FILE" ]; then
	# Try to read the agent name from the lock file
	AGENT_NAME=$(head -n 1 "$PRESENCE_FILE" 2>/dev/null)
	if [ -z "$AGENT_NAME" ]; then AGENT_NAME="Unknown Agent"; fi

	OUTPUT="$ICON_AGENT $AGENT_NAME working in $WORKTREE_NAME"
	echo "{\"text\": \"$OUTPUT\", \"style\": \"$COLOR_AGENT\"}"
	exit 0
fi

# 4. DETERMINE WORKTREE TYPE (The "Standard" State) ---------------------------
# We parse the folder name convention since it's faster than querying SQLite.
# (Alternatively, read a local .imi-meta file if you implement the cache)

case $WORKTREE_NAME in
trunk-main | main | master)
	WT_ICON="🌳" # Trunk
	WT_TYPE="Trunk"
	;;
feat-*)
	WT_ICON="🔨" # Feature
	WT_TYPE="Feat"
	;;
fix-*)
	WT_ICON="🚑" # Hotfix
	WT_TYPE="Fix"
	;;
pr-*)
	WT_ICON="👓" # Review
	WT_TYPE="Review"
	;;
aiops-*)
	WT_ICON="🧠" # AI Ops
	WT_TYPE="AIOps"
	;;
devops-*)
	WT_ICON="🏗️" # DevOps
	WT_TYPE="DevOps"
	;;
*)
	WT_ICON="📦" # Generic
	WT_TYPE="Misc"
	;;
esac

# 5. FINAL OUTPUT -------------------------------------------------------------
# Format: [Icon] [Project] / [Worktree Icon] [Worktree Name]
# Example: ⛩️ my-app / 🔨 feat-auth

TEXT="$ICON_IMI $PROJECT_NAME / $WT_ICON $WORKTREE_NAME"
echo "{\"text\": \"$TEXT\", \"style\": \"$COLOR_ACTIVE\"}"
