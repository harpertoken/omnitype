#!/bin/bash
set -euo pipefail

# Set up git config for the action
git config --global user.name "Release Bot"
git config --global user.email "release-bot@example.com"

# Get the latest two tags
LATEST_TAG=$(git describe --tags --abbrev=0)
PREVIOUS_TAG=$(git describe --tags --abbrev=0 ${LATEST_TAG}^ 2>/dev/null || echo "")

if [ -z "$PREVIOUS_TAG" ]; then
    echo "No previous tag found. Using first commit as base."
    PREVIOUS_TAG=$(git rev-list --max-parents=0 HEAD)
fi

# Create a markdown file with the diffs
OUTPUT_FILE="release_${LATEST_TAG}_changes.md"
echo "# Code Changes in $LATEST_TAG" > "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"
echo "## Full Changelog" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

# Get list of commits between tags
echo "## Commits" >> "$OUTPUT_FILE"
git log --pretty=format:'- %h %s (%an)' ${PREVIOUS_TAG}..${LATEST_TAG} >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

# Get list of changed files
echo "## Changed Files" >> "$OUTPUT_FILE"
git diff --name-only ${PREVIOUS_TAG}..${LATEST_TAG} | sort >> "$OUTPUT_FILE" || echo "No files changed" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

# Get detailed diffs for Rust files
echo "## Rust File Diffs" >> "$OUTPUT_FILE"
for file in $(git diff --name-only ${PREVIOUS_TAG}..${LATEST_TAG} | grep '\.rs$' || true); do
    if [ -n "$file" ]; then
        echo "" >> "$OUTPUT_FILE"
        echo "### $file" >> "$OUTPUT_FILE"
        echo '```diff' >> "$OUTPUT_FILE"
        git diff ${PREVIOUS_TAG}..${LATEST_TAG} -- "$file" >> "$OUTPUT_FILE" 2>/dev/null || echo "No changes" >> "$OUTPUT_FILE"
        echo '```' >> "$OUTPUT_FILE"
    fi
done

echo "## Summary of Changes" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"
echo "### Added Features" >> "$OUTPUT_FILE"
git log --pretty=format:'- %s' ${PREVIOUS_TAG}..${LATEST_TAG} | grep -i 'feat\|add\|new' | grep -v 'Merge' >> "$OUTPUT_FILE" || echo "No new features" >> "$OUTPUT_FILE"

echo "" >> "$OUTPUT_FILE"
echo "### Bug Fixes" >> "$OUTPUT_FILE"
git log --pretty=format:'- %s' ${PREVIOUS_TAG}..${LATEST_TAG} | grep -i 'fix\|bug\|error' | grep -v 'Merge' >> "$OUTPUT_FILE" || echo "No bug fixes" >> "$OUTPUT_FILE"

echo "" >> "$OUTPUT_FILE"
echo "### Performance Improvements" >> "$OUTPUT_FILE"
git log --pretty=format:'- %s' ${PREVIOUS_TAG}..${LATEST_TAG} | grep -i 'perf\|speed\|optimize' | grep -v 'Merge' >> "$OUTPUT_FILE" || echo "No performance improvements" >> "$OUTPUT_FILE"

echo ""
echo "Release notes generated in $OUTPUT_FILE"
