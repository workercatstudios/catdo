use super::*;
use tempfile::NamedTempFile;

fn release(version: &str) -> Release {
    let name = format!("catdo-{version}-linux-x86_64.AppImage");
    Release {
        tag_name: format!("v{version}"),
        draft: false,
        prerelease: false,
        assets: [name.clone(), format!("{name}.sha256")]
            .into_iter()
            .map(|name| Asset {
                browser_download_url: format!("{REPOSITORY}v{version}/{name}"),
                name,
                size: 100,
            })
            .collect(),
    }
}

fn installation(path: &Path) -> Installation {
    Installation {
        path: path.to_path_buf(),
        package: Package::AppImage,
    }
}

#[test]
fn only_newer_stable_releases_with_matching_assets_are_offered() {
    let current = Version::parse("0.1.0").unwrap();
    let install = installation(Path::new("/tmp/CatDo.AppImage"));
    for version in ["0.0.9", "0.1.0", "0.2.0-beta.1"] {
        assert!(
            select(release(version), &current, install.clone())
                .unwrap()
                .is_none()
        );
    }
    assert_eq!(
        select(release("0.2.0"), &current, install.clone())
            .unwrap()
            .unwrap()
            .version,
        Version::parse("0.2.0").unwrap()
    );
    let mut draft = release("0.2.0");
    draft.draft = true;
    assert!(select(draft, &current, install.clone()).unwrap().is_none());
    let mut incomplete = release("0.2.0");
    incomplete.assets.pop();
    assert!(select(incomplete, &current, install.clone()).is_err());
    let mut wrong_url = release("0.2.0");
    wrong_url.assets[0].browser_download_url = "https://example.com/update".into();
    assert!(select(wrong_url, &current, install).is_err());
}

#[test]
fn checksums_must_name_the_exact_asset() {
    let hash = "a".repeat(64);
    assert_eq!(
        checksum(&format!("{hash}  catdo.AppImage\n"), "catdo.AppImage").unwrap(),
        hash
    );
    assert!(checksum(&format!("{hash}  other.AppImage"), "catdo.AppImage").is_err());
    assert!(checksum("abc catdo.AppImage", "catdo.AppImage").is_err());
    assert!(
        checksum(
            &format!("{} catdo.AppImage", "z".repeat(64)),
            "catdo.AppImage"
        )
        .is_err()
    );
}

fn ready(directory: &Path) -> ReadyUpdate {
    let target = directory.join("CatDo with spaces.AppImage");
    fs::write(&target, b"old program").unwrap();
    let mut staged = NamedTempFile::new_in(directory).unwrap();
    staged.write_all(b"new program").unwrap();
    let update = select(
        release("0.2.0"),
        &Version::parse("0.1.0").unwrap(),
        installation(&target),
    )
    .unwrap()
    .unwrap();
    ReadyUpdate {
        digest: digest(staged.path()).unwrap(),
        update,
        staged: staged.into_temp_path(),
    }
}

#[test]
fn installation_is_atomic_preserves_old_binary_and_rejects_changed_download() {
    let directory = tempfile::tempdir().unwrap();
    let ready = ready(directory.path());
    // An already-open executable keeps referring to the previous inode.
    let mut running = File::open(&ready.update.installation.path).unwrap();
    let backup = ready.install().unwrap();
    assert_eq!(
        fs::read(&ready.update.installation.path).unwrap(),
        b"new program"
    );
    assert_eq!(fs::read(backup).unwrap(), b"old program");
    let mut old = String::new();
    running.read_to_string(&mut old).unwrap();
    assert_eq!(old, "old program");

    let changed = self::ready(directory.path());
    fs::write(&changed.staged, b"tampered").unwrap();
    assert!(changed.install().is_err());
    assert_eq!(
        fs::read(&changed.update.installation.path).unwrap(),
        b"old program"
    );
}

#[test]
fn failed_restart_restores_previous_executable() {
    let directory = tempfile::tempdir().unwrap();
    let ready = ready(directory.path());
    // This deliberately invalid executable makes exec fail, rather than replacing the test process.
    assert!(ready.install_and_restart().is_err());
    assert_eq!(
        fs::read(&ready.update.installation.path).unwrap(),
        b"old program"
    );
}

#[test]
fn restart_exec_child() {
    let Some(directory) = std::env::var_os("CATDO_RESTART_TEST_DIRECTORY") else {
        return;
    };
    let mut ready = ready(Path::new(&directory));
    fs::write(
        &ready.staged,
        b"#!/bin/sh\nprintf 'CatDo restarted successfully\\n'\n",
    )
    .unwrap();
    fs::set_permissions(&ready.staged, fs::Permissions::from_mode(0o755)).unwrap();
    ready.digest = digest(&ready.staged).unwrap();
    ready.install_and_restart().unwrap();
    panic!("exec must replace this process");
}

