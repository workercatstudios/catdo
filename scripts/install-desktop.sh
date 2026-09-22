#!/usr/bin/env bash
set -euo pipefail

catdo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
catdo_bin_dir="${XDG_BIN_HOME:-$HOME/.local/bin}"
catdo_data_dir="${XDG_DATA_HOME:-$HOME/.local/share}"

cd "$catdo_root"
case "${1:-}" in
    '')
        cargo build --locked --release -p catdo-desktop
        catdo_binary="$catdo_root/target/release/catdo"
        ;;
    --prebuilt)
        catdo_binary="$catdo_root/catdo"
        ;;
    *)
        printf 'Usage: bash scripts/install-desktop.sh [--prebuilt]\n' >&2
        exit 1
        ;;
esac
install -Dm755 "$catdo_binary" "$catdo_bin_dir/catdo"
install -Dm644 assets/com.workercat.catdo.png "$catdo_data_dir/catdo/brand/icon.png"
# Resolve the launcher to the installed binary even when the desktop session's
# PATH does not include ~/.local/bin. Quote according to the Desktop Entry spec.
python3 - "$catdo_bin_dir/catdo" "$catdo_data_dir/applications/com.workercat.catdo.desktop" "$catdo_data_dir/catdo/brand/icon.png" <<'PY'
import pathlib
import sys

binary, destination, icon = sys.argv[1:]
escaped = binary.replace('\\', '\\\\').replace('"', '\\"').replace('`', '\\`').replace('$', '\\$').replace('%', '%%')
template = pathlib.Path('packaging/com.workercat.catdo.desktop').read_text()
target = pathlib.Path(destination)
target.parent.mkdir(parents=True, exist_ok=True)
icon_value = icon.replace('\\', '\\\\').replace('\n', '\\n').replace('\r', '\\r')
target.write_text(template.replace('Exec=catdo', f'Exec="{escaped}"').replace('Icon=com.workercat.catdo', f'Icon={icon_value}'))
PY

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$catdo_data_dir/applications"
fi
printf 'Installed CatDo to %s\n' "$catdo_bin_dir/catdo"
