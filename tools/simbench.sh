#!/usr/bin/env sh
# Five sim_bench runs of a test exe; prints the sorted step averages (ms).
exe="${1:-$(ls -t target/check-dbg/release/deps/btd6_game-*.exe | head -1)}"
for i in 1 2 3 4 5; do "$exe" sim_bench --ignored --nocapture 2>&1 | grep -o "step avg [0-9.]* ms" | awk '{print $3}'; done | sort -n | tr '\n' ' '; echo
