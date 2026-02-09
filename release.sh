#!/usr/bin/env bash
set -euo pipefail

# Usage: ./release.sh 0.1.3
#
# That's it. It bumps the version everywhere, commits, tags, and pushes.
# CI handles the rest (builds, GitHub release, Chocolatey).

if [ $# -ne 1 ]; then
  echo "usage: ./release.sh <version>"
  echo ""
  echo "  example: ./release.sh 0.1.3"
  echo ""
  echo "  DO NOT include the 'v' prefix. Just the number."
  exit 1
fi

VERSION="$1"

# Don't let the user accidentally include the v prefix
if [[ "$VERSION" == v* ]]; then
  echo "error: don't include the 'v' prefix, just the version number"
  echo "  example: ./release.sh 0.1.3"
  exit 1
fi

# Make sure we're in the repo root
if [ ! -f Cargo.toml ]; then
  echo "error: run this from the repo root (where Cargo.toml is)"
  exit 1
fi

# Make sure the working tree is clean
if [ -n "$(git status --porcelain)" ]; then
  echo "error: you have uncommitted changes, commit or stash them first"
  exit 1
fi

# Make sure we're on master
BRANCH=$(git rev-parse --abbrev-ref HEAD)
if [ "$BRANCH" != "master" ]; then
  echo "error: you're on '$BRANCH', switch to master first"
  exit 1
fi

echo "Bumping version to $VERSION..."

# Bump Cargo.toml
sed -i "s/^version = \".*\"/version = \"$VERSION\"/" Cargo.toml

# Bump choco nuspec
sed -i "s|<version>.*</version>|<version>$VERSION</version>|" choco/bntui.nuspec

# Bump choco install script
sed -i "s/^\$version = '.*'/\$version = '$VERSION'/" choco/tools/chocolateyinstall.ps1

# Update Cargo.lock
cargo check --quiet 2>/dev/null

echo "Committing..."
git add Cargo.toml Cargo.lock choco/bntui.nuspec choco/tools/chocolateyinstall.ps1
git commit -m "Bump version to $VERSION"

echo "Tagging v$VERSION..."
git tag "v$VERSION"

echo "Pushing..."
git push origin master --tags

echo ""
echo "Done. CI is now building v$VERSION."
echo "  GitHub release: https://github.com/obselate/bntui/releases/tag/v$VERSION"
echo "  CI status:      https://github.com/obselate/bntui/actions"
