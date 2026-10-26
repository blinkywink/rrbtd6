#!/bin/sh
# Record SimRecorder scenarios in the real game, one launch each.
#   sh tools/mod/record.sh tools/mod/scenarios/dart_200.json [...]
# Each recording lands in research/dumps/recordings/<name>.rec (and <map>.map).
set -e
GAME="${BTD6:-/c/Program Files/Epic Games/BloonsTD6}"
UD="$GAME/UserData/SimRecorder"
OUT="$(cd "$(dirname "$0")/../.." && pwd)/research/dumps/recordings"
# Launched through its store: Steam (app 960090) when GAME is the Steam install, else Epic.
case "$GAME" in
  *Steam*) URI='steam://rungameid/960090' ;;
  *) URI='com.epicgames.launcher://apps/6a8dfa6e441e4f2f9048a98776c6077d%3A49c4bf5c6fd24259b87d0bcc96b6009f%3A7786b355a13b47a6b3915335117cd0b2?action=launch&silent=true' ;;
esac
mkdir -p "$UD" "$OUT"
for sc in "$@"; do
  name=$(basename "$sc" .json)
  cp "$sc" "$UD/scenario.json"
  # The previous run's game must be gone before its log can be removed.
  tasklist | grep -qi BloonsTD6.exe && taskkill //IM BloonsTD6.exe //F >/dev/null 2>&1 || true
  for i in $(seq 1 20); do
    tasklist | grep -qi BloonsTD6.exe || break
    sleep 1
  done
  for i in $(seq 1 20); do
    rm -f "$UD/$name.rec" "$GAME/MelonLoader/Latest.log" 2>/dev/null && break
    sleep 1
  done
  sleep 2
  powershell -NoProfile -Command "Start-Process '$URI'"
  # Wait for this run's log (it names the scenario), then for its end.
  started=""
  for i in $(seq 1 60); do
    if grep -q "scenario $name:" "$GAME/MelonLoader/Latest.log" 2>/dev/null; then started=1; break; fi
    sleep 3
  done
  ok=""
  for i in $(seq 1 200); do
    [ -n "$started" ] || break
    if grep -q "SimRecorder done" "$GAME/MelonLoader/Latest.log" 2>/dev/null; then ok=1; break; fi
    tasklist | grep -qi BloonsTD6.exe || break
    sleep 3
  done
  result=$(grep "SimRecorder done" "$GAME/MelonLoader/Latest.log" | tail -1)
  if [ -n "$ok" ] && [ -f "$UD/$name.rec" ]; then
    cp "$UD/$name.rec" "$OUT/"
    cp "$UD"/*.map "$OUT/" 2>/dev/null || true
    echo "$name: $result"
  else
    echo "$name: FAILED ($result)"
    grep "SimRecorder" "$GAME/MelonLoader/Latest.log" | tail -5
  fi
done
tasklist | grep -qi BloonsTD6.exe && taskkill //IM BloonsTD6.exe //F >/dev/null 2>&1 || true
