#!/usr/bin/env bash
set -euo pipefail
echo "Scanning for suspicious filenames (NUL, non-printable, tempwarn)"
git ls-files -z --others --exclude-standard | tr '\0' '\n' | sed -n l | grep -nE '(^NUL$)|tempwarn|\\x00|[[:cntrl:]]' || true
git ls-files -z | tr '\0' '\n' | sed -n l | grep -nE '(^NUL$)|tempwarn|\\x00|[[:cntrl:]]' || true
echo "Remove any hits and retry 'git add -A'."
