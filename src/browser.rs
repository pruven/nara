use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
    process::Command,
};

#[cfg(target_os = "linux")]
use gio::prelude::*;

#[derive(Clone, PartialEq, Eq)]
pub enum Browser {
    Firefox,
    Chrome,
    Edge,
    Brave,
    Chromium,
    Zen,
    Custom(PathBuf),
    #[cfg(target_os = "linux")]
    Desktop {
        id: String,
        name: String,
    },
}

impl Browser {
    pub const ALL: [Self; 6] = [
        Self::Firefox,
        Self::Chrome,
        Self::Edge,
        Self::Brave,
        Self::Chromium,
        Self::Zen,
    ];

    pub fn label(&self) -> &str {
        match self {
            Self::Firefox => "Firefox",
            Self::Chrome => "Chrome",
            Self::Edge => "Edge",
            Self::Brave => "Brave",
            Self::Chromium => "Chromium",
            Self::Zen => "Zen",
            #[cfg(target_os = "linux")]
            Self::Desktop { name, .. } => name,
            Self::Custom(path) => path
                .file_stem()
                .and_then(|name| name.to_str())
                .unwrap_or("Custom browser"),
        }
    }

    fn executable(&self) -> Option<PathBuf> {
        if let Self::Custom(path) = self {
            return custom_executable(path).ok();
        }
        #[cfg(target_os = "linux")]
        let candidates = match self {
            Self::Firefox => &["firefox", "firefox-esr"][..],
            Self::Chrome => &["google-chrome-stable", "google-chrome"],
            Self::Edge => &["microsoft-edge-stable", "microsoft-edge"],
            Self::Brave => &["brave-browser", "brave"],
            Self::Chromium => &["chromium", "chromium-browser"],
            Self::Zen => &["zen-browser", "zen"],
            Self::Custom(_) | Self::Desktop { .. } => unreachable!(),
        };
        #[cfg(windows)]
        let candidates = match self {
            Self::Firefox => &["firefox.exe", r"Mozilla Firefox\firefox.exe"][..],
            Self::Chrome => &["chrome.exe", r"Google\Chrome\Application\chrome.exe"],
            Self::Edge => &["msedge.exe", r"Microsoft\Edge\Application\msedge.exe"],
            Self::Brave => &[
                "brave.exe",
                r"BraveSoftware\Brave-Browser\Application\brave.exe",
            ],
            Self::Chromium => &["chromium.exe", r"Chromium\Application\chrome.exe"],
            Self::Zen => &["zen.exe", r"Zen Browser\zen.exe"],
            Self::Custom(_) => unreachable!(),
        };

        let path = std::env::var_os("PATH").unwrap_or_default();
        let on_path = std::env::split_paths(&path)
            .filter(|directory| directory.is_absolute())
            .flat_map(|directory| candidates.iter().map(move |name| directory.join(name)))
            .find(|path| path.is_file());
        #[cfg(windows)]
        let on_path = on_path.or_else(|| {
            ["LOCALAPPDATA", "ProgramFiles", "ProgramFiles(x86)"]
                .into_iter()
                .filter_map(std::env::var_os)
                .flat_map(|directory| {
                    candidates
                        .iter()
                        .map(move |name| PathBuf::from(&directory).join(name))
                })
                .find(|path| path.is_file())
        });
        on_path
    }

    pub fn installed(&self) -> bool {
        #[cfg(target_os = "linux")]
        if let Self::Desktop { id, .. } = self {
            return desktop_app(id).is_some();
        }
        self.executable().is_some()
    }

    pub fn open(&self, url: &url::Url) -> io::Result<()> {
        #[cfg(target_os = "linux")]
        if let Self::Desktop { id, .. } = self {
            let app = desktop_app(id).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("{} could not be found", self.label()),
                )
            })?;
            return app
                .launch_uris(&[url.as_str()], gio::AppLaunchContext::NONE)
                .map_err(io::Error::other);
        }
        let executable = self.executable().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("{} could not be found", self.label()),
            )
        })?;
        Command::new(executable).arg(url.as_str()).spawn()?;
        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn browser_apps() -> impl Iterator<Item = gio::AppInfo> {
    gio::AppInfo::all_for_type("x-scheme-handler/https")
        .into_iter()
        .chain(gio::AppInfo::all_for_type("x-scheme-handler/http"))
        .filter(|app| {
            app.supports_uris()
                && app.should_show()
                && app.id().is_some_and(|id| {
                    !matches!(id.as_str(), "nara.desktop" | "omarchy-browser.desktop")
                })
        })
}

