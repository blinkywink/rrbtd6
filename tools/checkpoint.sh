#!/bin/sh
# Save progress so another session can continue exactly here (see docs/dev/HANDOFF.md).
#
#   sh tools/checkpoint.sh "Wizard 0-0-5: undead spawn on the First segment list"
#
# Commits every tracked change plus docs/dev/HANDOFF.md and any new files under renderer/src, tools/
# and research/*.md, on the current branch (master). Run it after every verified step, with
# docs/dev/HANDOFF.md's "Now" and "Log" sections already updated. Nothing is pushed.
set -e
cd "$(dirname "$0")/.."
msg="${1:?usage: sh tools/checkpoint.sh \"what changed\"}"
git add -u
git add docs/dev/HANDOFF.md 2>/dev/null || true
# New source / tool / note files (never local_extracted or research/dumps: both git-ignored).
git ls-files --others --exclude-standard -- renderer/src tools research '*.md' | while read -r f; do
  case "$f" in *.log|*.tmp) continue;; esac
  git add -- "$f"
done
if git diff --cached --quiet; then
  echo "checkpoint: nothing to commit"
  exit 0
fi
git commit -q -m "$msg" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git log --oneline -1
