#!/bin/bash
# Test script to simulate release workflow locally

set -euo pipefail

echo "Testing release workflow locally..."
echo ""

# Get current version from Cargo.toml
CURRENT_VERSION=$(grep '^version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/')
echo "Current version in Cargo.toml: $CURRENT_VERSION"

# Check if we have any tags
TAGS=$(git tag -l | wc -l)
echo "Existing tags: $TAGS"

if [ "$TAGS" -eq 0 ]; then
    echo "First release detected"
    echo ""
    echo "Generated changelog:"
    echo "## Changes in v$CURRENT_VERSION"
    echo ""
    echo "**Initial Release**"
    echo ""
    echo "This is the first release of OmniType!"
    echo ""
    echo "### All Commits"
    git log --pretty=format:'- %h %s (%an)' --reverse | head -10
    if [ $(git log --oneline | wc -l) -gt 10 ]; then
        echo "... and $(( $(git log --oneline | wc -l) - 10 )) more commits"
    fi
else
    LATEST_TAG=$(git describe --tags --abbrev=0)
    echo "Latest tag: $LATEST_TAG"

    echo ""
    echo "Generated changelog:"
    echo "## Changes in v$CURRENT_VERSION"
    echo ""
    echo "### Commits since $LATEST_TAG"
    git log --pretty=format:'- %h %s (%an)' ${LATEST_TAG}..HEAD
fi

echo ""
echo "Test completed"
