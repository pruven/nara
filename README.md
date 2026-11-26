# Nara

Nara opens a compact link review window when a desktop app hands it an HTTP or
HTTPS link. Review the destination, optionally check its HTTP response, then
open the original link in your chosen browser. GPUI renders the review window,
and Rune renders browser settings. Both use a charcoal palette inspired by
[Saiki](https://github.com/pruven/saiki).

## Download

Get the latest x64 and ARM64 packages and SHA-256 checksums from [GitHub Releases](https://github.com/pruven/nara/releases/latest).

- **Linux** — Extract the archive, open a terminal in the extracted folder,
  and run `./install.sh ./nara`. The binary targets Ubuntu 24.04 or newer,
  or a distribution with compatible runtime libraries.
- **Windows** — Run the setup executable to install Nara and register it as a
  link handler. The portable ZIP can run directly from the extracted folder.

## Build and try it

```sh
cargo build --locked
cargo run --locked -- https://example.org
cargo run --locked -- --settings
```

Linux builds use Wayland or X11 and require the corresponding native libraries
for GPUI, GLib development files for browser discovery, a GPU driver with Vulkan
support, and a working Fontconfig setup. On Windows, use Rust's MSVC toolchain
with Visual Studio C++ Build Tools and the Windows SDK.

## Open links automatically

- **Linux** — Run `packaging/linux/install.sh target/debug/nara` as your regular
  user. Make sure `~/.local/bin` is on your desktop session's PATH.
- **Windows** — Build with `cargo build --locked --release`, then compile
  `packaging/windows/nara.iss` with Inno Setup to register Nara for HTTP and HTTPS.

Open **Nara Settings** and select a detected browser, or use **Choose another
browser…** to select the browser executable yourself. Nara saves the choice
and uses it whenever you click **Open in browser**.

Click **Set Nara as default link opener** in Settings to replace the current
system handler for HTTP and HTTPS links. On Linux, Nara applies and verifies
both associations and shows when it is the default link opener. On Windows,
choose Nara for HTTP and HTTPS in the Default Apps window that opens. To restore
the previous behavior, choose your browser again in the system settings.

Links that a browser handles within its own tabs follow that browser's usual
navigation flow. Nara receives links that applications send to the system
handler.

### Omarchy

Omarchy's default-browser menu does not list Nara. After installing Nara,
open **Nara Settings**, choose the browser that should open reviewed links,
and click **Set Nara as default link opener**. If Settings reports a
desktop-specific override, use the steps below to update it, then retry.

To configure Nara manually, run:

```sh
desktop_id=nara.desktop
# For the separately installed Nara (dev) build, use instead:
# desktop_id=nara-dev.desktop

# Set only HTTP/HTTPS handlers; leave local HTML files in your browser.
xdg-mime default "$desktop_id" x-scheme-handler/http x-scheme-handler/https

# Hyprland's desktop-specific associations can override the default handler.
# Back up the file, then replace Omarchy's routing entry with Nara.
if [ -f ~/.config/hyprland-mimeapps.list ]; then
  backup=$(mktemp ~/.config/hyprland-mimeapps.list.bak.XXXXXX)
  cp -a ~/.config/hyprland-mimeapps.list "$backup"
  sed -i "s/=omarchy-browser\\.desktop/=$desktop_id/g" \
    ~/.config/hyprland-mimeapps.list
fi

# Both queries should print nara.desktop (or nara-dev.desktop).
xdg-mime query default x-scheme-handler/http
xdg-mime query default x-scheme-handler/https
```

If `~/.config/hyprland-mimeapps.list` already names another app directly,
change its `x-scheme-handler/http` and `x-scheme-handler/https` entries under
`[Default Applications]` to `nara.desktop` (or `nara-dev.desktop`).

Test the setup with `xdg-open https://example.org`. Nara should show the link
review window before opening the link in the browser selected in Nara Settings.

## Review a link

On Wayland desktops with layer-shell support, Nara opens as a compact,
centered popup. Other desktops use a regular review window.

Nara shows the host and URL before making any request.

- **Check link** sends an HTTP HEAD request and displays the status and timing. It
  replaces the displayed URL with the final destination if redirected. Some
  sites treat HEAD differently from browser navigation.
- **Unshorten** follows HTTP redirects on demand and updates the URL to the final
  destination. It uses GET without rendering the page and follows up to five redirects.
- **Remove tracking** strips recognized tracking parameters such as `utm_*`,
  `fbclid`, and `gclid` locally. Other parameters keep their order and encoding.
  Links with a recognized signature parameter are left intact.

**Remove tracking** appears when the current URL contains parameters Nara can
remove. **Undo** restores the original link after one or more edits. **Copy**
and **Open in browser** use the URL currently shown in the popup.

| Shortcut | Action |
| --- | --- |
| Ctrl+V | Paste a link |
| Ctrl+C | Copy the full link |
| Ctrl+Enter | Check the current link |
| Ctrl+U | Unshorten the current link |
| Ctrl+R | Remove tracking parameters |
| Ctrl+Z | Restore the original link |
| Enter | Open in the selected browser |
| Ctrl+, | Browser settings |
| Esc | Close the review window |

## Release signing

Tagged releases require an Ed25519 key. Generate it in a secure location and keep
the PEM private. Store its contents as the GitHub Actions secret
`NARA_UPDATE_PRIVATE_KEY`. Store the raw public key as 64 lowercase hex digits
in the Actions variable `NARA_UPDATE_PUBLIC_KEY`.

```sh
openssl genpkey -algorithm ED25519 -out nara-update-key.pem
openssl pkey -in nara-update-key.pem -pubout -outform DER |
  tail -c 32 | od -An -v -tx1 | tr -d ' \n'
```

The release builds embed the public key. The updater requires a signed checksum
manifest for the target release before downloading its package. Local builds
need the same public key in their build environment to download updates.
