"""Include dependency license files and an inventory alongside desktop binaries."""

import json
from pathlib import Path
import shutil
import subprocess
import sys

destination = Path(sys.argv[1])
destination.mkdir(parents=True, exist_ok=True)
metadata = json.loads(subprocess.check_output([
    "cargo", "metadata", "--locked", "--format-version", "1",
    "--filter-platform", "x86_64-unknown-linux-gnu",
]))
lines = ["# Third-party Rust dependencies", "",
         "Dependencies retain their own licenses. This inventory includes build and test dependencies.",
         "License and notice files supplied in the published crates are included below.",
         "Upstream links identify the source when a crate omits a license file from its archive.", ""]
for package in sorted(metadata["packages"], key=lambda p: (p["name"], p["version"])):
    if not package["source"]:
        continue
    name = f'{package["name"]}-{package["version"]}'
    root = Path(package["manifest_path"]).parent
    files = set()
    if package["license_file"]:
        files.add(root / package["license_file"])
    # Include nested notices too, such as bundled fonts and vendored libraries.
    for path in root.rglob("*"):
        if path.is_file() and path.name.lower().startswith(("license", "licence", "copying", "notice", "copyright", "ofl")):
            files.add(path)
    for path in sorted(files):
        if not path.is_file():
            continue
        target = destination / name / path.relative_to(root)
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(path, target)
    upstream = package["repository"] or f'https://crates.io/crates/{package["name"]}/{package["version"]}'
    license_name = package["license"] or "See supplied license file"
    lines.append(f'- [{name}]({upstream}): {license_name}')
(destination / "README.md").write_text("\n".join(lines) + "\n")
