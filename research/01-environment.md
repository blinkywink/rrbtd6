# 01: Installation and runtime environment

Recorded 2026-10-21. Everything here was verified by direct inspection unless it is marked otherwise. The game install was only read, never modified.

## Install

| Item | Value | Evidence |
|---|---|---|
| Location | `C:\Program Files\Epic Games\BloonsTD6` | Epic manifest `C:\ProgramData\Epic\EpicGamesLauncher\Data\Manifests\*.item` |
| Build | `57.2.12199` (manifest `AppVersionString`) | same |
| Launch exe | `BloonsTD6.exe` (667 KB Unity stub) | manifest `LaunchExecutable` |
| Product | `Ninja Kiwi` / `BloonsTD6-Epic` | `BloonsTD6_Data/app.info` |
| Engine | Unity **6000.3.23f1** | UnityPlayer.dll version `6000.3.23.643820`; bundle headers; MelonLoader log |
| Runtime | **IL2CPP x64** | `GameAssembly.dll` (93 MB), `il2cpp_data/Metadata/global-metadata.dat` (22 MB), no `Managed/` folder |
| IL2CPP metadata | magic `0xFAB11BAF`, **version 39** | first 8 bytes of `global-metadata.dat` |
| Render | URP, with a D3D12 folder present | `Unity.RenderPipelines.Universal.Runtime` interop assembly, `D3D12/` |
| Assets | Addressables: `StreamingAssets/aa/StandaloneWindows64/{Full,Half}/*.bundle` (125 bundles in Full) | directory listing |
| Bundle format | `UnityFS` v8, LZ4HC (flag 3), no encryption seen | header parse (`tools/unityfs`) |

## Pre-existing modding setup (installed by the user, not by this project)

- MelonLoader v0.7.3 Open-Beta (`version.dll` proxy, `MelonLoader/`), running on the net6 runtime.
- `Mods/Btd6ModHelper.dll`, `Mods/EveryBloonIsRed.dll`, `Plugins/BTD6EpicGamesModCompat.dll`, `Plugins/UpdaterPlugin.dll`.
- `MelonLoader/Il2CppAssemblies/` holds 154 Il2CppInterop proxy assemblies, among them `Assembly-CSharp.dll` (39 MB).

**Staleness caveat:** the interop assemblies are dated 2026-10-15. `GameAssembly.dll` is dated 2026-10-19. Type and member *names and shapes* very probably still match, but field offsets and method RVAs may not. Before relying on addresses, re-check against a fresh dump of `global-metadata.dat` (REA's static decoder supports v39).

This project does **not** use or analyse `BTD6EpicGamesModCompat.dll`. Its purpose relates to running mods on the Epic build, which is outside this project's scope (no DRM or platform circumvention).

## Toolchain on this machine

| Tool | Status |
|---|---|
| Rust | installed 2026-10-21 via `winget Rustlang.Rustup`: rustc/cargo **1.99.1**, `stable-x86_64-pc-windows-msvc` |
| MSVC | VS 2022 Community, VC.Tools.x86.x64 |
| Python | 3.13 (used for the dependency-free tools in `tools/`) |
| Node / npm | 24.20.0 / 11.19.0, which meets REA's requirement (>=24.11) |
| Java | OpenJDK 21.0.9, enough for Ghidra (Ghidra itself is not installed) |
| .NET | runtime only (6.0.36 hostfxr), **no SDK** |
| Git | present |
