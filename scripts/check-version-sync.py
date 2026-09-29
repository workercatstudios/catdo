"""Fail when the app version is not the same everywhere it is declared.

The Cargo workspace version is the source of truth. The Cargo lockfile entries
for the workspace crates and the Android default version name must match it,
so a partial bump cannot reach main or a release.
"""

import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def declared_versions() -> dict[str, str]:
    cargo = tomllib.loads((ROOT / "Cargo.toml").read_text())
    versions = {"Cargo.toml": cargo["workspace"]["package"]["version"]}
    members = {
        tomllib.loads((ROOT / member / "Cargo.toml").read_text())["package"]["name"]
        for member in cargo["workspace"]["members"]
    }
    for package in tomllib.loads((ROOT / "Cargo.lock").read_text())["package"]:
        if package["name"] in members:
            versions[f"Cargo.lock ({package['name']})"] = package["version"]
    gradle = (ROOT / "apps/android/app/build.gradle.kts").read_text()
    match = re.search(r'versionName = [^\n]*\?: "([^"]+)"', gradle)
    versions["apps/android/app/build.gradle.kts (versionName)"] = match.group(1) if match else "missing"
    return versions


if __name__ == "__main__":
    versions = declared_versions()
    expected = versions["Cargo.toml"]
    mismatched = {source: found for source, found in versions.items() if found != expected}
    if mismatched:
        print(f"Cargo.toml declares version {expected}, but these disagree:")
        for source, found in mismatched.items():
            print(f"  {source}: {found}")
        sys.exit("Bump every version together. See docs/development.md.")
    print(f"All versions agree: {expected} ({len(versions)} sources)")