#[test]
fn successful_restart_executes_the_installed_file() {
    let directory = tempfile::tempdir().unwrap();
    let result = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "updates::tests::restart_exec_child",
            "--nocapture",
        ])
        .env("CATDO_RESTART_TEST_DIRECTORY", directory.path())
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(String::from_utf8_lossy(&result.stdout).contains("CatDo restarted successfully"));
}

#[gpui_kit::test]
fn install_click_saves_or_blocks_on_invalid_drafts_and_busy_clicks_do_nothing(
    cx: &mut gpui_kit::TestAppContext,
) {
    use crate::{app::CatDo, update_ui::UpdateState};
    let directory = tempfile::tempdir().unwrap();
    let staged = ready(directory.path());
    let target = staged.update.installation.path.clone();
    let mut store = catdo_core::Store::open(&directory.path().join("catdo.sqlite3")).unwrap();
    let data = store.load().unwrap();
    cx.update(gpui_kit::init);
    let window = cx.add_window(|window, cx| CatDo::new(store, data, window, cx));
    window
        .update(cx, |this, window, cx| {
            this.update_state = UpdateState::Downloading;
            this.click_update(cx);
            assert!(matches!(this.update_state, UpdateState::Downloading));
            assert_eq!(fs::read(&target).unwrap(), b"old program");
            this.update_state = UpdateState::Ready(staged);
            this.new_task(window, cx);
            this.click_update(cx);
            assert!(this.editor.is_some(), "An invalid draft must block restart");
            assert!(matches!(this.update_state, UpdateState::Ready(_)));
            assert_eq!(fs::read(&target).unwrap(), b"old program");
            this.editor = None;
            this.click_update(cx);
            // Our non-executable fixture exercises install failure and retry UI.
            assert!(matches!(
                this.update_state,
                UpdateState::Failed { retry: Some(_), .. }
            ));
            assert_eq!(fs::read(&target).unwrap(), b"old program");
        })
        .unwrap();
}

#[test]
fn archive_extracts_only_the_expected_regular_binary() {
    let directory = tempfile::tempdir().unwrap();
    let archive = directory.path().join("update.tar.gz");
    let writer = flate2::write::GzEncoder::new(
        File::create(&archive).unwrap(),
        flate2::Compression::default(),
    );
    let mut tar = tar::Builder::new(writer);
    for (path, content) in [
        ("unrelated", b"ignore".as_slice()),
        ("catdo-0.2.0-linux-x86_64/catdo", b"program".as_slice()),
    ] {
        let mut header = tar::Header::new_gnu();
        header.set_mode(0o755);
        header.set_size(content.len() as u64);
        header.set_cksum();
        tar.append_data(&mut header, path, content).unwrap();
    }
    tar.into_inner().unwrap().finish().unwrap();
    let mut output = Vec::new();
    extract_binary(&archive, &Version::parse("0.2.0").unwrap(), &mut output).unwrap();
    assert_eq!(output, b"program");
    assert!(extract_binary(&archive, &Version::parse("0.3.0").unwrap(), &mut Vec::new()).is_err());
}

// Explicit release smoke check; normal tests never use the network or execute
// downloaded files. Both packages are installed into disposable directories.
#[test]
#[ignore = "downloads the latest official release from GitHub"]
fn official_release_download_install_and_version() {
    for package in [Package::Binary, Package::AppImage] {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("CatDo with spaces");
        fs::write(&target, b"previous program").unwrap();
        let release = client()
            .unwrap()
            .get("https://api.github.com/repos/workercatstudios/catdo/releases/latest")
            .send()
            .unwrap()
            .error_for_status()
            .unwrap()
            .json::<Release>()
            .unwrap();
        let update = select(
            release,
            &Version::parse("0.0.0").unwrap(),
            Installation {
                path: target.clone(),
                package,
            },
        )
        .unwrap()
        .unwrap();
        let version = update.version.clone();
        let ready = download(update).unwrap();
        assert_eq!(
            fs::read(&target).unwrap(),
            b"previous program",
            "Downloading must never install the update"
        );
        ready.install().unwrap();
        let output = Command::new(target)
            .arg("--version")
            .env("APPIMAGE_EXTRACT_AND_RUN", "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            format!("CatDo {version}")
        );
    }
}
