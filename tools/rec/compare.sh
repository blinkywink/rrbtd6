#!/bin/bash
# Compare recordings against the clone; one line per non-matching recording, then a total.
#
#   sh tools/rec/compare.sh wizard sub          # research/dumps/recordings/pending/<set>/*.rec
#   sh tools/rec/compare.sh --root dart boomerang   # research/dumps/recordings/<prefix>_*.rec
#   sh tools/rec/compare.sh --one pending/wizard/wizard_005.rec   # one recording, full report
#
# Builds the test with -j 2, no debug info and 16 codegen units in target/check-lite (sets: once,
# then the recordings run RRBTD6_JOBS at a time, default 12): the
# default profile (thin LTO, 1 codegen unit) runs Windows out of commit memory when other big
# apps are open ("memory allocation ... failed" / STATUS_STACK_BUFFER_OVERRUN).
set -u
cd "$(dirname "$0")/../../renderer"
REC="$(cd .. && pwd)/research/dumps/recordings"
REC_W="$(cygpath -m "$REC" 2>/dev/null || echo "$REC")"
run() {
  CARGO_PROFILE_RELEASE_DEBUG=0 CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16 RUST_MIN_STACK=268435456 \
    RRBTD6_REC="$1" CARGO_TARGET_DIR=../target/check-lite \
    cargo test -j 2 -p btd6-game --release compare_recording -- --ignored --nocapture 2>&1
}
if [ "${1:-}" = "--one" ]; then
  run "$REC_W/$2" | grep -E "^(MATCH|DIFF)|^    (first|game  |clone |stats|tower |xp )|^  rt |^  rt [0-9]* clone towerpos|^RNG |^DBGWIND |^DBGAIM |^DBGBT |^  rt [0-9]* clone effects|^error|memory allocation"
  exit 0
fi
# Sets: build once, then run every recording's replay in parallel (RRBTD6_JOBS at a time,
# default 12; each is one single-threaded process), and print the results set by set.
EXE=$(CARGO_PROFILE_RELEASE_DEBUG=0 CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16 RUST_MIN_STACK=268435456 \
  CARGO_TARGET_DIR=../target/check-lite cargo test -j 2 -p btd6-game --release --no-run --message-format=json 2>/dev/null \
  | grep '"executable":"[^"]' | grep '"name":"btd6-game"' | grep -o '"executable":"[^"]*"' | tail -1 | cut -d'"' -f4)
if [ -z "$EXE" ]; then
  echo "error: the test build failed (run with --one to see)"
  exit 1
fi
OUTD=$(mktemp -d)
root=0
if [ "${1:-}" = "--root" ]; then root=1; shift; fi
list=()
for name in "$@"; do
  if [ $root = 1 ]; then files=("$REC"/${name}_*.rec); else files=("$REC"/pending/$name/*.rec); fi
  for f in "${files[@]}"; do
    b=$(basename "$f")
    # Effect / visual captures aren't simulation comparisons.
    case "$b" in *_fx*|*fxp*|*rot3d*|*hand_lvl*|menu_*) continue;; esac
    list+=("$name|${f#"$REC"/}")
  done
done
printf '%s\n' "${list[@]}" | xargs -P "${RRBTD6_JOBS:-12}" -I{} bash -c '
  name=${1%%|*}; rel=${1#*|}; b=$(basename "$rel")
  mkdir -p "$3/$name"
  RUST_MIN_STACK=268435456 RRBTD6_REC="$2/$rel" "$4" compare_recording --ignored --nocapture 2>&1 \
    | grep -E "^(MATCH|DIFF)|^error|memory allocation" | head -1 > "$3/$name/$b.out"' _ {} "$REC_W" "$OUTD" "$EXE"
for name in "$@"; do
  m=0; n=0
  for o in "$OUTD/$name"/*.out; do
    [ -e "$o" ] || continue
    line=$(cat "$o"); n=$((n+1))
    case "$line" in
      MATCH*) m=$((m+1));;
      "") echo "NO RESULT $(basename "$o" .out) (run with --one to see)";;
      *) echo "${line:0:220}";;
    esac
  done
  echo "== $name: $m/$n match"
done
rm -rf "$OUTD"
