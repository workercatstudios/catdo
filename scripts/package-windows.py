"""Package and smoke-test the Windows x86-64 desktop after a release build.

Produces the standalone executable used by the in-app updater, a zip with
license notices, and SHA-256 checksum files in dist/.
"""

import hashlib
import importlib.util
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import tempfile
import tomllib
import zipfile

root = Path(__file__).resolve().parent.parent
if sys.platform != "win32" or platform.machine().lower() not in ("amd64", "x86_64"):
    sys.exit("This package currently supports Windows x86-64 only.")
spec = importlib.util.spec_from_file_location("release_metadata", root / "scripts/release-metadata.py")
metadata = importlib.util.module_from_spec(spec)
spec.loader.exec_module(metadata)
version, _ = metadata.metadata(tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"]["version"])
name = f"catdo-{version}-windows-x86_64"
binary = root / "target/release/catdo.exe"

# Release builds have no console window, but still print to redirected output.
assert "Usage: catdo" in subprocess.check_output([binary, "--help"], text=True)
assert subprocess.check_output([binary, "--version"], text=True).strip() == f"CatDo {version}"

dist = root / "dist"
dist.mkdir(exist_ok=True)
shutil.copyfile(binary, dist / f"{name}.exe")
with tempfile.TemporaryDirectory(prefix="catdo package ") as directory:
    package = Path(directory) / name
    package.mkdir()
    shutil.copyfile(binary, package / "catdo.exe")
    shutil.copyfile(root / "packaging/README-windows.md", package / "README.md")
    for file in ["LICENSE.md", "NOTICE"]:
        shutil.copyfile(root / file, package / file)
    subprocess.run([sys.executable, root / "scripts/collect-licenses.py", package / "licenses", "x86_64-pc-windows-msvc"],
                   check=True)
    with zipfile.ZipFile(dist / f"{name}.zip", "w", zipfile.ZIP_DEFLATED) as archive:
        for path in sorted(package.rglob("*")):
            archive.write(path, path.relative_to(package.parent))

for asset in [f"{name}.exe", f"{name}.zip"]:
    digest = hashlib.sha256((dist / asset).read_bytes()).hexdigest()
    (dist / f"{asset}.sha256").write_text(f"{digest}  {asset}\n", newline="\n")
with zipfile.ZipFile(dist / f"{name}.zip") as archive:
    for file in ["catdo.exe", "README.md", "LICENSE.md", "NOTICE", "licenses/README.md"]:
        assert f"{name}/{file}" in archive.namelist(), f"Missing release file: {file}"
print(f"Packaged {name}.exe and {name}.zip")
