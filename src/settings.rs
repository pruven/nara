#[cfg(windows)]
use std::process::Command;

#[cfg(target_os = "linux")]
use gio::prelude::*;

use rune::{
    AppHandle, AppSpec, Cx, El, FileDialogOptions, col, color::Color, el, text, tok::type_,
};

use crate::{
    BACKGROUND, BORDER, FOREGROUND, MUTED, SURFACE,
    browser::{self, Browser},
    update::{self, Update},
};

struct Settings {
    browsers: Vec<Browser>,
    selected: Option<Browser>,
    message: String,
    #[cfg(target_os = "linux")]
    default_handler: bool,
    update: Option<Update>,
    update_message: String,
    updating: bool,
    handle: Option<AppHandle<Self>>,
}

impl Settings {
    fn new(browsers: Vec<Browser>) -> Self {
        let selected = browser::preferred_from(&browsers);
        Self {
            browsers,
            selected,
            message: String::new(),
            #[cfg(target_os = "linux")]
            default_handler: is_default_handler(),
            update: None,
            update_message: format!("Nara {}", env!("CARGO_PKG_VERSION")),
            updating: false,
            handle: None,
        }
    }

    fn update_in_background(
        &mut self,
        message: &str,
        work: impl FnOnce() -> (String, Option<Update>) + Send + 'static,
    ) {
        let Some(handle) = self.handle.clone() else {
            return;
        };
        self.updating = true;
        self.update_message = message.to_owned();
        let result = std::thread::Builder::new().spawn(move || {
            let (message, update) = work();
            handle.update(move |state, _| {
                state.update_message = message;
                state.update = update;
                state.updating = false;
            });
        });
        if let Err(error) = result {
            self.updating = false;
            self.update_message = format!("Could not start update task. {error}");
        }
    }

