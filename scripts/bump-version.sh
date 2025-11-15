#!/usr/bin/env bash

set -e

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

usage() {
    echo "Usage: $0 <version>"
    echo ""
    echo "Arguments:"
    echo "  <version>          Version number (e.g., 1.0.0 or v1.0.0)"
    echo ""
    echo "Examples:"
    echo "  $0 1.0.0"
    echo "  $0 v1.2.3"
    exit 1
}

if [ $# -ne 1 ]; then
    usage
fi

VERSION="$1"

# Strip 'v' prefix if present
VERSION="${VERSION#v}"

# Validate version format (semver)
if ! [[ $VERSION =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo -e "${RED}Error: Invalid version format. Use semver (e.g., 1.0.0)${NC}"
    exit 1
fi

echo -e "${GREEN}Bumping version to ${VERSION}${NC}"

# Get script directory and project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$PROJECT_ROOT"

# Check if we're on main branch
CURRENT_BRANCH=$(git branch --show-current)
if [ "$CURRENT_BRANCH" != "main" ]; then
    echo -e "${YELLOW}Warning: You're not on the main branch (current: $CURRENT_BRANCH)${NC}"
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Check for uncommitted changes
if ! git diff-index --quiet HEAD --; then
    echo -e "${RED}Error: You have uncommitted changes. Please commit or stash them first.${NC}"
    exit 1
fi

# Check for required tools
echo "Checking required tools..."

# Check for cargo-set-version
if ! cargo set-version --help &> /dev/null; then
    echo -e "${YELLOW}Installing cargo-edit for version management...${NC}"
    cargo install cargo-edit
fi

# Check for npm (should already be installed)
if ! command -v npm &> /dev/null; then
    echo -e "${RED}Error: npm is not installed${NC}"
    exit 1
fi

echo "Updating app/Cargo.toml..."
# cd "$PROJECT_ROOT/app"
cargo set-version -p app "$VERSION"

echo "Updating client/package.json..."
cd "$PROJECT_ROOT/client"
# Use --no-git-tag-version to prevent npm from creating a git commit/tag
npm version "$VERSION" --no-git-tag-version

cd "$PROJECT_ROOT"

echo -e "${GREEN}✓ Version bumped to ${VERSION}${NC}"
echo ""
echo "Changed files:"
git diff --name-only

echo ""
echo "Committing changes..."
git add app/Cargo.toml Cargo.lock client/package.json client/package-lock.json
git commit -m "chore: bump version to $VERSION"

echo -e "${GREEN}✓ Version bump committed${NC}"
echo ""
echo -e "${YELLOW}Next steps:${NC}"
echo -e "  1. Push the commit: ${GREEN}git push origin $CURRENT_BRANCH${NC}"
echo -e "  2. Go to: ${GREEN}https://github.com/thomas-god/activities/releases/new${NC}"
echo -e "  3. Select tag: ${GREEN}v${VERSION}${NC} (create new tag)"
echo -e "  4. Write release notes and publish"
echo ""
echo "Once published, CI will automatically build and push Docker images."
