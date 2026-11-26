#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod browser;
mod check;
mod review;
mod settings;
mod update;

#[cfg(target_os = "linux")]
const DESKTOP_ID: &str = match option_env!("NARA_DESKTOP_ID") {
    Some(id) => id,
    None => "nara.desktop",
};

const BACKGROUND: u32 = 0x19191c;
const SURFACE: u32 = 0x242428;
const BORDER: u32 = 0x39393e;
const FOREGROUND: u32 = 0xf2f1ef;
const MUTED: u32 = 0xb4b2ba;

fn main() {
    let input = std::env::args().nth(1);
    match input.as_deref() {
        #[cfg(target_os = "linux")]
        Some("--desktop-id") => println!("{DESKTOP_ID}"),
        Some("--settings") => settings::run(),
        Some("--help") | Some("-h") => println!("Usage: nara [--settings | URL]"),
        _ => review::run(input),
    }
}
