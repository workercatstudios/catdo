"""Validate release tags against the version embedded in the desktop build."""

import re
import sys
import tomllib
from pathlib import Path


def metadata(version: str, tag: str = "") -> tuple[str, bool]:
    number = r"(?:0|[1-9][0-9]*)"
    identifier = rf"(?:{number}|[0-9]*[A-Za-z-][0-9A-Za-z-]*)"
    if not re.fullmatch(rf"{number}\.{number}\.{number}(?:-{identifier}(?:\.{identifier})*)?", version):
        raise ValueError("Use a Cargo version such as 0.1.0 or 0.2.0-rc.1 (without build metadata).")
    if tag and tag != f"v{version}":
        raise ValueError(f"Tag {tag!r} must match the Cargo workspace version: v{version}")
    return version, "-" in version


if __name__ == "__main__":
    root = Path(__file__).resolve().parent.parent
    version = tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"]["version"]
    try:
        version, prerelease = metadata(version, sys.argv[1] if len(sys.argv) > 1 else "")
    except ValueError as error:
        sys.exit(str(error))
    print(f"version={version}")
    print(f"prerelease={str(prerelease).lower()}")
