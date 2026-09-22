//! GitHub release discovery and verified, atomic Linux updates.
use anyhow::{Context, Result, bail, ensure};
use reqwest::blocking::Client;
use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Write},
    os::unix::{fs::PermissionsExt, process::CommandExt},
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};
use tempfile::TempPath;

const REPOSITORY: &str = "https://github.com/workercatstudios/catdo/releases/download/";
const MAX_DOWNLOAD: u64 = 256 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Package {
    AppImage,
    Binary,
}

#[derive(Clone, Debug)]
pub struct Installation {
    pub path: PathBuf,
    pub package: Package,
}

impl Installation {
    pub fn detect() -> Result<Self> {
        ensure!(
            cfg!(all(target_os = "linux", target_arch = "x86_64")),
            "Updates currently support Linux x86-64"
        );
        let (path, package) = match std::env::var_os("APPIMAGE") {
            Some(path) => (PathBuf::from(path), Package::AppImage),
            None => (std::env::current_exe()?, Package::Binary),
        };
        Ok(Self {
            path: path.canonicalize()?,
            package,
        })
    }
}

#[derive(Deserialize)]
pub(crate) struct Release {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<Asset>,
}

#[derive(Clone, Debug, Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    size: u64,
}

#[derive(Clone, Debug)]
pub struct Update {
    pub version: Version,
    asset: Asset,
    checksum: Asset,
    pub installation: Installation,
}

pub struct ReadyUpdate {
    pub update: Update,
    staged: TempPath,
    digest: String,
}

fn client() -> Result<Client> {
    Ok(Client::builder()
        .user_agent(concat!("CatDo/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(180))
        .https_only(true)
        .build()?)
}

pub fn check() -> Result<Option<Update>> {
    let installation = Installation::detect()?;
    let response = client()?
        .get("https://api.github.com/repos/workercatstudios/catdo/releases/latest")
        .header("Accept", "application/vnd.github+json")
        .send()?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let release: Release =
        serde_json::from_reader(response.error_for_status()?.take(2 * 1024 * 1024))?;
    select(
        release,
        &Version::parse(env!("CARGO_PKG_VERSION"))?,
        installation,
    )
}

fn select(
    release: Release,
    current: &Version,
    installation: Installation,
) -> Result<Option<Update>> {
    let version = Version::parse(
        release
            .tag_name
            .strip_prefix('v')
            .unwrap_or(&release.tag_name),
    )?;
    if release.draft || release.prerelease || !version.pre.is_empty() || version <= *current {
        return Ok(None);
    }
    let extension = match installation.package {
        Package::AppImage => "AppImage",
        Package::Binary => "tar.gz",
    };
    let name = format!("catdo-{version}-linux-x86_64.{extension}");
    let asset = |name: &str| -> Result<Asset> {
        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == name)
            .with_context(|| format!("Release {version} is missing {name}"))?;
        ensure!(
            asset.browser_download_url == format!("{REPOSITORY}{}/{name}", release.tag_name),
            "Unexpected update download URL"
        );
        ensure!(
            asset.size > 0 && asset.size <= MAX_DOWNLOAD,
            "Invalid update size"
        );
        Ok(asset.clone())
    };
    let download = asset(&name)?;
    let checksum = asset(&format!("{name}.sha256"))?;
    Ok(Some(Update {
        version,
        asset: download,
        checksum,
        installation,
    }))
}

fn checksum(text: &str, name: &str) -> Result<String> {
    let fields: Vec<_> = text.split_whitespace().collect();
    ensure!(
        fields.len() == 2 && fields[1].trim_start_matches('*') == name,
        "Invalid update checksum file"
    );
    ensure!(
        fields[0].len() == 64 && fields[0].bytes().all(|c| c.is_ascii_hexdigit()),
        "Invalid SHA-256 checksum"
    );
    Ok(fields[0].to_ascii_lowercase())
}

fn digest(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut hash = Sha256::new();
    std::io::copy(&mut file, &mut hash)?;
    Ok(format!("{:x}", hash.finalize()))
}

pub fn download(update: Update) -> Result<ReadyUpdate> {
    let client = client()?;
    let mut checksum_text = String::new();
    client
        .get(&update.checksum.browser_download_url)
        .send()?
        .error_for_status()?
        .take(4096)
        .read_to_string(&mut checksum_text)?;
    let expected = checksum(&checksum_text, &update.asset.name)?;
    // Stage next to the executable: installation must be a same-filesystem rename.
    let parent = update
        .installation
        .path
        .parent()
        .context("No installation directory")?;
    let mut archive = tempfile::Builder::new().prefix(".catdo-download-").tempfile_in(parent)
        .context("CatDo's installation folder is not writable. Move or install CatDo to a folder you own, then retry")?;
    let mut response = client
        .get(&update.asset.browser_download_url)
        .send()?
        .error_for_status()?
        .take(MAX_DOWNLOAD + 1);
    let size = std::io::copy(&mut response, &mut archive)?;
    ensure!(
        size == update.asset.size,
        "Update download was incomplete or too large"
    );
    ensure!(
        digest(archive.path())? == expected,
        "Update checksum did not match. Please download again"
    );
    let mut staged = match update.installation.package {
        Package::AppImage => archive,
        Package::Binary => {
            let mut staged = tempfile::Builder::new()
                .prefix(".catdo-update-")
                .tempfile_in(parent)?;
            extract_binary(archive.path(), &update.version, &mut staged)?;
            staged
        }
    };
    staged.as_file_mut().flush()?;
    staged
        .as_file()
        .set_permissions(fs::Permissions::from_mode(0o755))?;
    staged.as_file().sync_all()?;
    let digest = digest(staged.path())?;
    Ok(ReadyUpdate {
        update,
        staged: staged.into_temp_path(),
        digest,
    })
}

fn extract_binary(archive: &Path, version: &Version, output: &mut impl Write) -> Result<()> {
    let expected = PathBuf::from(format!("catdo-{version}-linux-x86_64/catdo"));
    let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(File::open(archive)?));
    for entry in tar.entries()? {
        let mut entry = entry?;
        if entry.path()? == expected {
            ensure!(
                entry.header().entry_type().is_file() && entry.size() <= MAX_DOWNLOAD,
                "Invalid executable in update archive"
            );
            std::io::copy(&mut entry, output)?;
            return Ok(());
        }
    }
    bail!("Update archive does not contain CatDo")
}

