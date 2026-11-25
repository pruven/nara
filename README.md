# Nara

Nara opens a compact link review window when a desktop app hands it an HTTP or
HTTPS link. Review the destination, optionally check its HTTP response, then
open the original link in your chosen browser. GPUI renders the review window,
and Rune renders browser settings. Both use a charcoal palette inspired by
[Saiki](https://github.com/pruven/saiki).

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

Use the default handler button to begin routing desktop links through Nara.
On Windows, choose Nara for HTTP and HTTPS in Default Apps. To restore the
previous behavior, choose your browser again in the system settings.

Links that a browser handles within its own tabs follow that browser's usual
navigation flow. Nara receives links that applications send to the system
handler.

## Review a link

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
