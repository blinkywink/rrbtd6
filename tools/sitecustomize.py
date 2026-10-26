# Installed into the extraction interpreter's site-packages by tools/extract_all.py.
# Loads tools/unitypy_lowmem.py into every tools process while an extraction runs.
import os as _os
import sys as _sys

_tools = _os.environ.get("RRBTD6_TOOLS")
if _tools and _os.environ.get("RRBTD6_BUNDLE_CACHE"):
    _sys.path.insert(0, _tools)
    try:
        import unitypy_lowmem  # noqa: F401
    finally:
        _sys.path.remove(_tools)
