#!/usr/bin/env bash
# check-secrets.sh — Scan tracked source files for accidentally committed secrets.
# Exits 1 if any forbidden patterns are found.
#
# Usage:
#   bash scripts/check-secrets.sh
#
set -euo pipefail

# Patterns that indicate a real secret value (not a placeholder or comment).
# Each entry is an extended-regex that matches key=value or key: value forms
# where the value looks like a real credential.
FORBIDDEN_PATTERNS=(
  # Stellar secret keys (56-char base32 starting with S)
  '(STELLAR_(FEE_BUMP_)?SECRET(_KEY)?|SECRET_KEY)\s*[=:]\s*S[A-Z2-7]{55}[^A-Z2-7]'
  # Vercel / generic opaque tokens assigned real values
  'BLOB_READ_WRITE_TOKEN\s*[=:]\s*[A-Za-z0-9_\-]{20,}'
  'KV_REST_API_TOKEN\s*[=:]\s*[A-Za-z0-9_\-]{20,}'
  'VERCEL_TOKEN\s*[=:]\s*[A-Za-z0-9_\-]{20,}'
  # GitHub Personal Access Tokens
  'GITHUB_TOKEN\s*[=:]\s*gh[pousr]_[A-Za-z0-9_]{36,}'
  # Generic private/secret key assignments (catch-all)
  '(private_key|PRIVATE_KEY)\s*[=:]\s*S[A-Z2-7]{55}[^A-Z2-7]'
)

# Files and paths that are intentionally allowed to contain patterns above
# (documentation, examples, this script itself).
EXCLUDE_PATHSPECS=(
  ':(exclude)*.example'
  ':(exclude)*.md'
  ':(exclude)scripts/check-secrets.sh'
  ':(exclude).github/workflows/*.yml'
)

found=0

for pattern in "${FORBIDDEN_PATTERNS[@]}"; do
  results=$(git grep -rn -E "$pattern" -- "${EXCLUDE_PATHSPECS[@]}" 2>/dev/null || true)
  if [[ -n "$results" ]]; then
    echo "ERROR: Potential secret found (pattern: ${pattern})"
    echo "$results"
    echo ""
    found=1
  fi
done

if [[ $found -eq 1 ]]; then
  echo "Secret scan FAILED."
  echo "Remove secret values from source files and reference them via environment variables instead."
  echo "See docs/SECRET_ROTATION_RUNBOOK.md for the full secret inventory and management procedures."
  exit 1
fi

echo "Secret scan passed — no committed secrets detected."
