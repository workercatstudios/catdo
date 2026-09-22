#!/usr/bin/env bash
set -euo pipefail

catdo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$catdo_root"
if [[ "$(uname -s)" != Linux || "$(uname -m)" != x86_64 ]]; then
    printf 'This package currently supports Linux x86-64 only.\n' >&2
    exit 1
fi
catdo_metadata="$(python3 scripts/release-metadata.py)"
catdo_version="$(sed -n 's/^version=//p' <<< "$catdo_metadata")"
catdo_name="catdo-$catdo_version-linux-x86_64"
catdo_stage="$(mktemp -d)"
trap 'rm -rf -- "$catdo_stage"' EXIT
catdo_package="$catdo_stage/$catdo_name"

install -Dm755 target/release/catdo "$catdo_package/catdo"
install -Dm755 scripts/install-desktop.sh "$catdo_package/scripts/install-desktop.sh"
install -Dm644 packaging/com.workercat.catdo.desktop "$catdo_package/packaging/com.workercat.catdo.desktop"
install -Dm644 assets/com.workercat.catdo.png "$catdo_package/assets/com.workercat.catdo.png"
install -Dm644 assets/README.md "$catdo_package/assets/README.md"
install -Dm644 packaging/README.md "$catdo_package/README.md"
install -Dm644 LICENSE.md "$catdo_package/LICENSE.md"
install -Dm644 NOTICE "$catdo_package/NOTICE"
python3 scripts/collect-licenses.py "$catdo_package/licenses"
mkdir -p dist
tar -czf "dist/$catdo_name.tar.gz" -C "$catdo_stage" "$catdo_name"
cd dist
sha256sum "$catdo_name.tar.gz" > "$catdo_name.tar.gz.sha256"
printf 'Packaged %s.tar.gz\n' "$catdo_name"
