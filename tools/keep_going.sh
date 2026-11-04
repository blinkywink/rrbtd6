#!/bin/sh
# Stop hook: refuses to let Claude end its turn while towers are not exact.
# Disable it by creating .claude/STOP_KEEP_GOING (delete the file to re-enable).
cd "$(dirname "$0")/.." || exit 0
[ -f .claude/STOP_KEEP_GOING ] && exit 0
# One free stop while a background job runs (its completion wakes Claude anyway).
if [ -f .claude/keep_going_wait ]; then rm -f .claude/keep_going_wait; exit 0; fi
cat <<'EOF'
{"decision":"block","reason":"Keep going: not every tower is exact yet. Pick the next open item in docs/dev/HANDOFF.md yourself (current tower groups first, then re-record and the next tower) and work on it. Follow the usual rules: verify against the game code, run the full regression in the background before each commit, commit only with no count drops, update HANDOFF/progress, rebuild BTD6.exe and publish at milestones (never the IPA). Do not ask the user what to do next."}
EOF
