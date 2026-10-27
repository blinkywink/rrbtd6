"""Windows release integration check: real embedded Python, synthetic game, no game services.

Usage: python -I tools/test_launcher_bootstrap.py <built launcher.exe>
Downloads Python and installs the shipped requirements from PyPI. Kept separate
from offline unit tests because this check intentionally needs the network.
"""
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import urllib.request
import zipfile


def main():
    if sys.platform != "win32":
        raise SystemExit("This integration check requires Windows")
    source = Path(__file__).resolve().parents[1]
    launcher = Path(sys.argv[1]).resolve(strict=True)
    version = re.search(r'const PYTHON_VERSION: &str = "([\d.]+)"',
                        (source / "launcher/src/main.rs").read_text()).group(1)
    with tempfile.TemporaryDirectory(prefix="rrbtd6-embedded-test-") as folder:
        root = Path(folder).resolve()
        shutil.copyfile(launcher, root / "rrbtd6-launcher.exe")
        game = root / "synthetic-game"
        game.mkdir()
        (game / "GameAssembly.dll").write_bytes(b"synthetic fixture; never loaded")
        (root / "rrbtd6.json").write_text(json.dumps({"game": str(game)}))
        tools = root / "tools"
        tools.mkdir()
        requirements = (source / "tools/requirements.txt").read_bytes()
        (tools / "requirements.txt").write_bytes(requirements)
        # Stop after proving imports work, before the launcher can start a game.
        (tools / "extract_all.py").write_text(
            'import UnityPy, PIL, numpy, lz4, capstone, tpk_ar\n'
            'print("EMBEDDED_EXTRACTION_READY", flush=True)\n'
            'raise SystemExit(73)\n', encoding="utf-8")
        python = root / "python"
        python.mkdir()
        archive = root / "python.zip"
        urllib.request.urlretrieve(
            f"https://www.python.org/ftp/python/{version}/python-{version}-embed-amd64.zip", archive)
        with zipfile.ZipFile(archive) as bundle:
            bundle.extractall(python)
        for path in python.glob("*._pth"):
            path.write_text(path.read_text().replace("#import site", "import site"))
        # Deliberately interrupted setup: runtime unpacked, pip not yet installed.
        for attempt in range(2):
            result = subprocess.run([str(root / "rrbtd6-launcher.exe"), "--offline"],
                                    input="", capture_output=True, text=True, encoding="utf-8",
                                    errors="replace", timeout=300, cwd=root)
            output = result.stdout + result.stderr
            if result.returncode != 1 or "EMBEDDED_EXTRACTION_READY" not in output or "exit code: 73" not in output:
                raise AssertionError(f"Bootstrap attempt {attempt + 1} failed:\n{output}")
            assert (python / "requirements.installed").read_bytes() == requirements
            if attempt == 1:
                assert "downloading" not in output and "Installing extraction packages" not in output, output
        print("Embedded Python recovered, full requirements imported, second launch reused setup.")


if __name__ == "__main__":
    main()
