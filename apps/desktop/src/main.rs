// Release builds on Windows open no console window.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod calendar;
mod cloud;
mod commands;
mod desktop;
mod editor;
mod instance;
mod management;
mod reminders;
mod shell;
mod sidebar;
mod sync_ui;
mod task_list;
#[cfg(test)]
mod tests;
mod theme;
#[cfg(test)]
mod theme_tests;
mod tray;
mod update_ui;
mod updates;

use anyhow::{Context, Result};
use catdo_core::Store;
use directories::ProjectDirs;
use gpui_kit::component::Root;
use gpui_kit::*;

fn main() {
    #[cfg(windows)]
    windows::attach_console();
    if let Err(error) = run() {
        let message = format!("CatDo could not start: {error:#}");
        eprintln!("{message}");
        #[cfg(windows)]
        windows::alert_without_console(&message);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let path = match args.next().as_deref() {
        Some("--version") => {
            println!("CatDo {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        Some("--data-dir") => {
            std::path::PathBuf::from(args.next().context("--data-dir requires a directory")?)
        }
        Some("--help") => {
            println!(
                "CatDo\n\nUsage: catdo [--data-dir DIRECTORY] [--help] [--version]\n\nThe default database is stored in the user's local application data directory."
            );
            return Ok(());
        }
        Some(arg) => anyhow::bail!("Unknown argument: {arg}"),
        None => ProjectDirs::from("com", "workercat", "catdo")
            .context("Could not find the application data directory")?
            .data_local_dir()
            .to_path_buf(),
    };
    let restarted = std::env::var_os(updates::RESTARTED).is_some();
    let Some(instance) = instance::Instance::claim(&path, restarted)? else {
        return Ok(());
    };
    #[cfg(windows)]
    windows::register_notification_icon(&path);
    let mut store = Store::open(&path.join("catdo.sqlite3"))?;
    let data = store.load()?;
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
            // Groups CatDo's windows and names its notifications.
            #[cfg(windows)]
            cx.set_app_identity("com.workercat.catdo", "CatDo");
            app::bind_keys(cx);
            let options = desktop::window_options(cx);
            if let Err(error) = cx.open_window(options, |window, cx| {
                let view = cx.new(|cx| app::CatDo::new(store, data, window, cx));
                desktop::start(view.clone(), instance, cx);
                cx.new(|cx| Root::new(view, window, cx))
            }) {
                eprintln!("Could not open CatDo: {error:#}");
                cx.quit();
            }
        });
    Ok(())
}

#[cfg(windows)]
mod windows {
    use std::{fs, path::Path};
    use windows_sys::Win32::{
        System::{
            Console::{
                ATTACH_PARENT_PROCESS, AttachConsole, GetStdHandle, STD_ERROR_HANDLE,
                STD_OUTPUT_HANDLE,
            },
            Registry::{HKEY_CURRENT_USER, REG_SZ, RegSetKeyValueW},
        },
        UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW},
    };

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain([0]).collect()
    }

    /// GPUI registers the name shown on notifications; this adds the icon,
    /// which Windows reads from a file.
    pub fn register_notification_icon(directory: &Path) {
        let png = include_bytes!("../../../assets/com.workercat.catdo.png");
        let brand = directory.join("brand");
        let icon = brand.join("icon.png");
        if fs::read(&icon).ok().as_deref() != Some(png.as_slice())
            && fs::create_dir_all(&brand)
                .and_then(|()| fs::write(&icon, png))
                .is_err()
        {
            return;
        }
        let value = wide(&icon.to_string_lossy());
        unsafe {
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                wide(r"Software\Classes\AppUserModelId\com.workercat.catdo").as_ptr(),
                wide("IconUri").as_ptr(),
                REG_SZ,
                value.as_ptr().cast(),
                (value.len() * 2) as u32,
            );
        }
    }

    /// Without a console window, `--help` and errors print to the terminal
    /// that started CatDo, if any.
    pub fn attach_console() {
        unsafe {
            if GetStdHandle(STD_OUTPUT_HANDLE).is_null() {
                AttachConsole(ATTACH_PARENT_PROCESS);
            }
        }
    }

    /// Reports a startup failure that would otherwise go unseen.
    pub fn alert_without_console(message: &str) {
        unsafe {
            if GetStdHandle(STD_ERROR_HANDLE).is_null() {
                MessageBoxW(
                    std::ptr::null_mut(),
                    wide(message).as_ptr(),
                    wide("CatDo").as_ptr(),
                    MB_ICONERROR | MB_OK,
                );
            }
        }
    }
}
