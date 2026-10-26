"""The launcher's isolated Python must still resolve bundled shader helpers."""
from contextlib import redirect_stdout
import io
from pathlib import Path
import runpy
import sys
import tempfile
import types
import unittest
from unittest.mock import patch


class ShaderImportTests(unittest.TestCase):
    def test_rebuilder_imports_its_sibling_in_isolated_mode(self):
        script = Path(__file__).parent / "assets/rebuild_shader_glsl.py"
        lz4 = types.ModuleType("lz4")
        lz4.__path__ = []
        # No shader inputs: compression is never used, so CI needs no game data
        # or extraction packages to verify the real sibling-import path.
        with tempfile.TemporaryDirectory(prefix="rrbtd6-shader-import-") as folder, \
                patch.object(sys, "argv", [str(script), folder]), patch.object(sys, "path", sys.path[:]), \
                patch.dict(sys.modules, {"lz4": lz4, "lz4.block": types.ModuleType("lz4.block")}), \
                redirect_stdout(io.StringIO()) as output:
            runpy.run_path(str(script), run_name="__main__")
            self.assertEqual(Path(sys.modules["dxbc_glsl"].__file__).resolve(), script.with_name("dxbc_glsl.py").resolve())
            self.assertIn("Rebuilt 0 local shaders", output.getvalue())


if __name__ == "__main__":
    unittest.main()
