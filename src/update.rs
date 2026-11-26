use std::{
    error::Error,
    io::{self, Read, Write},
    path::Path,
    time::Duration,
};

use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};

const REPOSITORY: &str = "https://github.com/pruven/nara";
const LATEST: &str = "https://api.github.com/repos/pruven/nara/releases/latest";
const MAX_PACKAGE_SIZE: u64 = 256 * 1024 * 1024;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    size: u64,
}

#[derive(Clone)]
pub struct Update {
    pub version: Version,
    pub filename: &'static str,
    size: u64,
}

fn metadata_agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .https_only(true)
        .timeout_global(Some(Duration::from_secs(15)))
        .build()
        .into()
}

pub fn check() -> Result<Option<Update>> {
    let release: Release = serde_json::from_str(
        &metadata_agent()
            .get(LATEST)
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", concat!("Nara/", env!("CARGO_PKG_VERSION")))
            .call()?
            .body_mut()
            .with_config()
            .limit(1024 * 1024)
            .read_to_string()?,
    )?;

    let version = Version::parse(
        release
            .tag_name
            .strip_prefix('v')
            .ok_or("Invalid release tag")?,
    )?;
    if version <= Version::parse(env!("CARGO_PKG_VERSION"))? {
        return Ok(None);
    }
    let filename = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "nara-linux-x86_64.tar.gz",
        ("linux", "aarch64") => "nara-linux-aarch64.tar.gz",
        ("windows", "x86_64") => "nara-windows-x86_64-setup.exe",
        ("windows", "aarch64") => "nara-windows-arm64-setup.exe",
        _ => return Err("No update package is available for this platform".into()),
    };
    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == filename)
        .ok_or("This release is missing the package for this platform")?;
    if asset.size == 0 || asset.size > MAX_PACKAGE_SIZE {
        return Err("Invalid update package size".into());
    }
    Ok(Some(Update {
        version,
        filename,
        size: asset.size,
    }))
}

impl Update {
    pub fn download(&self, destination: &Path) -> Result<()> {
        let base = format!("{REPOSITORY}/releases/download/v{}", self.version);
        let checksums = metadata_agent()
            .get(format!("{base}/SHA256SUMS"))
            .call()?
            .body_mut()
            .with_config()
            .limit(64 * 1024)
            .read_to_string()?;
        let expected = checksums
            .lines()
            .filter_map(|line| line.split_once("  "))
            .find_map(|(hash, filename)| (filename == self.filename).then_some(hash))
            .ok_or("The update package checksum is missing")?;

        let timeout = Some(Duration::from_secs(30));
        let package_agent: ureq::Agent = ureq::Agent::config_builder()
            .https_only(true)
            .timeout_resolve(timeout)
            .timeout_connect(timeout)
            .timeout_recv_response(timeout)
            .timeout_per_read(timeout)
            .timeout_per_write(timeout)
            .build()
            .into();
        let mut response = package_agent
            .get(format!("{base}/{}", self.filename))
            .call()?
            .into_body()
            .into_reader()
            .take(self.size + 1);
        let directory = destination
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let mut file = tempfile::NamedTempFile::new_in(directory)?;
        let mut hash = Sha256::new();
        let mut buffer = [0; 64 * 1024];
        let mut size = 0;
        loop {
            let count = response.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            file.write_all(&buffer[..count])?;
            hash.update(&buffer[..count]);
            size += count as u64;
        }
        if size != self.size || format!("{:x}", hash.finalize()) != expected {
            return Err(io::Error::other("The update package failed verification").into());
        }
        file.as_file().sync_all()?;
        file.persist(destination)?;
        Ok(())
    }
}
