"""Use the matching client to store legacy floats in its exact runtime format.

The client handles conversion before creating a window. Running this through
the client ensures the quantizer's f32 arithmetic/rounding matches rendering.
"""
from pathlib import Path
import subprocess
import sys


def run(root, models):
    root = Path(root).resolve(strict=True)
    models = Path(models).resolve(strict=True)
    executable = next((root / n for n in ("rrbtd6.exe", "BTD6.exe") if (root / n).is_file()), None)
    if executable is None:
        raise FileNotFoundError("A matching RustTD6 client is required for animation conversion")
    subprocess.run([str(executable), "--canonicalize-anims", str(models)], check=True, cwd=root)


if __name__ == "__main__":
    run(sys.argv[1], sys.argv[2])
