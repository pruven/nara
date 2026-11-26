#!/bin/sh
set -eu

if [ "$#" -ne 1 ] || [ ! -x "$1" ]; then
    printf '%s\n' 'Usage: packaging/linux/install.sh path/to/nara' >&2
    exit 2
fi

assets=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
data_dir=${XDG_DATA_HOME:-"$HOME/.local/share"}
# Ask the binary so a custom build and its installed entry always agree.
desktop_id=$("$1" --desktop-id)
case "$desktop_id" in
  *.desktop) binary_name=${desktop_id%.desktop} ;;
  *) printf '%s\n' 'The binary did not report a valid desktop ID.' >&2; exit 1 ;;
esac
case "$binary_name" in
  ''|*[!a-zA-Z0-9_.-]*) printf '%s\n' 'Invalid desktop ID.' >&2; exit 1 ;;
esac
app_name=Nara
if [ "$binary_name" = nara-dev ]; then
  app_name='Nara (dev)'
elif [ "$binary_name" != nara ]; then
  app_name="Nara ($binary_name)"
fi
install -Dm755 "$1" "$HOME/.local/bin/$binary_name"
mkdir -p "$data_dir/applications"
sed -e "s/^Name=Nara\$/Name=$app_name/" \
    -e "s/^Exec=nara /Exec=$binary_name /" \
    -e "s/^Icon=nara\$/Icon=$binary_name/" \
    "$assets/nara.desktop" > "$data_dir/applications/$desktop_id"
chmod 644 "$data_dir/applications/$desktop_id"
install -Dm644 "$assets/nara.svg" "$data_dir/icons/hicolor/scalable/apps/$binary_name.svg"
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$data_dir/applications"
fi
printf '%s\n' 'Nara installed. Open Nara Settings to choose a browser and set up link handling.'
