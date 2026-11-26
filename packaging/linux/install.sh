#!/bin/sh
set -eu

if [ "$#" -ne 1 ] || [ ! -x "$1" ]; then
    printf '%s\n' 'Usage: packaging/linux/install.sh path/to/nara' >&2
    exit 2
fi

assets=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
data_dir=${XDG_DATA_HOME:-"$HOME/.local/share"}
install -Dm755 "$1" "$HOME/.local/bin/nara"
install -Dm644 "$assets/nara.desktop" "$data_dir/applications/nara.desktop"
install -Dm644 "$assets/nara.svg" "$data_dir/icons/hicolor/scalable/apps/nara.svg"
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$data_dir/applications"
fi
printf '%s\n' 'Nara installed. Open Nara Settings to choose a browser and set up link handling.'
