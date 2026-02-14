#!/usr/bin/env bash
set -euo pipefail
echo "Untracked/tracked files that look suspicious:"
git ls-files -z --others --exclude-standard | tr '\0' '\n' | grep -nE '(^NUL$)|tempwarn|[[:cntrl:]]' || true
git ls-files -z | tr '\0' '\n' | grep -nE '(^NUL$)|tempwarn|[[:cntrl:]]' || true
echo "Delete any offending paths and retry 'git add'."
