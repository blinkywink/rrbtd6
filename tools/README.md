# Tools

All tools are std-only Python 3. Run them with `python -I` (isolated mode). They only read the game install and write into this repository.

| Tool | Command |
|---|---|
| .NET metadata dump | `python -I tools/dotnet_meta/dotnet_meta.py <asm.dll> research/dumps/<name>.txt --json research/dumps/<name>.jsonl` |
| UnityFS bundle | `python -I tools/unityfs/unityfs.py list <bundle>` / `extract <bundle> local_extracted/<dir>` |

`external/` holds third-party repositories cloned for review (git-ignored). See `research/00-tooling.md`.
