#!/bin/bash
# scripts/release.sh - Automate Indodax CLI releases
# Usage: ./scripts/release.sh [version]
# If version is omitted, it reads from Cargo.toml

set -e

# 1. Get current version from Cargo.toml if not provided
if [ -z "$1" ]; then
    VERSION=$(grep '^version =' Cargo.toml | head -n1 | cut -d'"' -f2)
    echo "Using version $VERSION from Cargo.toml"
else
    VERSION=$1
    echo "Setting version to $VERSION"
    # Update Cargo.toml
    sed -i "s/^version = \".*\"/version = \"$VERSION\"/" Cargo.toml
fi

# 2. Sync package.json
if [ -f "package.json" ]; then
    sed -i "s/\"version\": \".*\"/\"version\": \"$VERSION\"/" package.json
fi

# 3. Update RELEASE_NOTES.md headers
if [ -f "RELEASE_NOTES.md" ]; then
    sed -i "s/Welcome to Indodax CLI v[0-9.]*/Welcome to Indodax CLI v$VERSION/" RELEASE_NOTES.md
    sed -i "s/What's New in v[0-9.]*/What's New in v$VERSION/" RELEASE_NOTES.md
fi

# 4. Update README.md highlights
if [ -f "README.md" ]; then
    sed -i "s/Recent Highlights (v[0-9.]*)/Recent Highlights (v$VERSION)/" README.md
fi

# 5. Update TODO.md
if [ -f "TODO.md" ]; then
    sed -i "s/WebSocket Reliability Overhaul (v[0-9.]*)/WebSocket Reliability Overhaul (v$VERSION)/" TODO.md
fi

echo "✅ All files updated to v$VERSION"

# 6. Git Operations (Optional - uncomment if you want auto-commit)
# git add Cargo.toml package.json RELEASE_NOTES.md README.md TODO.md
# git commit -m "chore: release v$VERSION"
# git push origin main
# git tag -f v$VERSION
# git push origin v$VERSION -f

# 7. GitHub Release
if command -v gh &> /dev/null; then
    echo "Updating GitHub release v$VERSION..."
    gh release edit "v$VERSION" --notes-file RELEASE_NOTES.md || \
    gh release create "v$VERSION" --title "v$VERSION" --notes-file RELEASE_NOTES.md
fi

echo "🚀 Release v$VERSION completed successfully!"
