use catdo_core::Store;
use gpui_kit::component::{ActiveTheme, ThemeMode};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{App, AppContext, Hsla, Rgba, TestAppContext, WindowAppearance, rgb};
use kirakira::Root;

use crate::{
    app::CatDo,
    theme::{Accent, Appearance, accent, chip},
};

fn hex(color: Hsla) -> u32 {
    let c = Rgba::from(color);
    let byte = |v: f32| (v * 255.).round() as u32;
    byte(c.r) << 16 | byte(c.g) << 8 | byte(c.b)
}

/// WCAG contrast of `color` on `background`, with `color`'s alpha composited over it.
fn contrast(color: Hsla, background: Hsla) -> f32 {
    let (fg, bg) = (Rgba::from(color), Rgba::from(background));
    let mix = |f: f32, b: f32| f * fg.a + b * (1. - fg.a);
    let luminance = |r: f32, g: f32, b: f32| {
        let lin = |v: f32| {
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
    };
    let a = luminance(mix(fg.r, bg.r), mix(fg.g, bg.g), mix(fg.b, bg.b));
    let b = luminance(bg.r, bg.g, bg.b);
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// Text in a meaning colour, on the page and on its own pastel chip.
fn assert_meanings_readable(cx: &App) {
    let background = cx.theme().background;
    for meaning in [
        Accent::Today,
        Accent::Inbox,
        Accent::Upcoming,
        Accent::Calendar,
        Accent::Overdue,
    ] {
        let tone = accent(meaning, cx);
        let chip = {
            let wash = Rgba::from(chip(meaning, cx));
            let bg = Rgba::from(background);
            let mix = |f: f32, b: f32| f * wash.a + b * (1. - wash.a);
            Hsla::from(Rgba {
                r: mix(wash.r, bg.r),
                g: mix(wash.g, bg.g),
                b: mix(wash.b, bg.b),
                a: 1.,
            })
        };
        assert!(
            contrast(tone, background) >= 4.5,
            "{meaning:?} text on the page: {}",
            contrast(tone, background)
        );
        assert!(
            contrast(tone, chip) >= 4.5,
            "{meaning:?} text on its chip: {}",
            contrast(tone, chip)
        );
    }
    assert!(contrast(accent(Accent::Done, cx), background) >= 3.0);
}

#[gpui_kit::test]
fn kirakira_theme_and_meaning_colours_in_both_modes(cx: &mut TestAppContext) {
    let temp = tempfile::tempdir().unwrap();
    let mut store = Store::open(&temp.path().join("theme.sqlite3")).unwrap();
    let data = store.load().unwrap();
    cx.update(gpui_kit::init);
    let window = cx.add_window(|window, cx| CatDo::new(store, data, window, cx));
    window
        .update(cx, |_, window, cx| {
            Appearance::Light.apply(window, cx);
            let theme = cx.theme();
            assert_eq!(hex(theme.primary), 0xD6336F, "Kirakira's pink primary");
            assert_eq!(hex(theme.background), 0xFFFCF8, "Kirakira's warm paper");
            assert_eq!(hex(theme.foreground), 0x4B3832, "Kirakira's cocoa ink");
            assert_eq!(hex(theme.sidebar), 0xFBF6F1);
            assert_eq!(
                theme.secondary_active, theme.accent,
                "selection is Kirakira's blush, never its teal plate"
            );
            assert_meanings_readable(cx);

            Appearance::Dark.apply(window, cx);
            let theme = cx.theme();
            assert_eq!(hex(theme.primary), 0xFF7AA5);
            assert_eq!(hex(theme.background), 0x1F2238, "Kirakira's navy night");
            assert_eq!(theme.secondary_active, theme.accent);
            assert_meanings_readable(cx);
            assert_eq!(accent(Accent::Done, cx), cx.theme().primary);
            assert_eq!(accent(Accent::Project(7), cx), rgb(0x8FA6D6).into());
        })
        .unwrap();
}

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
    cx.update_window(root.into(), |_, window, cx| window.click("settings", cx))
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
