"""Release integration check: real Python setup, synthetic game, no game services.

Usage: python -I tools/tests/test_launcher_bootstrap.py <built launcher>

Windows: starts from an embeddable Python that was unpacked but never got pip (an interrupted
setup) and checks the launcher recovers it. macOS / Linux: the launcher downloads its standalone
Python itself. Either way it installs the shipped requirements from PyPI, runs a stand-in
extraction that proves the imports work, and a second launch reuses everything. Kept separate from
offline unit tests because this check intentionally needs the network.
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
    windows = sys.platform == "win32"
    source = Path(__file__).resolve().parents[2]
    launcher = Path(sys.argv[1]).resolve(strict=True)
    version = re.search(r'const PYTHON_VERSION: &str = "([\d.]+)"',
                        (source / "launcher/src/main.rs").read_text()).group(1)
    with tempfile.TemporaryDirectory(prefix="rrbtd6-embedded-test-") as folder:
        root = Path(folder).resolve()
        name = "rrbtd6-launcher.exe" if windows else "rrbtd6-launcher"
        shutil.copyfile(launcher, root / name)
        (root / name).chmod(0o755)
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
        if windows:
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
            result = subprocess.run([str(root / name), "--offline", "--unattended"],
                                    input="", capture_output=True, text=True, encoding="utf-8",
                                    errors="replace", timeout=900, cwd=root)
            output = result.stdout + result.stderr
            # "exit code: 73" on Windows, "exit status: 73" elsewhere.
            if result.returncode != 1 or "EMBEDDED_EXTRACTION_READY" not in output or not re.search(r"exit (code|status): 73", output):
                raise AssertionError(f"Bootstrap attempt {attempt + 1} failed:\n{output}")
            assert (python / "requirements.installed").read_bytes() == requirements
            if attempt == 1:
                assert "downloading" not in output and "Installing setup packages" not in output, output
        print("Python set up, full requirements imported, second launch reused setup.")


if __name__ == "__main__":
    main()
