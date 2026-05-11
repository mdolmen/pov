"""Build standalone binaries for the Tauri bundle.

Produces two PyInstaller --onefile binaries, named with the Rust target triple
so Tauri can find them as sidecars:

  src-tauri/binaries/pov-backend-<triple>   (FastAPI server)
  src-tauri/binaries/pov-<triple>           (CLI tool)
"""

import subprocess
import sys
from pathlib import Path


def _target_triple() -> str:
    output = subprocess.check_output(["rustc", "-vV"], text=True)
    for line in output.splitlines():
        if line.startswith("host:"):
            return line.split(":", 1)[1].strip()
    raise RuntimeError("could not determine Rust target triple from `rustc -vV`")


def _build(name: str, script: Path, out_dir: Path) -> None:
    triple = _target_triple()
    dest = out_dir / f"{name}-{triple}"

    print(f"building {dest.name} ...", flush=True)
    subprocess.run(
        [
            sys.executable, "-m", "PyInstaller",
            "--onefile",
            "--name", name,
            "--distpath", str(out_dir),
            # uvicorn and watchdog use dynamic imports
            "--collect-all", "uvicorn",
            "--collect-all", "watchdog",
            # clean build artefacts out of source tree
            "--workpath", str(out_dir / "_build"),
            "--specpath", str(out_dir / "_build"),
            "--noconfirm",
            str(script),
        ],
        check=True,
    )

    tmp = out_dir / name
    if dest.exists():
        dest.unlink()
    tmp.rename(dest)
    print(f"  → {dest}", flush=True)


def main() -> None:
    backend_dir = Path(__file__).parent
    out_dir = backend_dir.parent / "src-tauri" / "binaries"
    out_dir.mkdir(exist_ok=True)

    _build("pov-backend", backend_dir / "main.py", out_dir)
    # Named pov-cli (not pov) to avoid conflicting with the Cargo package name.
    # install_cli() copies it to ~/.local/bin/pov at install time.
    _build("pov-cli", backend_dir / "pov" / "cli.py", out_dir)


if __name__ == "__main__":
    main()
