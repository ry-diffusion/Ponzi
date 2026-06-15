#!/bin/bash
# CalVer auto-bump: sets version to today's date (YYYY.M.D)
VERSION=$(date +"%Y.%-m.%-d")
sed -i "s/^version = \".*\"/version = \"$VERSION\"/" Cargo.toml
echo "Bumped to $VERSION"
cargo generate-lockfile 2>/dev/null
