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
    if let Err(error) = run() {
        eprintln!("CatDo could not start: {error:#}");
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
    let Some(instance) = instance::Instance::claim(&path)? else {
        return Ok(());
    };
    let mut store = Store::open(&path.join("catdo.sqlite3"))?;
    let data = store.load()?;
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
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