impl ReadyUpdate {
    /// Replace only the program, retaining one previous version for rollback.
    fn install(&self) -> Result<PathBuf> {
        ensure!(
            digest(self.staged.as_ref())? == self.digest,
            "Downloaded update changed; download it again"
        );
        let target = &self.update.installation.path;
        let backup = target.with_file_name(format!(
            "{}.previous",
            target
                .file_name()
                .context("No executable name")?
                .to_string_lossy()
        ));
        if backup.exists() {
            fs::remove_file(&backup)?;
        }
        fs::hard_link(target, &backup).context("Could not keep a rollback copy of CatDo")?;
        fs::rename(&self.staged, target).context("Could not replace CatDo")?;
        File::open(target.parent().context("No installation directory")?)?.sync_all()?;
        Ok(backup)
    }

    pub fn install_and_restart(&self) -> Result<()> {
        let backup = self.install()?;
        let target = &self.update.installation.path;
        let mut command = Command::new(target);
        command.args(std::env::args_os().skip(1));
        if self.update.installation.package == Package::AppImage {
            // Extraction also works on systems without FUSE. Runtime flags have
            // already been consumed before CatDo receives its arguments.
            command.env("APPIMAGE_EXTRACT_AND_RUN", "1");
            // The AppImage runtime must establish its own mount and library paths.
            for key in [
                "APPIMAGE",
                "APPDIR",
                "ARGV0",
                "OWD",
                "LD_LIBRARY_PATH",
                "LD_PRELOAD",
            ] {
                command.env_remove(key);
            }
        }
        let error = command.exec();
        fs::rename(&backup, target)
            .context("Restart failed, and CatDo could not restore the previous version")?;
        Err(error).context("Could not restart CatDo; the previous version was restored")
    }
}

#[cfg(test)]
#[path = "update_tests.rs"]
mod tests;
