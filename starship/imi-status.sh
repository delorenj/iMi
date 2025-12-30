#!/bin/bash

# Configuration
IMI_CONFIG="$HOME/.config/iMi/config.toml"
ICON_IMI="⛩️ " # Shinto shrine representing the "Structure"
ICON_UNREGISTERED="💤"
ICON_AGENT="🤖"
COLOR_Active="green"
COLOR_AGENT="purple"
COLOR_UNREGISTERED="dimmed white"

# 1. Check if iMi is even configured for this user
if [ ! -f "$IMI_CONFIG" ]; then
    exit 0
fi

# 2. Check if we are in a git repo
IS_GIT=$(git rev-parse --is-inside-work-tree 2>/dev/null)
if [ -z "$IS_GIT" ]; then
    exit 0
fi

# Get the root of the current git worktree
GIT_ROOT=$(git rev-parse --show-toplevel)
# Get the parent directory (The "Project Cluster" root in iMi structure)
PROJECT_ROOT=$(dirname "$GIT_ROOT")

# 3. Check for iMi Structure (Look for the 'sync' folder in the project root)
if [ -d "$PROJECT_ROOT/sync" ]; then
    # --- REGISTERED iMi REPO ---
    
    # Detect Project Name
    PROJECT_NAME=$(basename "$PROJECT_ROOT")
    
    # Detect Worktree Name
    WORKTREE_NAME=$(basename "$GIT_ROOT")
    
    # Detect Agent Activity (Mockup: checking for a hypothetical lock file)
    # You would adjust this based on how your 'monitor' command works
    AGENT_ACTIVE=false
    if [ -f "$PROJECT_ROOT/.imi/agents/$WORKTREE_NAME.lock" ]; then
        AGENT_ACTIVE=true
    fi

    # Formatting Icons based on Worktree
    case $WORKTREE_NAME in
        trunk-main) WT_ICON="🌳" ;;
        feat-*)     WT_ICON="🔨" ;;
        fix-*)      WT_ICON="🚑" ;;
        pr-*)       WT_ICON="👓" ;;
        aiops-*)    WT_ICON="🧠" ;;
        *)          WT_ICON="📦" ;;
    esac

    # Output Logic
    if [ "$AGENT_ACTIVE" = true ]; then
        # Agent is working here!
        echo "{\"text\": \"$ICON_AGENT $PROJECT_NAME/$WORKTREE_NAME\", \"style\": \"$COLOR_AGENT\"}"
    else
        # Standard View
        echo "{\"text\": \"$ICON_IMI $WT_ICON $PROJECT_NAME\", \"style\": \"$COLOR_Active\"}"
    fi

else
    # --- UNREGISTERED GIT REPO ---
    # Only show if specific requirement met: iMi config exists, but repo not registered
    echo "{\"text\": \"$ICON_UNREGISTERED iMi?\", \"style\": \"$COLOR_UNREGISTERED\"}"
fi
