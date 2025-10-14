#!/bin/bash
# iMi Quick Start Script
# This script helps you get started with iMi

set -e

echo "🚀 iMi Quick Start Guide"
echo "========================"
echo ""

# Check if iMi is installed
if ! command -v iMi &> /dev/null; then
    echo "❌ iMi is not installed!"
    echo ""
    echo "Installing iMi from source..."
    cd "$(dirname "$0")"
    cargo install --path .
    echo "✅ iMi installed successfully!"
else
    echo "✅ iMi is already installed at: $(which iMi)"
fi

echo ""
echo "📍 Current location: $(pwd)"
echo ""

# Check if in a git repository
if ! git rev-parse --git-dir &> /dev/null; then
    echo "⚠️  You are not in a Git repository."
    echo ""
    echo "To use iMi, you need to:"
    echo "  1. Navigate to a Git repository"
    echo "  2. Or clone/create one first"
    echo ""
    echo "Example:"
    echo "  cd ~/code"
    echo "  git clone <your-repo-url> trunk-main"
    echo "  cd trunk-main"
    echo "  iMi init"
    exit 1
fi

echo "✅ Git repository detected"
echo ""

# Check directory name
current_dir=$(basename "$PWD")
if [[ ! "$current_dir" =~ ^trunk- ]]; then
    echo "⚠️  Current directory: $current_dir"
    echo ""
    echo "iMi works best when initialized from a 'trunk-*' directory."
    echo ""
    echo "Recommendation:"
    echo "  cd .."
    echo "  mv $current_dir trunk-main"
    echo "  cd trunk-main"
    echo "  iMi init"
    echo ""
    echo "Or continue anyway with: iMi init"
else
    echo "✅ Correct directory naming: $current_dir"
    echo ""
    
    # Offer to initialize
    echo "Would you like to initialize iMi here? (y/n)"
    read -r response
    if [[ "$response" =~ ^[Yy]$ ]]; then
        echo ""
        echo "Initializing iMi..."
        iMi init
        echo ""
        echo "✅ iMi initialized!"
    fi
fi

echo ""
echo "📚 Quick Command Reference:"
echo "  iMi feat <name>      - Create feature worktree"
echo "  iMi fix <name>       - Create bugfix worktree"
echo "  iMi review <pr>      - Create PR review worktree"
echo "  iMi status           - Show all worktrees"
echo "  iMi monitor          - Monitor activities"
echo "  iMi --help           - Full help"
echo ""
echo "📖 Full documentation:"
echo "  README.md   - Features and examples"
echo "  INSTALL.md  - Installation guide"
echo "  GEMINI.md   - Technical details"
echo ""
echo "🎉 You're ready to use iMi!"
