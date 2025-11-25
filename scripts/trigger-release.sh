#!/bin/bash

set -euo pipefail

# Require commit SHA as argument
if [ $# -eq 0 ]; then
  echo "Usage: $0 <commit-sha on main branch>"
  echo ""
  echo "Example:"
  echo "  $0 abc123def          # Tag specific commit"
  exit 1
fi

COMMIT_SHA="$1"

VERSION=$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].version')

RELEASE_TAG="v$VERSION"

check_release_exists() {
  gh release view "$RELEASE_TAG" >/dev/null 2>&1
}

echo "Target release: $RELEASE_TAG"

if check_release_exists; then
  echo "Release $RELEASE_TAG already exists! Check the release status in https://github.com/block/ai-rules/releases"
  exit 1
fi

# Verify the commit exists
if ! git rev-parse --verify "$COMMIT_SHA" >/dev/null 2>&1; then
  echo "Error: Commit '$COMMIT_SHA' not found"
  exit 1
fi

COMMIT_SHA_RESOLVED=$(git rev-parse "$COMMIT_SHA")
echo "Tagging commit: $COMMIT_SHA_RESOLVED"
git --no-pager log -1 --oneline "$COMMIT_SHA_RESOLVED"

echo ""
echo "Creating and pushing tag $RELEASE_TAG..."
git tag "$RELEASE_TAG" "$COMMIT_SHA_RESOLVED"
git push origin "$RELEASE_TAG"

echo "Tag $RELEASE_TAG pushed successfully!"
echo "Release workflow should start automatically. Check: https://github.com/block/ai-rules/actions/workflows/release.yml"