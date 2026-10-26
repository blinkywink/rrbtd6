#!/bin/sh
# Build tools/mod/SimRecorder into <game>/Mods/SimRecorder.dll with Visual Studio's csc against
# the .NET 6 runtime, MelonLoader and the game's interop assemblies (no .NET SDK needed).
set -e
GAME="${BTD6:-/c/Program Files/Epic Games/BloonsTD6}"
CSC="/c/Program Files/Microsoft Visual Studio/2022/Community/MSBuild/Current/Bin/Roslyn/csc.exe"
NET6=$(ls -d "/c/Program Files/dotnet/shared/Microsoft.NETCore.App/6.0."* | sort -V | tail -1)
HERE=$(cd "$(dirname "$0")" && pwd)
OUT="$HERE/bin"
mkdir -p "$OUT"
RSP="$OUT/refs.rsp"
: > "$RSP"
for f in "$NET6"/*.dll "$GAME"/MelonLoader/net6/*.dll "$GAME"/MelonLoader/Il2CppAssemblies/*.dll; do
  case "$f" in "$NET6"/*) case "$(basename "$f")" in System.*|Microsoft.*|netstandard.dll|mscorlib.dll) ;; *) continue ;; esac ;; esac
  case "$(basename "$f")" in
    *Native*|clrjit.dll|coreclr.dll|hostpolicy.dll|mscordaccore*|mscordbi.dll|mscorrc.dll|clretwrc.dll|msquic.dll|Microsoft.DiaSymReader*|createdump*) continue ;;
  esac
  echo "-r:\"$(cygpath -w "$f")\"" >> "$RSP"
done
"$CSC" -nologo -noconfig -nostdlib -target:library -langversion:10 -nullable:disable -optimize+ \
  -out:"$(cygpath -w "$OUT/SimRecorder.dll")" @"$(cygpath -w "$RSP")" "$(cygpath -w "$HERE/SimRecorder/SimRecorder.cs")"
cp "$OUT/SimRecorder.dll" "$GAME/Mods/SimRecorder.dll"
echo "built -> $GAME/Mods/SimRecorder.dll"