    fn select(&mut self, browser: Browser) {
        match browser::save(&browser) {
            Ok(()) => {
                self.message = format!("Links will open in {}.", browser.label());
                self.selected = Some(browser);
            }
            Err(error) => self.message = format!("Could not save browser preference. {error}"),
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
    for browser in state.browsers.iter().cloned() {
        let label = if state.selected.as_ref() == Some(&browser) {
            format!("{} · Selected", browser.label())
        } else {
            browser.label().to_owned()
        };
        browsers = browsers.child(
            button(&label, browser.label())
                .on_press(move |state: &mut Settings| state.select(browser.clone())),
        );
    }
    if state.browsers.is_empty() {
        browsers = browsers.child(text("No browsers detected.").color(color(MUTED)));
    }
    #[cfg(windows)]
    let default_button = "Set Nara as default link opener…";
    #[cfg(target_os = "linux")]
    let default_button = if state.default_handler {
        "Nara is the default link opener"
    } else {
        "Set Nara as default link opener"
    };

    let content = col()
        .w_full()
        .h_full()
        .padding(24.0)
        .gap(14.0)
        .fill(color(BACKGROUND))
        .child(
            text("Nara settings")
                .font(type_::TITLE)
                .color(color(FOREGROUND)),
        )
        .child(
            text("Open web links from other apps in Nara before your browser.")
                .font(type_::BODY)
                .color(color(MUTED))
                .wrap(),
        )
        .child(
            button(default_button, "default-handler").on_press(|state: &mut Settings| {
                if !state.selected.as_ref().is_some_and(Browser::installed) {
                    state.message = "Choose a browser below first.".to_owned();
                    return;
                }
                state.message = match set_default() {
                    Ok(message) => message.to_owned(),
                    Err(error) => format!("Could not set up the link handler. {error}"),
                };
                #[cfg(target_os = "linux")]
                {
                    state.default_handler = is_default_handler();
                }
            }),
        )
        .child(
            text(state.message.clone())
                .font(type_::BODY)
                .color(color(MUTED))
                .wrap(),
        )
        .child(
            text("Choose where Nara opens reviewed links.")
                .font(type_::BODY)
                .color(color(MUTED)),
        )
        .child(
            browsers
                .scroll(rune::ScrollSpec {
                    stick_bottom: false,
                })
                .shrink()
                .min_h(0.0),
        )
        .child(
            button(
                &match &state.selected {
                    Some(browser @ Browser::Custom(_)) => {
                        format!("{} · Selected (browse…)", browser.label())
                    }
                    _ => "Choose another browser…".to_owned(),
                },
                "custom-browser",
            )
            .on_press_tx(|_: &mut Settings, tx| {
                let options = FileDialogOptions {
                    title: "Choose a browser executable".to_owned(),
                    ..Default::default()
                };
                #[cfg(windows)]
                let options = options.filter("Browser executable", &["exe"]);
                tx.open_file(options, |state: &mut Settings, result, _| match result {
                    Ok(Some(path)) => state.select(Browser::Custom(path)),
                    Ok(None) => {}
                    Err(error) => {
                        state.message = format!("Could not open the file chooser. {error}")
                    }
                });
            }),
        );

    let mut updates = col().w_full().gap(8.0);
    if !state.updating {
        updates = updates.child(button("Check for updates", "check-update").on_press(
            |state: &mut Settings| {
                let previous = state.update.clone();
                state.update_in_background("Checking for updates…", move || match update::check()
                {
                    Ok(Some(update)) => (
                        format!("Nara {} is available.", update.version),
                        Some(update),
                    ),
                    Ok(None) => ("You have the latest version.".to_owned(), None),
                    Err(error) => (format!("Could not check for updates. {error}"), previous),
                });
            },
        ));
        if let Some(update) = state.update.clone() {
            updates = updates.child(button("Download update", "download-update").on_press_tx(
                move |state: &mut Settings, tx| {
                    state.updating = true;
                    let update = update.clone();
                    tx.save_file(
                        FileDialogOptions {
                            title: "Save Nara update".to_owned(),
                            directory: dirs::download_dir(),
                            file_name: Some(update.filename.to_owned()),
                            ..Default::default()
                        },
                        move |state: &mut Settings, result, _| {
                            state.updating = false;
                            match result {
                                Ok(Some(path)) => {
                                    state.update_in_background(
                                        "Downloading update…",
                                        move || {
                                            let message = match update.download(&path) {
                                                Ok(()) => {
                                                    format!("Update saved to {}.", path.display())
                                                }
                                                Err(error) => {
                                                    format!("Could not download update. {error}")
                                                }
                                            };
                                            (message, Some(update))
                                        },
                                    );
                                }
                                Ok(None) => {}
                                Err(error) => {
                                    state.update_message =
                                        format!("Could not open the save dialog. {error}");
                                }
                            }
                        },
                    );
                },
            ));
        }
    }
    content.child(
        updates.child(
            text(state.update_message.clone())
                .font(type_::BODY)
                .color(color(MUTED))
                .wrap(),
        ),
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
fn is_default_handler() -> bool {
    ["http", "https"].into_iter().all(|scheme| {
        gio::AppInfo::default_for_uri_scheme(scheme)
            .and_then(|app| app.id())
            .is_some_and(|id| id == crate::DESKTOP_ID)
    })
}

#[cfg(target_os = "linux")]
fn set_default() -> std::io::Result<&'static str> {
    let app = gio::AppInfo::all()
        .into_iter()
        .find(|app| app.id().is_some_and(|id| id == crate::DESKTOP_ID))
        .ok_or_else(|| {
            std::io::Error::other(format!("Install {} first and retry.", crate::DESKTOP_ID))
        })?;
    let config = dirs::config_dir()
        .ok_or_else(|| std::io::Error::other("Could not find the configuration folder."))?;
    check_desktop_overrides(&config)?;
    with_association_rollback(&config.join("mimeapps.list"), || {
        for scheme in ["http", "https"] {
            app.set_as_default_for_type(&format!("x-scheme-handler/{scheme}"))
                .map_err(|error| {
                    std::io::Error::other(format!(
                        "Could not assign {scheme} links to Nara. {error}"
                    ))
                })?;
        }
        if !is_default_handler() {
            return Err(std::io::Error::other(
                "The desktop did not apply both link associations. Check your system's default apps.",
            ));
        }
        Ok(())
    })?;
    Ok("Nara now opens HTTP and HTTPS links instead of your previous default browser.")
}

// Desktop-specific defaults take precedence over the generic file GIO writes.
// Explain that override before changing either scheme, rather than leaving a
// partially working setup or rewriting desktop-managed configuration silently.
#[cfg(target_os = "linux")]
fn check_desktop_overrides(config: &std::path::Path) -> std::io::Result<()> {
    let desktops: Vec<String> = std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .split(':')
        .filter(|name| {
            !name.is_empty()
                && name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        })
        .map(str::to_lowercase)
        .collect();
    let mut directories = vec![config.to_owned()];
    let system = std::env::var_os("XDG_CONFIG_DIRS")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "/etc/xdg".into());
    directories.extend(std::env::split_paths(&system).filter(|path| path.is_absolute()));
    let apps = gio::AppInfo::all();
    for scheme in ["http", "https"] {
        'directories: for directory in &directories {
            for desktop in desktops.iter().map(Some).chain(std::iter::once(None)) {
                let filename = desktop.map_or_else(
                    || "mimeapps.list".to_owned(),
                    |name| format!("{name}-mimeapps.list"),
                );
                let path = directory.join(filename);
                let file = gio::glib::KeyFile::new();
                if file
                    .load_from_file(&path, gio::glib::KeyFileFlags::NONE)
                    .is_err()
                {
                    continue;
                }
                let Ok(ids) = file.string_list(
                    "Default Applications",
                    &format!("x-scheme-handler/{scheme}"),
                ) else {
                    continue;
                };
                let first = ids.iter().find(|id| {
                    apps.iter().any(|app| {
                        app.id()
                            .is_some_and(|candidate| candidate.as_str() == id.as_str())
                    })
                });
                if let Some(id) = first {
                    if desktop.is_some() && id.as_str() != crate::DESKTOP_ID {
                        return Err(std::io::Error::other(format!(
                            "{} overrides {scheme} links with {}. Set its x-scheme-handler/{scheme} entry under [Default Applications] to {} and retry. No associations were changed.",
                            path.display(),
                            id,
                            crate::DESKTOP_ID,
                        )));
                    }
                    break 'directories;
                }
            }
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn with_association_rollback(
    path: &std::path::Path,
    apply: impl FnOnce() -> std::io::Result<()>,
) -> std::io::Result<()> {
    use std::{fs, io::Write};

    // GIO writes both defaults and added associations to this user file. Keep
    // the exact contents, including the absence of any previous default.
    let previous = match fs::read(path) {
        Ok(bytes) => Some((bytes, fs::metadata(path)?.permissions())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };
    if let Err(error) = apply() {
        let restore = (|| -> std::io::Result<()> {
            if let Some((bytes, permissions)) = previous {
                let mut file = tempfile::NamedTempFile::new_in(
                    path.parent().expect("association file has a parent"),
                )?;
                file.write_all(&bytes)?;
                file.as_file().set_permissions(permissions)?;
                file.persist(path)?;
            } else {
                match fs::remove_file(path) {
                    Ok(()) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error),
                }
            }
            Ok(())
        })();
        return match restore {
            Ok(()) => Err(error),
            Err(restore_error) => Err(std::io::Error::other(format!(
                "{error} Could not restore previous associations: {restore_error}",
            ))),
        };
    }
    Ok(())
}

pub fn run() {
    let browsers = browser::available();
    let installed = browsers.len().clamp(1, 7);
    let app = rune::App::new(AppSpec::new(
        "Nara Settings",
        (440.0, 470.0 + 44.0 * installed as f32),
        || Settings::new(browser::available()),
        view,
    ))
    .init(move || Settings::new(browsers));
    let handle = app.handle();
    let state_handle = handle.clone();
    handle.update(move |state, _| state.handle = Some(state_handle));
    app.run();
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn default_link_associations() {
        // GIO caches desktop state. Run in a fresh process with isolated XDG folders
        // so this test never changes the user's actual default applications.
        if std::env::var_os("NARA_TEST_DEFAULT_HANDLER").is_none() {
            for desktop in [
                "",
                "Hyprland",
                "Valid-Name_2",
                "foo.bar",
                "bad name",
                "Ünicode",
            ] {
                let root = tempfile::tempdir().unwrap();
                let status = std::process::Command::new(std::env::current_exe().unwrap())
                    .args(["--exact", "settings::tests::default_link_associations"])
                    .env("NARA_TEST_DEFAULT_HANDLER", "1")
                    .env("XDG_CONFIG_HOME", root.path().join("config"))
                    .env("XDG_CONFIG_DIRS", root.path().join("config-dirs"))
                    .env("XDG_DATA_HOME", root.path().join("data"))
                    .env("XDG_DATA_DIRS", root.path().join("data-dirs"))
                    .env("XDG_CURRENT_DESKTOP", desktop)
                    .status()
                    .unwrap();
                assert!(status.success());
            }
            return;
        }

        let applications = dirs::data_dir().unwrap().join("applications");
        std::fs::create_dir_all(&applications).unwrap();
        for id in [crate::DESKTOP_ID, "other-browser.desktop"] {
            std::fs::write(
                applications.join(id),
                format!(
                    "[Desktop Entry]\nType=Application\nName={id}\nExec=/bin/true %u\nMimeType=x-scheme-handler/http;x-scheme-handler/https;\n"
                ),
            )
            .unwrap();
        }
        let config = dirs::config_dir().unwrap();
        std::fs::create_dir_all(&config).unwrap();
        let associations = config.join("mimeapps.list");
        let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap();
        if !desktop.is_empty() {
            let override_path = config.join(format!("{}-mimeapps.list", desktop.to_lowercase()));
            let original = "[Default Applications]\nx-scheme-handler/http=other-browser.desktop;\nx-scheme-handler/https=other-browser.desktop;\n";
            std::fs::write(&override_path, original).unwrap();
            if matches!(desktop.as_str(), "Hyprland" | "Valid-Name_2" | "foo.bar") {
                let error = set_default().unwrap_err().to_string();
                assert!(error.contains(override_path.to_str().unwrap()), "{error}");
                assert!(error.contains("No associations were changed"));
                assert!(!associations.exists());
                assert_eq!(std::fs::read_to_string(override_path).unwrap(), original);
                return;
            }
            // Invalid desktop names must not let this file block setup.
            check_desktop_overrides(&config).unwrap();
        }
        let app = gio::AppInfo::all()
            .into_iter()
            .find(|app| app.id().is_some_and(|id| id == crate::DESKTOP_ID))
            .unwrap();
        // Exercise a failure after HTTP succeeds, and one after both schemes
        // succeed (the verification-failure path), with and without old defaults.
        for original in [
            None,
            Some(
                "[Default Applications]\nx-scheme-handler/http=other-browser.desktop;\nx-scheme-handler/https=other-browser.desktop;\ntext/html=other-browser.desktop;\n",
            ),
        ] {
            for change_https in [false, true] {
                if let Some(contents) = original {
                    std::fs::write(&associations, contents).unwrap();
                }
                let result = with_association_rollback(&associations, || {
                    app.set_as_default_for_type("x-scheme-handler/http")
                        .unwrap();
                    if change_https {
                        app.set_as_default_for_type("x-scheme-handler/https")
                            .unwrap();
                    }
                    Err(std::io::Error::other("injected association failure"))
                });
                assert!(result.unwrap_err().to_string().contains("injected"));
                assert_eq!(
                    std::fs::read_to_string(&associations).ok().as_deref(),
                    original
                );
            }
        }
        std::fs::remove_file(&associations).unwrap();
        assert!(!is_default_handler());
        set_default().unwrap();
        assert!(is_default_handler());
        let other = gio::AppInfo::all()
            .into_iter()
            .find(|app| app.id().is_some_and(|id| id == "other-browser.desktop"))
            .unwrap();
        other
            .set_as_default_for_type("x-scheme-handler/https")
            .unwrap();
        assert!(!is_default_handler(), "both schemes must belong to Nara");
        set_default().unwrap();
        assert!(is_default_handler());
    }
}
