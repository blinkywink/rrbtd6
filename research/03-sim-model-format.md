# 03: `sim_model` game-data blob

## Extraction (verified)

```
python -I tools/unityfs/unityfs.py extract \
  "<game>/BloonsTD6_Data/StreamingAssets/aa/StandaloneWindows64/Full/game_model_assets_all_3ee3964e87be5bbd5deb780c8c20919a.bundle" \
  local_extracted/game_model
```

- The bundle is `UnityFS` v8, Unity 6000.3.23f1, LZ4HC, 17,079,968 bytes. It has one node, `CAB-e0d6b686af0b9f8e79ff9fb1b582e47d` (SerializedFile v22, with a type tree).
- The SerializedFile holds 2 objects: class 142 (AssetBundle) and class 49 (TextAsset) **`sim_model`**, 17,076,980 bytes.
- Byte entropy is 5.27 bits/byte over the first 1 MiB. The data is not encrypted and not compressed.
- Output goes to the git-ignored `local_extracted/`. It is proprietary data: **do not commit or redistribute.**

## Encoding (partly verified)

- Strings use .NET `BinaryWriter` encoding: a 7-bit varint length followed by UTF-8. Seen directly, for example `0x12 "DartMonkey Paragon"` and `0x0e "Improved Rangs"` **[V]**.
- There are 821 occurrences of `DartMonkey`, 6,196 of `Moab`, and 541 of `BloonModel` (type names are embedded) **[V]**.
- The first 24 int32 values, little-endian, are `354277, 9, 2168, 5, 5, 3, 10, 10, 20, 10, 10, 9411, 3, 0, 0, 0, 1, 0, 3, 1, 0, 0, 1, 0`. **[I]** 354,277 is probably the total record (object) count used by `Create_Records`, and the values after it are per-type array or set counts.

## Next steps to decode

1. Test hypothesis H-DATA-1: each `Set_v_<T>_Fields` reads fields in **metadata declaration order** of `<T>` and its base chain, with references stored as int32 indices into `m[]`. The field lists in `research/dumps/Assembly-CSharp.jsonl` give the candidate order.
2. To confirm, disassemble `GameModelLoader.Create_Records`, `Set_v_BloonModel_Fields` and `Read_a_Single_Array` in `GameAssembly.dll` (Ghidra with REA, or a fresh IL2CPP dump for the RVAs).
3. Write `tools/sim_model/decode.py`, which emits JSON of all BloonModels, RoundSets and TowerModels into `local_extracted/`.
4. Check the decoded values against values visible in-game (Red bloon speed, the round-1 composition) before marking anything verified.
