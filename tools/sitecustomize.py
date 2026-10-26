# Installed into the extraction interpreter's site-packages by tools/extract_all.py.
# Loads tools/unitypy_lowmem.py into every tools process while an extraction runs.
import os as _os
import sys as _sys

_tools = _os.environ.get("RRBTD6_TOOLS")
if _tools and _os.environ.get("RRBTD6_BUNDLE_CACHE"):
    if _sys.platform == "win32":
        # Python 3.13's platform.system()/machine() ask WMI for OS details.
        # fmod_toolkit invokes these even when UnityPy only exports a texture.
        # An unresponsive WMI service otherwise blocks every exporter forever.
        # Use platform's own native Windows fallback in extraction processes.
        import platform as _platform
        if hasattr(_platform, "_wmi"):
            _platform._wmi = None
    _sys.path.insert(0, _tools)
    try:
        import unitypy_lowmem  # noqa: F401
    finally:
        _sys.path.remove(_tools)
    # -I and the embeddable Python ignore the script directory. Exporters share
    # helpers in this trusted shipped folder; never add the working directory.
    _assets = _os.path.join(_tools, "assets")
    if _assets not in _sys.path:
        _sys.path.insert(0, _assets)
