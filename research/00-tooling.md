# 00: Analysis tooling and workflow

## Reverse Engineer Anything (REA)

- Verified repository: **https://github.com/morluto/rea**. GitHub API reports it as not a fork, MIT licence, last pushed 2026-10-21. Its README says the official channels are this repo, rea.tools, and the `rea-agents` npm package.
- Shallow-cloned at commit `52415975942a4b9c53a817915b313ccc404f280e` into `tools/external/rea` (git-ignored). It was read only; none of its scripts have been executed.
- **Installed and in use (2026-10-25, user-approved):** `npm install --global rea-agents@6.8.0` (CLI only; `rea setup` was not run, so no agent or MCP configuration changed). Node 24.20 is below REA's stated minimum (24.22.1); the commands used so far work.
- Ghidra: 12.1.4 (REA supports 12.1.x only) and 12.2, both from the official NSA GitHub releases, SHA-256 checked against the release notes, in `tools/external/ghidra/`. They need a full JDK (javac): the system Temurin is a JRE, so `JAVA_HOME` points at PyCharm's bundled JBR 25.
- REA IL2CPP workflow: `rea inspect-il2cpp-metadata <global-metadata.dat> --binary <GameAssembly.dll> --ghidra-seeds tools/external/rea_work/seeds.txt` names every method (`Type$$Method`); with `GHIDRA_INSTALL_DIR`, `JAVA_HOME` and `REA_GHIDRA_SEED_FILE` set, `rea decompile <GameAssembly.dll> 'Type$$Method' --provider ghidra --snapshot tools/external/rea_work/btd6.snapshot` decompiles (first run = full Ghidra auto-analysis).
- Quick path without auto-analysis: `python -I tools/il2cpp/decompile.py Weapons.Weapon::Process` (own Ghidra headless script; addresses from `tools/il2cpp/live_methods.py`, field offsets annotated). `research/dumps/methods-all.jsonl` is from an older build: don't use its addresses.

REA capabilities that apply to BTD6 (from its README capability table):

| REA capability | BTD6 use |
|---|---|
| IL2CPP metadata (static, versions 24/27/29/31/**39**) | Fresh type and method index of the current `global-metadata.dat` (v39). It's independent of the possibly stale MelonLoader interop assemblies. |
| .NET assemblies (static, CIL, metadata) | The interop assemblies, Btd6ModHelper |
| Native binaries via Ghidra/IDA/Hopper | Decompile `GameAssembly.dll` method bodies: `GameModelLoader`, `Bloon.Move`, `Spawner.Emit`, `SeededRandom..cctor`. Needs Ghidra (JDK 21 is already present). |
| Desktop installation inspection | Cross-check of 01-environment.md |

Not relevant: Android, web, EVM, firmware, packet captures. Network capture tooling is also deliberately unused, since there is no online-service work.

## In-repo tools (written for this project, std-only Python)

| Tool | Purpose |
|---|---|
| `tools/dotnet_meta/dotnet_meta.py` | ECMA-335 metadata reader. Dumps types, fields, properties and methods with decoded signatures, as txt and jsonl. |
| `tools/unityfs/unityfs.py` | UnityFS bundle reader with an LZ4 block decoder. Lists and extracts nodes. |

## Recommended workflow

1. **Static first.** Metadata gives names and shapes (done). The `sim_model` decode gives exact data. Native disassembly gives behaviour.
2. **Observation second.** An offline logging mod (MelonLoader) records traces from the real simulation for specific scenarios. The user launches the game. These traces become `tests/observations/*.trace`.
3. **Reimplementation.** Each mechanic gets a hypothesis ID. It moves to *verified* only when a disassembly citation or a matching observation trace exists.
