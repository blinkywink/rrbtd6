#!/bin/bash
# Cash check over recordings: runs each with RRBTD6_REC_CASH and prints, per recording, the sim
# result (MATCH / DIFF) and its first cash difference against the F lines (tools/rec/cashcmp.py).
#   sh tools/rec/cash_sweep.sh 'pending/village/*.rec' dart_002.rec ...   (paths or quoted globs
#   under research/dumps/recordings/)
set -u
cd "$(dirname "$0")/../../renderer"
REC="$(cd .. && pwd)/research/dumps/recordings"
REC_W="$(cygpath -m "$REC" 2>/dev/null || echo "$REC")"
TOOL="$(cygpath -m "$(cd .. && pwd)/tools/rec/cashcmp.py" 2>/dev/null || echo "$(cd .. && pwd)/tools/rec/cashcmp.py")"
EXE=$(CARGO_PROFILE_RELEASE_DEBUG=0 CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16 RUST_MIN_STACK=268435456 \
  CARGO_TARGET_DIR=../target/check-lite cargo test -j 2 -p btd6-game --release --no-run --message-format=json 2>/dev/null \
  | grep '"executable":"[^"]' | grep '"name":"btd6-game"' | grep -o '"executable":"[^"]*"' | tail -1 | cut -d'"' -f4)
[ -n "$EXE" ] || { echo "error: the test build failed"; exit 1; }
OUTD=$(mktemp -d)
(cd "$REC" && for p in "$@"; do ls $p; done) | xargs -P "${RRBTD6_JOBS:-8}" -I{} bash -c '
  o="$3/$(echo {} | tr / _).out"
  RRBTD6_REC_CASH=1 RUST_MIN_STACK=268435456 RRBTD6_REC="$2/{}" "$1" compare_recording --ignored --nocapture > "$o" 2>&1
  r=$(grep -E "^(MATCH|DIFF)" "$o" | head -1 | cut -c1-5)
  python -I "$4" "$2/{}" "$o" | head -1 | sed "s|^|$r {}: |"' _ "$EXE" "$REC_W" "$OUTD" "$TOOL"
rm -rf "$OUTD"
