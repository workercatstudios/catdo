#!/usr/bin/env bash
set -euo pipefail

catdo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$catdo_root"
if [[ "$(uname -s)" != Linux || "$(uname -m)" != x86_64 ]]; then
    printf 'AppImage packaging currently supports Linux x86-64 only.\n' >&2
    exit 1
fi
catdo_metadata="$(python3 scripts/release-metadata.py)"
catdo_version="$(sed -n 's/^version=//p' <<< "$catdo_metadata")"
catdo_tools="$catdo_root/.local/release-tools"
catdo_linuxdeploy="$catdo_tools/linuxdeploy-x86_64.AppImage"
mkdir -p "$catdo_tools" dist
# Pin a named upstream release and verify it before executing any packaging code.
if [[ ! -f "$catdo_linuxdeploy" ]]; then
    curl --fail --location --retry 3 \
      https://github.com/linuxdeploy/linuxdeploy/releases/download/1-alpha-20251107-1/linuxdeploy-x86_64.AppImage \
      --output "$catdo_linuxdeploy"
fi
printf '%s  %s\n' c20cd71e3a4e3b80c3483cef793cda3f4e990aca14014d23c544ca3ce1270b4d "$catdo_linuxdeploy" | sha256sum --check
chmod +x "$catdo_linuxdeploy"
catdo_runtime="$catdo_tools/runtime-x86_64"
if [[ ! -f "$catdo_runtime" ]]; then
    curl --fail --location --retry 3 \
      https://github.com/AppImage/type2-runtime/releases/download/20251108/runtime-x86_64 \
      --output "$catdo_runtime"
fi
printf '%s  %s\n' 2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d "$catdo_runtime" | sha256sum --check
catdo_stage="$(mktemp -d)"
trap 'rm -rf -- "$catdo_stage"' EXIT
catdo_appdir="$catdo_stage/CatDo.AppDir"
install -Dm644 LICENSE.md "$catdo_appdir/usr/share/doc/catdo/LICENSE.md"
install -Dm644 NOTICE "$catdo_appdir/usr/share/doc/catdo/NOTICE"
install -Dm644 packaging/README.md "$catdo_appdir/usr/share/doc/catdo/README.md"
python3 scripts/collect-licenses.py "$catdo_appdir/usr/share/doc/catdo/licenses"
# Preserve distro copyright notices for shared libraries bundled by linuxdeploy.
if command -v dpkg-query >/dev/null; then
    mkdir -p "$catdo_appdir/usr/share/doc/catdo/system-libraries"
    while read -r catdo_library; do
        catdo_owner="$(dpkg-query --search "$catdo_library" 2>/dev/null | head -1 | cut -d: -f1 || true)"
        if [[ -n "$catdo_owner" && -f "/usr/share/doc/$catdo_owner/copyright" ]]; then
            cp "/usr/share/doc/$catdo_owner/copyright" "$catdo_appdir/usr/share/doc/catdo/system-libraries/$catdo_owner.txt"
        fi
    done < <(ldd target/release/catdo | awk '/=> \// {print $3}')
fi
# Dlopen-loaded graphics drivers and Fontconfig remain supplied by the host.
export APPIMAGE_EXTRACT_AND_RUN=1
export NO_STRIP=1
# The distro patchelf handles modern ELF binaries correctly.
export PATCHELF="$(command -v patchelf)"
export LDAI_VERSION="$catdo_version"
export LDAI_RUNTIME_FILE="$catdo_runtime"
export LDAI_OUTPUT="$catdo_root/dist/catdo-$catdo_version-linux-x86_64.AppImage"
"$catdo_linuxdeploy" --appdir "$catdo_appdir" \
    --executable "$catdo_root/target/release/catdo" \
    --desktop-file "$catdo_root/packaging/com.workercat.catdo.desktop" \
    --icon-file "$catdo_root/assets/com.workercat.catdo.png" \
    --output appimage
"$LDAI_OUTPUT" --appimage-extract-and-run --help
[[ "$("$LDAI_OUTPUT" --appimage-extract-and-run --version)" == "CatDo $catdo_version" ]]
cd dist
sha256sum "$(basename "$LDAI_OUTPUT")" > "$(basename "$LDAI_OUTPUT").sha256"
