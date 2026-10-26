#!/usr/bin/env bash
# Publish HEAD to the public GitHub repository as one new commit on its own history.
# The private history stays local; paths in tools/public_exclude.txt are left out.
# The working tree and index are not touched (a temporary index is used).
# Usage: sh tools/publish.sh [message]
set -euo pipefail
cd "$(dirname "$0")/.."
REMOTE="${RRBTD6_PUBLIC_REMOTE:-public}"
REF=refs/public/main
NAME="blinkywink"
EMAIL="168236311+blinkywink@users.noreply.github.com"

index="$(mktemp)"
trap 'rm -f "$index"' EXIT
export GIT_INDEX_FILE="$index"
git read-tree HEAD
grep -v -e '^#' -e '^[[:space:]]*$' tools/public_exclude.txt | while read -r path; do
  git rm --cached -r -q --ignore-unmatch -- "$path"
done
tree="$(git write-tree)"
unset GIT_INDEX_FILE

git fetch -q "$REMOTE" "+refs/heads/main:$REF" 2>/dev/null || true
parent="$(git rev-parse -q --verify "$REF" || true)"
if [ -n "$parent" ] && [ "$(git rev-parse "$parent^{tree}")" = "$tree" ]; then
  echo "public repo already up to date"; exit 0
fi
message="${1:-Update from $(git rev-parse --short HEAD)}"
commit="$(GIT_AUTHOR_NAME="$NAME" GIT_AUTHOR_EMAIL="$EMAIL" GIT_COMMITTER_NAME="$NAME" GIT_COMMITTER_EMAIL="$EMAIL" \
  git commit-tree "$tree" ${parent:+-p "$parent"} -m "$message")"
git update-ref "$REF" "$commit"
git push "$REMOTE" "$REF:refs/heads/main"
echo "published $commit"
