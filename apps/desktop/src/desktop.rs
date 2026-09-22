//! Window lifetime is separate from CatDo's model: closing to the tray keeps
//! sync and reminders running, without retaining a hidden native window.
use crate::{
    app::CatDo,
    instance::Instance,
    tray::{CatTray, Event},
};
use gpui_kit::{
    App, AppContext, Bounds, Entity, Global, TitlebarOptions, WindowBounds, WindowOptions,
    component::{Root, TitleBar},
    px, size,
};
use ksni::TrayMethods;
use std::{sync::mpsc, time::Duration};

struct Desktop {
    view: Entity<CatDo>,
    tray_online: bool,
}
impl Global for Desktop {}

pub fn window_options(cx: &App) -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
            None,
            size(px(1240.), px(820.)),
            cx,
        ))),
        window_decorations: Some(gpui_kit::WindowDecorations::Client),
        window_min_size: Some(size(px(980.), px(650.))),
        titlebar: Some(TitlebarOptions {
            title: Some("CatDo".into()),
            ..TitleBar::title_bar_options()
        }),
        app_id: Some("com.workercat.catdo".into()),
        ..TitleBar::window_options()
    }
}

pub fn start(view: Entity<CatDo>, instance: Instance, cx: &mut App) {
    // Linux otherwise quits automatically after our window-close observer runs.
    cx.set_quit_mode(gpui_kit::QuitMode::Explicit);
    cx.set_global(Desktop {
        view: view.clone(),
        tray_online: false,
    });
    view.update(cx, |this, cx| this.start_updates(cx));
    cx.on_window_closed(|cx, _| {
        if cx.windows().is_empty() && !cx.global::<Desktop>().tray_online {
            cx.quit();
        }
    })
    .detach();
    let (sender, receiver) = mpsc::channel();
    let tray = cx
        .background_executor()
        .spawn(async move { CatTray(sender).spawn().await });
    cx.spawn(async move |cx| {
        let _handle = match tray.await {
            Ok(handle) => {
                cx.update(|cx| cx.global_mut::<Desktop>().tray_online = true);
                Some(handle)
            }
            Err(error) => {
                eprintln!("System tray unavailable; closing the window will quit CatDo: {error}");
                None
            }
        };
        loop {
            cx.background_executor()
                .timer(Duration::from_millis(200))
                .await;
            let mut events: Vec<_> = receiver.try_iter().collect();
            if instance.take_show_request() {
                events.push(Event::Show);
            }
            cx.update(|cx| {
                for event in events {
                    handle_event(event, cx);
                }
            });
        }
    })
    .detach();
}

fn handle_event(event: Event, cx: &mut App) {
    match event {
        Event::Show => show(cx),
        Event::Quit => {
            let view = cx.global::<Desktop>().view.clone();
            if view.update(cx, |this, cx| this.save_before_close(cx)) {
                cx.quit();
            } else {
                show(cx);
            }
        }
        Event::Offline => {
            cx.global_mut::<Desktop>().tray_online = false;
            // A disappearing tray must never leave an inaccessible background app.
            if cx.windows().is_empty() {
                show(cx);
            }
        }
        Event::Online => cx.global_mut::<Desktop>().tray_online = true,
    }
}

fn show(cx: &mut App) {
    if let Some(window) = cx.windows().first().copied() {
        let _ = window.update(cx, |_, window, _| window.activate_window());
        return;
    }
    let view = cx.global::<Desktop>().view.clone();
    if let Err(error) = cx.open_window(window_options(cx), |window, cx| {
        view.update(cx, |this, cx| this.reopen(window, cx));
        cx.new(|cx| Root::new(view, window, cx))
    }) {
        eprintln!("Could not reopen CatDo: {error:#}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use catdo_core::Store;
    use gpui_kit::component::input::InputEvent;
    use gpui_kit::test::TestWindowExt;
    use gpui_kit::{TestAppContext, VisualTestContext};

    #[gpui_kit::test]
    fn close_reopen_retains_tasks_undo_and_rebinds_input(cx: &mut TestAppContext) {
        let temp = tempfile::tempdir().unwrap();
        let mut store = Store::open(&temp.path().join("catdo.sqlite3")).unwrap();
        let data = store.load().unwrap();
        cx.update(gpui_kit::init);
        let mut view = None;
        let window = cx.add_window(|window, cx| {
            let app = cx.new(|cx| CatDo::new(store, data, window, cx));
            view = Some(app.clone());
            Root::new(app, window, cx)
        });
        let view = view.unwrap();
        cx.update(|cx| {
            cx.set_global(Desktop {
                view: view.clone(),
                tray_online: true,
            })
        });
        cx.update_window(window.into(), |_, window, cx| {
            view.update(cx, |this, cx| {
                this.quick_add.update(cx, |input, cx| {
                    input.set_value("Keep this task", window, cx)
                });
                this.quick_create(window, cx);
                this.quick_add
                    .update(cx, |input, cx| input.set_value("Still typing", window, cx));
            });
            window.render_frame(cx);
        })
        .unwrap();
        assert!(VisualTestContext::from_window(window.into(), cx).simulate_close());
        cx.update_window(window.into(), |_, window, _| window.remove_window())
            .unwrap();
        cx.run_until_parked();
        cx.update(|cx| {
            assert!(cx.windows().is_empty());
            assert_eq!(view.read(cx).data.tasks.len(), 1);
            assert_eq!(view.read(cx).undo.len(), 1);
            // Losing the tray must reveal a closed window automatically.
            handle_event(Event::Offline, cx);
            assert_eq!(cx.windows().len(), 1);
            assert!(!cx.global::<Desktop>().tray_online);
        });
        let reopened = cx.update(|cx| cx.windows()[0]);
        cx.update_window(reopened, |_, window, cx| {
            let input = view.read(cx).quick_add.clone();
            assert_eq!(input.read(cx).value(), "Still typing");
            input.update(cx, |_, cx| {
                cx.emit(InputEvent::PressEnter {
                    secondary: false,
                    shift: false,
                })
            });
            window.render_frame(cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update(|cx| {
            assert_eq!(view.read(cx).data.tasks.len(), 2);
            assert_eq!(view.read(cx).data.tasks[1].title, "Still typing");
        });
    }

    #[gpui_kit::test]
    fn invalid_editor_blocks_window_close(cx: &mut TestAppContext) {
        let temp = tempfile::tempdir().unwrap();
        let mut store = Store::open(&temp.path().join("catdo.sqlite3")).unwrap();
        let data = store.load().unwrap();
        cx.update(gpui_kit::init);
        let mut view = None;
        let window = cx.add_window(|window, cx| {
            let app = cx.new(|cx| CatDo::new(store, data, window, cx));
            view = Some(app.clone());
            app.update(cx, |this, cx| this.new_task(window, cx));
            Root::new(app, window, cx)
        });
        cx.update_window(window.into(), |_, window, cx| {
            window.render_frame(cx);
        })
        .unwrap();
        assert!(!VisualTestContext::from_window(window.into(), cx).simulate_close());
        cx.run_until_parked();
        cx.update(|cx| assert_eq!(cx.windows().len(), 1));
    }
}
