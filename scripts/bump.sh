#!/bin/bash
# Script to bump version and create a new release

set -euo pipefail

# Function to show usage
usage() {
    echo "Usage: $0 <major|minor|patch|VERSION>"
    echo ""
    echo "Examples:"
    echo "  $0 patch    # 0.1.0 -> 0.1.1"
    echo "  $0 minor    # 0.1.0 -> 0.2.0"
    echo "  $0 major    # 0.1.0 -> 1.0.0"
    echo "  $0 1.2.3    # Set specific version"
    exit 1
}

# Check if argument provided
if [ $# -ne 1 ]; then
    usage
fi

# Get current version
CURRENT_VERSION=$(grep '^version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/')
echo "Current version: $CURRENT_VERSION"

# Parse current version
IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT_VERSION"

# Calculate new version
case "$1" in
    "major")
        NEW_VERSION="$((MAJOR + 1)).0.0"
        ;;
    "minor")
        NEW_VERSION="$MAJOR.$((MINOR + 1)).0"
        ;;
    "patch")
        NEW_VERSION="$MAJOR.$MINOR.$((PATCH + 1))"
        ;;
    *)
        # Assume it's a specific version
        if [[ "$1" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
            NEW_VERSION="$1"
        else
            echo "Invalid version format: $1"
            usage
        fi
        ;;
esac

echo "New version: $NEW_VERSION"

# Confirm with user
read -p "Continue with version bump? (y/N): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Aborted"
    exit 1
fi

# Update Cargo.toml
echo "Updating Cargo.toml..."
sed -i.bak "s/^version = \".*\"/version = \"$NEW_VERSION\"/" Cargo.toml
rm Cargo.toml.bak

# Update Cargo.lock if it exists
if [ -f Cargo.lock ]; then
    echo "Updating Cargo.lock..."
    cargo check --quiet
fi

# Commit changes
echo "Committing changes..."
git add Cargo.toml
if [ -f Cargo.lock ]; then
    git add Cargo.lock
fi
git commit -m "chore: bump version to $NEW_VERSION"

# Create and push tag
echo "Creating tag v$NEW_VERSION..."
git tag "v$NEW_VERSION"

echo "Version bumped to $NEW_VERSION"
echo ""
echo "To trigger release, push the tag:"
echo "   git push origin v$NEW_VERSION"
echo ""
echo "Or push everything:"
echo "   git push && git push --tags"