#[cfg(target_os = "linux")]
fn desktop_app(id: &str) -> Option<gio::AppInfo> {
    browser_apps().find(|app| app.id().is_some_and(|candidate| candidate == id))
}

fn browser_name(name: &str) -> String {
    let name = name.to_lowercase();
    name.strip_suffix(" browser").unwrap_or(&name).to_owned()
}

pub fn available() -> Vec<Browser> {
    let mut browsers = Vec::new();
    #[cfg(target_os = "linux")]
    for app in browser_apps() {
        let Some(id) = app.id() else { continue };
        let name = app.display_name().to_string();
        if !browsers
            .iter()
            .any(|browser: &Browser| browser_name(browser.label()) == browser_name(&name))
        {
            browsers.push(Browser::Desktop {
                id: id.to_string(),
                name,
            });
        }
    }
    for browser in Browser::ALL {
        if browser.installed()
            && !browsers
                .iter()
                .any(|entry| browser_name(entry.label()) == browser_name(browser.label()))
        {
            browsers.push(browser);
        }
    }
    browsers.sort_by_key(|browser| browser.label().to_lowercase());
    browsers
}

fn custom_executable(path: &std::path::Path) -> io::Result<PathBuf> {
    if !path.is_absolute() || !path.is_file() {
        return Err(io::Error::other("Choose an installed browser executable."));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if path.metadata()?.permissions().mode() & 0o111 == 0 {
            return Err(io::Error::other("That file is not executable."));
        }
    }
    #[cfg(windows)]
    if !path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
    {
        return Err(io::Error::other("Choose the browser's .exe file."));
    }
    if path.canonicalize()? == std::env::current_exe()?.canonicalize()? {
        return Err(io::Error::other(
            "Choose the browser Nara should forward links to.",
        ));
    }
    Ok(path.to_owned())
}

fn preference_path() -> io::Result<PathBuf> {
    dirs::config_dir()
        .map(|root| root.join("nara").join("browser"))
        .ok_or_else(|| io::Error::other("Could not find the user configuration folder"))
}

pub fn preferred() -> Option<Browser> {
    let stored = fs::read_to_string(preference_path().ok()?).ok()?;
    if let Some(executable) = stored.strip_prefix("path:") {
        return Some(Browser::Custom(PathBuf::from(executable)));
    }
    #[cfg(target_os = "linux")]
    if let Some(id) = stored.trim().strip_prefix("desktop:") {
        return Some(Browser::Desktop {
            id: id.to_owned(),
            name: desktop_app(id)
                .map_or_else(|| id.to_owned(), |app| app.display_name().to_string()),
        });
    }
    let legacy = Browser::ALL
        .into_iter()
        .find(|browser| browser.label() == stored.trim())?;
    #[cfg(target_os = "linux")]
    if let Some(browser) = available()
        .into_iter()
        .find(|browser| browser_name(browser.label()) == browser_name(legacy.label()))
    {
        return Some(browser);
    }
    Some(legacy)
}

pub fn save(browser: &Browser) -> io::Result<()> {
    let path = preference_path()?;
    let directory = path.parent().expect("preference path has a parent");
    fs::create_dir_all(directory)?;
    let mut file = tempfile::NamedTempFile::new_in(directory)?;
    if let Browser::Custom(executable) = browser {
        custom_executable(executable)?;
        let executable = executable
            .to_str()
            .ok_or_else(|| io::Error::other("The browser path must be valid Unicode."))?;
        write!(file, "path:{executable}")?;
    } else {
        #[cfg(target_os = "linux")]
        if let Browser::Desktop { id, .. } = browser {
            writeln!(file, "desktop:{id}")?;
            file.persist(path)?;
            return Ok(());
        }
        writeln!(file, "{}", browser.label())?;
    }
    file.persist(path)?;
    Ok(())
}
