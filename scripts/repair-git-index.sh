#!/usr/bin/env bash
set -euo pipefail
# backup + rebuild repo index (safe; does not change working files)
cp -v .git/index .git/index.bak 2>/dev/null || true
rm -f .git/index
git reset --mixed
echo "Index rebuilt. Run: git add -A && git commit -m '...' && git push"
