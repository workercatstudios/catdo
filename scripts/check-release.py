"""Check tag validation and smoke-test an extracted release without user data."""

import argparse
import importlib.util
import os
from pathlib import Path
import struct
import subprocess
import tarfile
import tempfile
import tomllib

root = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location("release_metadata", root / "scripts/release-metadata.py")
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

assert module.metadata("0.1.0", "v0.1.0") == ("0.1.0", False)
assert module.metadata("0.2.0-rc.1", "v0.2.0-rc.1") == ("0.2.0-rc.1", True)
assert module.metadata("0.1.0") == ("0.1.0", False)
for version, tag in [("0.1.0", "v0.2.0"), ("0.1.0", "0.1.0"),
                     ("../escape", ""), ("0.1.0\ninvalid", ""), ("01.0.0", ""),
                     ("0.1.0-01", ""), ("0.1.0+build", "")]:
    try:
        module.metadata(version, tag)
    except ValueError:
        pass
    else:
        raise AssertionError(f"Accepted invalid release: {version!r}, {tag!r}")

icon = (root / "assets/com.workercat.catdo.png").read_bytes()
assert icon[:8] == b"\x89PNG\r\n\x1a\n" and struct.unpack(">II", icon[16:24]) == (512, 512)
print("Release tag and icon checks passed.")

parser = argparse.ArgumentParser()
parser.add_argument("--archive", type=Path)
args = parser.parse_args()
if args.archive:
    with tempfile.TemporaryDirectory(prefix="catdo release check ") as directory:
        temp = Path(directory)
        with tarfile.open(args.archive) as archive:
            archive.extractall(temp, filter="data")
        package, = temp.iterdir()
        for name in ["catdo", "LICENSE.md", "NOTICE", "README.md", "licenses/README.md", "assets/com.workercat.catdo.png",
                     "packaging/com.workercat.catdo.desktop", "scripts/install-desktop.sh"]:
            assert (package / name).is_file(), f"Missing release file: {name}"
        output = subprocess.check_output([str(package / "catdo"), "--help"], text=True)
        assert "Usage: catdo" in output
        version = tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"]["version"]
        assert subprocess.check_output([str(package / "catdo"), "--version"], text=True).strip() == f"CatDo {version}"
        env = os.environ | {"XDG_BIN_HOME": str(temp / "bin space"), "XDG_DATA_HOME": str(temp / "data space")}
        task = Path(env["XDG_DATA_HOME"]) / "catdo/catdo.sqlite3"
        task.parent.mkdir(parents=True)
        task.write_bytes(b"existing task data sentinel")
        subprocess.run(["bash", str(package / "scripts/install-desktop.sh"), "--prebuilt"], env=env, check=True)
        binary = Path(env["XDG_BIN_HOME"]) / "catdo"
        assert binary.is_file() and os.access(binary, os.X_OK)
        assert task.read_bytes() == b"existing task data sentinel"
        launcher = Path(env["XDG_DATA_HOME"]) / "applications/com.workercat.catdo.desktop"
        assert f'Exec="{binary}"' in launcher.read_text()
        assert (Path(env["XDG_DATA_HOME"]) / "catdo/brand/icon.png").read_bytes() == icon
        print("Archive, binary, launcher, icon, and preservation of existing task data passed.")
