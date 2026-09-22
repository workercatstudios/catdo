use catdo_core::Store;
use gpui_kit::component::{ActiveTheme, Root, ThemeMode};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{AppContext, TestAppContext, WindowAppearance};

use crate::{app::CatDo, theme::Appearance};

#[test]
fn system_resolves_light_dark_and_vibrant_appearances() {
    for (system, expected) in [
        (WindowAppearance::Light, ThemeMode::Light),
        (WindowAppearance::Dark, ThemeMode::Dark),
        (WindowAppearance::VibrantLight, ThemeMode::Light),
        (WindowAppearance::VibrantDark, ThemeMode::Dark),
    ] {
        assert_eq!(Appearance::System.resolve(system), expected);
        assert_eq!(Appearance::Light.resolve(system), ThemeMode::Light);
        assert_eq!(Appearance::Dark.resolve(system), ThemeMode::Dark);
    }
}

#[gpui_kit::test]
fn theme_control_switches_modes_and_restores_saved_choice(cx: &mut TestAppContext) {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("theme.sqlite3");
    let mut store = Store::open(&path).unwrap();
    let data = store.load().unwrap();
    cx.update(gpui_kit::init);
    let mut app = None;
    let root = cx.add_window(|window, cx| {
        let entity = cx.new(|cx| CatDo::new(store, data, window, cx));
        app = Some(entity.clone());
        Root::new(entity, window, cx)
    });
    let app = app.unwrap();
    cx.update_window(root.into(), |_, window, cx| {
        assert_eq!(app.read(cx).appearance, Appearance::System);
        assert!(!cx.theme().is_dark());
        window.render_frame(cx);
    })
    .unwrap();
    cx.update_window(root.into(), |_, window, cx| window.click("Dark", cx))
        .unwrap();
    cx.update(|cx| {
        assert_eq!(app.read(cx).appearance, Appearance::Dark);
        assert!(cx.theme().is_dark());
    });
    cx.update_window(root.into(), |_, window, cx| window.click("Light", cx))
        .unwrap();
    cx.update(|cx| {
        assert_eq!(app.read(cx).appearance, Appearance::Light);
        assert!(!cx.theme().is_dark());
    });
    cx.update_window(root.into(), |_, window, cx| window.click("Dark", cx))
        .unwrap();
    cx.update_window(root.into(), |_, window, cx| window.click("System", cx))
        .unwrap();
    cx.update(|cx| {
        assert_eq!(app.read(cx).appearance, Appearance::System);
        assert!(
            !cx.theme().is_dark(),
            "System must immediately restore the window’s light appearance"
        );
        assert_eq!(
            app.read(cx)
                .store
                .preference::<Appearance>("appearance")
                .unwrap(),
            Some(Appearance::System)
        );
    });
    cx.update_window(root.into(), |_, window, cx| window.click("Dark", cx))
        .unwrap();

    let mut reopened = Store::open(&path).unwrap();
    assert_eq!(
        reopened.preference::<Appearance>("appearance").unwrap(),
        Some(Appearance::Dark)
    );
    let data = reopened.load().unwrap();
    let window = cx.add_window(|window, cx| CatDo::new(reopened, data, window, cx));
    window
        .update(cx, |app, _, cx| {
            assert_eq!(app.appearance, Appearance::Dark);
            assert!(
                cx.theme().is_dark(),
                "The saved override must be restored before first render"
            );
        })
        .unwrap();
}
