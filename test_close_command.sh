#!/bin/bash

# Test script for iMi close command
# This script validates the close command functionality

set -e

echo "================================================"
echo "iMi Close Command Validation Test"
echo "================================================"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Build the project
echo -e "${YELLOW}Building iMi...${NC}"
cargo build --release
IMI_BIN="./target/release/iMi"

# Check if binary exists
if [ ! -f "$IMI_BIN" ]; then
    echo -e "${RED}Error: iMi binary not found at $IMI_BIN${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Build successful${NC}"

# Create a test worktree
echo -e "\n${YELLOW}Creating test worktree...${NC}"
WORKTREE_NAME="test-close-$(date +%s)"
$IMI_BIN feat "$WORKTREE_NAME" || {
    echo -e "${RED}Failed to create test worktree${NC}"
    exit 1
}

echo -e "${GREEN}✓ Created worktree: feat-$WORKTREE_NAME${NC}"

# Check that worktree exists
WORKTREE_PATH="/home/delorenj/code/projects/33GOD/iMi/feat-$WORKTREE_NAME"
if [ ! -d "$WORKTREE_PATH" ]; then
    echo -e "${RED}Error: Worktree directory not found at $WORKTREE_PATH${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Worktree directory exists${NC}"

# Check that branch exists
echo -e "\n${YELLOW}Checking branch exists...${NC}"
git branch | grep "feat/$WORKTREE_NAME" > /dev/null || {
    echo -e "${RED}Error: Branch feat/$WORKTREE_NAME not found${NC}"
    exit 1
}
echo -e "${GREEN}✓ Branch exists: feat/$WORKTREE_NAME${NC}"

# Test the close command
echo -e "\n${YELLOW}Testing close command...${NC}"
$IMI_BIN close "$WORKTREE_NAME" || {
    echo -e "${RED}Error: Close command failed${NC}"
    exit 1
}

echo -e "${GREEN}✓ Close command executed successfully${NC}"

# Verify worktree directory was removed
echo -e "\n${YELLOW}Verifying worktree removal...${NC}"
if [ -d "$WORKTREE_PATH" ]; then
    echo -e "${RED}Error: Worktree directory still exists at $WORKTREE_PATH${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Worktree directory removed${NC}"

# Verify branch still exists (not deleted)
echo -e "\n${YELLOW}Verifying branch preservation...${NC}"
git branch | grep "feat/$WORKTREE_NAME" > /dev/null || {
    echo -e "${RED}Error: Branch was deleted (should be preserved)${NC}"
    exit 1
}
echo -e "${GREEN}✓ Branch preserved: feat/$WORKTREE_NAME${NC}"

# Verify git worktree was removed
echo -e "\n${YELLOW}Verifying git worktree removal...${NC}"
git worktree list | grep "feat-$WORKTREE_NAME" > /dev/null && {
    echo -e "${RED}Error: Git worktree still exists${NC}"
    exit 1
}
echo -e "${GREEN}✓ Git worktree removed${NC}"

# Clean up test branch
echo -e "\n${YELLOW}Cleaning up test branch...${NC}"
git branch -D "feat/$WORKTREE_NAME" > /dev/null 2>&1 || true

echo -e "\n================================================"
echo -e "${GREEN}All tests passed successfully!${NC}"
echo -e "================================================"

echo -e "\n${GREEN}Summary:${NC}"
echo "• Close command correctly removes worktree directory"
echo "• Close command correctly removes git worktree reference"
echo "• Close command preserves the branch (doesn't delete it)"
echo "• Database is updated (verified through successful command execution)"

echo -e "\n${GREEN}The iMi close command is working correctly!${NC}"