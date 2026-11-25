use std::process::Command;

use rune::{AppSpec, Cx, El, col, color::Color, el, text, tok::type_};

use crate::{
    BACKGROUND, BORDER, FOREGROUND, MUTED, SURFACE,
    browser::{self, Browser},
};

struct Settings {
    browsers: Vec<Browser>,
    selected: Option<Browser>,
    message: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            browsers: Browser::ALL
                .into_iter()
                .filter(|browser| browser.installed())
                .collect(),
            selected: browser::preferred(),
            message: String::new(),
        }
    }
}

fn color(rgb: u32) -> Color {
    rune::color::hex(&format!("#{rgb:06x}")).expect("valid RGB color")
}

fn button(label: &str, key: &str) -> El {
    el().key(key.to_owned())
        .role("button")
        .label(label.to_owned())
        .focusable(true)
        .w_full()
        .h(38.0)
        .center()
        .radius(8.0)
        .fill(color(SURFACE))
        .border(1.0, color(BORDER))
        .hover(color(0x343438))
        .child(
            text(label.to_owned())
                .font(type_::BODY)
                .color(color(FOREGROUND)),
        )
}

fn view(state: &Settings, _: &mut Cx<Settings>) -> El {
    let mut browsers = col().w_full().gap(6.0);
    for browser in state.browsers.iter().copied() {
        let label = if state.selected == Some(browser) {
            format!("{} · Selected", browser.label())
        } else {
            browser.label().to_owned()
        };
        browsers = browsers.child(button(&label, browser.label()).on_press(
            move |state: &mut Settings| match browser::save(browser) {
                Ok(()) => {
                    state.selected = Some(browser);
                    state.message = format!("Links will open in {}.", browser.label());
                }
                Err(error) => state.message = format!("Could not save browser preference. {error}"),
            },
        ));
    }
    if state.browsers.is_empty() {
        browsers = browsers.child(text("No supported browser found.").color(color(MUTED)));
    }
    let mut content = col()
        .w_full()
        .h_full()
        .padding(24.0)
        .gap(14.0)
        .fill(color(BACKGROUND))
        .child(
            text("Browser settings")
                .font(type_::TITLE)
                .color(color(FOREGROUND)),
        )
        .child(
            text("Choose where Nara opens reviewed links.")
                .font(type_::BODY)
                .color(color(MUTED)),
        )
        .child(browsers);

    #[cfg(windows)]
    let default_button = "Open Windows Default Apps";
    #[cfg(target_os = "linux")]
    let default_button = "Set Nara as default link handler";

    content = content.child(button(default_button, "default-handler").on_press(
        |state: &mut Settings| {
            if !state.selected.is_some_and(|browser| browser.installed()) {
                state.message = "Choose a browser above first.".to_owned();
                return;
            }
            state.message = match set_default() {
                Ok(message) => message.to_owned(),
                Err(error) => format!("Could not set up the link handler. {error}"),
            };
        },
    ));
    content.child(
        text(state.message.clone())
            .font(type_::BODY)
            .color(color(MUTED))
            .wrap(),
    )
}

#[cfg(windows)]
fn set_default() -> std::io::Result<&'static str> {
    // This fixed URI opens Windows Settings, where the user selects the handler.
    Command::new("explorer.exe")
        .arg("ms-settings:defaultapps?registeredAppUser=Nara")
        .spawn()?;
    Ok("Select Nara for HTTP and HTTPS in Windows Settings.")
}

#[cfg(target_os = "linux")]
fn set_default() -> std::io::Result<&'static str> {
    let status = Command::new("xdg-settings")
        .args(["set", "default-web-browser", "nara.desktop"])
        .status()?;
    if !status.success() {
        return Err(std::io::Error::other(
            "Install nara.desktop first and retry.",
        ));
    }
    Ok("Nara is now the default web browser for this desktop.")
}

pub fn run() {
    let installed = Browser::ALL
        .into_iter()
        .filter(|browser| browser.installed())
        .count();
    rune::run(AppSpec::new(
        "Nara Settings",
        (440.0, 228.0 + 44.0 * installed.max(1) as f32),
        Settings::default,
        view,
    ));
}
