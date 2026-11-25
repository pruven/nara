use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
    process::Command,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Browser {
    Firefox,
    Chrome,
    Edge,
    Brave,
    Chromium,
    Zen,
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

    pub fn label(self) -> &'static str {
        match self {
            Self::Firefox => "Firefox",
            Self::Chrome => "Chrome",
            Self::Edge => "Edge",
            Self::Brave => "Brave",
            Self::Chromium => "Chromium",
            Self::Zen => "Zen",
        }
    }

    fn executable(self) -> Option<PathBuf> {
        #[cfg(target_os = "linux")]
        let candidates = match self {
            Self::Firefox => &["firefox", "firefox-esr"][..],
            Self::Chrome => &["google-chrome-stable", "google-chrome"],
            Self::Edge => &["microsoft-edge-stable", "microsoft-edge"],
            Self::Brave => &["brave-browser", "brave"],
            Self::Chromium => &["chromium", "chromium-browser"],
            Self::Zen => &["zen-browser", "zen"],
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

    pub fn installed(self) -> bool {
        self.executable().is_some()
    }

    pub fn open(self, url: &url::Url) -> io::Result<()> {
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

fn preference_path() -> io::Result<PathBuf> {
    dirs::config_dir()
        .map(|root| root.join("nara").join("browser"))
        .ok_or_else(|| io::Error::other("Could not find the user configuration folder"))
}

pub fn preferred() -> Option<Browser> {
    let stored = fs::read_to_string(preference_path().ok()?).ok()?;
    Browser::ALL
        .into_iter()
        .find(|browser| browser.label() == stored.trim())
}

pub fn save(browser: Browser) -> io::Result<()> {
    let path = preference_path()?;
    let directory = path.parent().expect("preference path has a parent");
    fs::create_dir_all(directory)?;
    let mut file = tempfile::NamedTempFile::new_in(directory)?;
    writeln!(file, "{}", browser.label())?;
    file.persist(path)?;
    Ok(())
}
