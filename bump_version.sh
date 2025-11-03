#!/bin/bash

# Version bump script for iMi project
# Usage: ./bump_version.sh [patch|minor|major]

set -e

if [ $# -ne 1 ]; then
    echo "Usage: $0 [patch|minor|major]"
    exit 1
fi

BUMP_TYPE=$1

# Get current version from Cargo.toml
CURRENT_VERSION=$(grep -E '^version = "' Cargo.toml | head -1 | cut -d'"' -f2)

if [ -z "$CURRENT_VERSION" ]; then
    echo "Error: Could not find version in Cargo.toml"
    exit 1
fi

echo "Current version: $CURRENT_VERSION"

# Parse version components
IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT_VERSION"

# Bump version based on type
case $BUMP_TYPE in
    patch)
        PATCH=$((PATCH + 1))
        NEW_VERSION="$MAJOR.$MINOR.$PATCH"
        COMMIT_MSG="chore: bump version to $NEW_VERSION (patch)"
        ;;
    minor)
        MINOR=$((MINOR + 1))
        PATCH=0
        NEW_VERSION="$MAJOR.$MINOR.$PATCH"
        COMMIT_MSG="chore: bump version to $NEW_VERSION (minor)"
        ;;
    major)
        MAJOR=$((MAJOR + 1))
        MINOR=0
        PATCH=0
        NEW_VERSION="$MAJOR.$MINOR.$PATCH"
        COMMIT_MSG="chore: bump version to $NEW_VERSION (major)"
        ;;
    *)
        echo "Error: Invalid bump type. Use patch, minor, or major."
        exit 1
        ;;
esac

echo "New version: $NEW_VERSION"

# Update Cargo.toml
sed -i.bak "s/^version = \".*\"/version = \"$NEW_VERSION\"/" Cargo.toml
rm -f Cargo.toml.bak

# Verify the change
UPDATED_VERSION=$(grep -E '^version = "' Cargo.toml | head -1 | cut -d'"' -f2)
if [ "$UPDATED_VERSION" != "$NEW_VERSION" ]; then
    echo "Error: Version update failed"
    exit 1
fi

echo "Version updated successfully to $NEW_VERSION"

# Git operations
git add Cargo.toml
git commit -m "$COMMIT_MSG"
git tag "v$NEW_VERSION"

echo "Git commit and tag created for version $NEW_VERSION"