"""Extraction bootstrap checks; no UnityPy install, assets or network needed."""
import os
import platform
from pathlib import Path
import runpy
import sys
import types
import unittest
from unittest.mock import patch

HOOK = Path(__file__).with_name("sitecustomize.py")
ENV = {"RRBTD6_TOOLS": str(HOOK.parent), "RRBTD6_BUNDLE_CACHE": "unused-test-cache"}


class BootstrapTests(unittest.TestCase):
    @unittest.skipUnless(sys.platform == "win32" and hasattr(platform, "_wmi"), "Windows WMI fallback")
    def test_native_platform_fallback_never_queries_wmi(self):
        class UnresponsiveWmi:
            def exec_query(self, query):
                raise AssertionError("WMI must not be contacted by extraction")

        before = sys.path[:]
        with patch.dict(os.environ, ENV), patch.dict(sys.modules, {"unitypy_lowmem": types.ModuleType("unitypy_lowmem")}), \
                patch.object(platform, "_wmi", UnresponsiveWmi()), patch.object(platform, "_uname_cache", None), \
                patch.object(sys, "path", sys.path[:]):
            runpy.run_path(str(HOOK))
            self.assertIsNone(platform._wmi)
            self.assertEqual(platform.system(), "Windows")
            self.assertTrue(platform.machine())
            self.assertTrue(platform.win32_ver()[1])
            self.assertEqual(sys.path[0], str(HOOK.parent / "assets"))
        self.assertEqual(sys.path, before)

    def test_isolated_helpers_are_available_without_adding_working_directory(self):
        before = sys.path[:]
        with patch.dict(os.environ, ENV), patch.dict(sys.modules, {"unitypy_lowmem": types.ModuleType("unitypy_lowmem")}), \
                patch.object(sys, "path", sys.path[:]), \
                patch.object(platform, "_wmi", getattr(platform, "_wmi", None), create=True):
            runpy.run_path(str(HOOK))
            runpy.run_path(str(HOOK))
            self.assertEqual(sys.path, [str(HOOK.parent / "assets"), *before])
        self.assertEqual(sys.path, before)

    def test_unrelated_python_process_is_untouched(self):
        before = sys.path[:]
        old_wmi = getattr(platform, "_wmi", None)
        with patch.dict(os.environ, {}, clear=True), patch.dict(sys.modules, {"unitypy_lowmem": None}):
            runpy.run_path(str(HOOK))
        self.assertIs(getattr(platform, "_wmi", None), old_wmi)
        self.assertEqual(sys.path, before)

    def test_search_path_restored_when_loader_import_fails(self):
        before = sys.path[:]
        with patch.dict(os.environ, ENV), patch.dict(sys.modules, {"unitypy_lowmem": None}):
            # Keep this test independent of the host platform's WMI state.
            with patch.object(platform, "_wmi", getattr(platform, "_wmi", None), create=True):
                with self.assertRaises(ModuleNotFoundError):
                    runpy.run_path(str(HOOK))
        self.assertEqual(sys.path, before)


if __name__ == "__main__":
    unittest.main()
