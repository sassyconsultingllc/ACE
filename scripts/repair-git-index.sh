#!/usr/bin/env bash
set -euo pipefail
echo "Backing up and rebuilding .git/index (safe — won't change working files)"
cp -v .git/index .git/index.bak 2>/dev/null || true
rm -f .git/index
git reset --mixed
echo "Index rebuilt. Now run: git add -A && git commit -m 'fix: rebuild index'"
