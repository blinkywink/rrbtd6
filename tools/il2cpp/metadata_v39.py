"""Static reader for IL2CPP global-metadata.dat version 39 (Unity 6000.3).

Std-only and read-only. The layouts below were inferred from this file
(sizes divide exactly by section counts) and are validated at load time:
names must decode, and method/field ranges must tile their sections.

Usage:
    python -I metadata_v39.py <global-metadata.dat> <out.jsonl>
"""
import json
import struct
import sys

SECTIONS = [
    "stringLiteral", "stringLiteralData", "string", "events", "properties", "methods",
    "parameterDefaultValues", "fieldDefaultValues", "fieldAndParameterDefaultValueData",
    "fieldMarshaledSizes", "parameters", "fields", "genericParameters",
    "genericParameterConstraints", "genericContainers", "nestedTypes", "interfaces",
    "vtableMethods", "interfaceOffsets", "typeDefinitions", "images", "assemblies",
    "fieldRefs", "referencedAssemblies", "attributeData", "attributeDataRange",
    "unresolvedIndirectCallParameterTypes", "unresolvedIndirectCallParameterRanges",
    "windowsRuntimeTypeNames", "windowsRuntimeStrings", "exportedTypeDefinitions",
]

# Il2CppTypeDefinition, 82 bytes.
TYPEDEF = struct.Struct("<5iH9i8H2I")
TYPEDEF_FIELDS = ["nameIndex", "namespaceIndex", "byvalTypeIndex", "declaringTypeIndex", "parentIndex",
                  "genericContainerIndex", "flags", "fieldStart", "methodStart", "eventStart",
                  "propertyStart", "nestedTypesStart", "interfacesStart", "vtableStart",
                  "interfaceOffsetsStart", "method_count", "property_count", "field_count",
                  "event_count", "nested_type_count", "vtable_count", "interfaces_count",
                  "interface_offsets_count", "bitfield", "token"]
# Il2CppMethodDefinition, 32 bytes.
METHOD = struct.Struct("<IHIIIHI4H")
METHOD_FIELDS = ["nameIndex", "declaringType", "returnType", "returnParameterToken", "parameterStart",
                 "genericContainerIndex", "token", "flags", "iflags", "slot", "parameterCount"]
# Il2CppImageDefinition, 36 bytes. typeStart/typeCount are 16-bit in v39
# (validated: each image's typeStart == previous typeStart + typeCount).
IMAGE = struct.Struct("<IIHHHHIiIII")
IMAGE_FIELDS = ["nameIndex", "assemblyIndex", "typeStart", "typeCount", "exportedTypeStart",
                "exportedTypeCount", "unknown0", "entryPointIndex", "token",
                "customAttributeStart", "customAttributeCount"]


class Metadata:
    def __init__(self, path):
        d = self.d = open(path, "rb").read()
        magic, ver = struct.unpack_from("<Ii", d, 0)
        assert magic == 0xFAB11BAF and ver == 39, (hex(magic), ver)
        self.sec = {}
        for i, name in enumerate(SECTIONS):
            self.sec[name] = struct.unpack_from("<3i", d, 8 + 12 * i)
        self.types = self._table("typeDefinitions", TYPEDEF, TYPEDEF_FIELDS)
        self.methods = self._table("methods", METHOD, METHOD_FIELDS)
        self.images = self._table("images", IMAGE, IMAGE_FIELDS)
        self._validate()

    def _table(self, name, st, fields):
        off, size, count = self.sec[name]
        assert size == st.size * count, (name, size, st.size, count)
        return [dict(zip(fields, st.unpack_from(self.d, off + i * st.size))) for i in range(count)]

    def string(self, idx):
        off = self.sec["string"][0] + idx
        return self.d[off:self.d.index(b"\0", off)].decode("utf-8", "replace")

    def _validate(self):
        nm = len(self.methods)
        for t in self.types:
            assert t["methodStart"] == -1 or 0 <= t["methodStart"] + t["method_count"] <= nm
        for a, b in zip(self.images, self.images[1:]):
            assert b["typeStart"] == a["typeStart"] + a["typeCount"], (a, b)
        for m in self.methods[:1000]:
            assert 0 < m["token"] >> 24 == 0x06, hex(m["token"])

    def type_name(self, ti):
        t = self.types[ti]
        name = self.string(t["nameIndex"])
        if t["declaringTypeIndex"] != -1 and t["bitfield"] & 0:  # nested info not needed yet
            pass
        ns = self.string(t["namespaceIndex"])
        return f"{ns}.{name}" if ns else name

    def iter_methods(self):
        """Yields (image, type full name, method name, token, flags, slot)."""
        for img in self.images:
            iname = self.string(img["nameIndex"])
            for ti in range(img["typeStart"], img["typeStart"] + img["typeCount"]):
                t = self.types[ti]
                tn = self.type_name(ti)
                if t["methodStart"] == -1:
                    continue
                for mi in range(t["methodStart"], t["methodStart"] + t["method_count"]):
                    m = self.methods[mi]
                    yield iname, tn, self.string(m["nameIndex"]), m["token"], m["flags"], m["slot"], m["parameterCount"]


def main():
    md = Metadata(sys.argv[1])
    print({k: v[2] for k, v in md.sec.items() if k in ("typeDefinitions", "methods", "images")})
    n = 0
    with open(sys.argv[2], "w", encoding="utf-8") as f:
        for row in md.iter_methods():
            f.write(json.dumps(dict(zip(["image", "type", "method", "token", "flags", "slot", "nparams"], row))) + "\n")
            n += 1
    print(n, "methods written")


if __name__ == "__main__":
    main()
