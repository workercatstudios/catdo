//! The Kirakira theme: warm paper, cocoa ink and a pink primary.
//!
//! Surfaces and text come from GPUI Component's theme (`cx.theme()`), the same way the web
//! components read shadcn's tokens. [`apply`] installs the Kirakira light and dark themes there.
//! The decorative colours and the four motion curves are Kirakira's own: read them with
//! [`ActiveKira::kira`] and [`ActiveKira::curves`].

use std::rc::Rc;

use gpui_kit::component::{ActiveTheme as _, Theme, ThemeMode, ThemeRegistry};
use gpui_kit::{App, Global, Hsla, Window, rgb};

use crate::motion::Easing;

/// The Kirakira theme set in GPUI Component's theme format.
pub const THEME_JSON: &str = include_str!("../themes/kirakira.json");

/// Names of the two themes in [`THEME_JSON`].
pub const LIGHT_THEME: &str = "Kirakira Light";
pub const DARK_THEME: &str = "Kirakira Dark";

/// Kirakira's decorative colours, the `--kk-*` variables of the web theme.
///
/// `--primary` is a deeper raspberry so white text on it passes WCAG AA; [`Palette::pink`] is the
/// brighter decoration pink for sparkles, confetti, plates and hearts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    /// Text and outlines: cocoa instead of black.
    pub ink: Hsla,
    /// The page background.
    pub paper: Hsla,
    /// Hearts, highlights, decoration.
    pub pink: Hsla,
    /// Tints and accent surfaces.
    pub blush: Hsla,
    /// Sparkles and confetti.
    pub yellow: Hsla,
    /// Warm accents.
    pub orange: Hsla,
    /// Secondary plates.
    pub teal: Hsla,
    /// Confetti and cool accents.
    pub sky: Hsla,
    /// Confetti and fresh accents.
    pub lime: Hsla,
    /// Soft accents.
    pub lilac: Hsla,
    /// Dark plates and lids.
    pub navy: Hsla,
}

impl Palette {
    /// The light palette.
    pub fn light() -> Self {
        Self {
            ink: rgb(0x4b3832).into(),
            paper: rgb(0xfffcf8).into(),
            pink: rgb(0xec5f8f).into(),
            blush: rgb(0xfde6ee).into(),
            yellow: rgb(0xf7d35c).into(),
            orange: rgb(0xf4a35f).into(),
            teal: rgb(0x4fb0aa).into(),
            sky: rgb(0x5aa9e6).into(),
            lime: rgb(0xb9cc5a).into(),
            lilac: rgb(0xb79ad1).into(),
            navy: rgb(0x2d3f63).into(),
        }
    }

    /// The dark palette: a navy night, warm off-white ink and a lighter pink.
    pub fn dark() -> Self {
        Self {
            ink: rgb(0xf6ece4).into(),
            paper: rgb(0x1f2238).into(),
            pink: rgb(0xff7aa5).into(),
            blush: rgb(0x3b2f4a).into(),
            ..Self::light()
        }
    }

    /// The confetti colours, in the order bursts and sparkles cycle through them.
    pub fn confetti(&self) -> [Hsla; 6] {
        [
            self.pink,
            self.yellow,
            self.teal,
            self.sky,
            self.lime,
            self.lilac,
        ]
    }
}

/// The four Kirakira curves. Overshoot is written into the keyframes, so most keyframed motion runs
/// on plain `ease-in-out`; these are for the rest.
#[derive(Clone, Debug)]
pub struct Curves {
    /// `cubic-bezier(0.85, 0, 0.15, 1)`: wipes, panels and scene changes. A fast middle with soft
    /// ends.
    pub snap: Easing,
    /// `cubic-bezier(0.05, 0.3, 0.1, 1)`: arrivals. Letters flying in, rings expanding, lids
    /// opening.
    pub out: Easing,
    /// `cubic-bezier(0.8, 0, 1, 1)`: falls and exits.
    pub r#in: Easing,
    /// `cubic-bezier(0.34, 1.56, 0.64, 1)`: hover and UI transitions that overshoot a little.
    pub spring: Easing,
}

impl Default for Curves {
    fn default() -> Self {
        // Kirakira's spring overshoots: use the unclamped curve.
        let bezier = crate::motion::cubic_bezier;
        Self {
            snap: bezier(0.85, 0.0, 0.15, 1.0),
            out: bezier(0.05, 0.3, 0.1, 1.0),
            r#in: bezier(0.8, 0.0, 1.0, 1.0),
            spring: bezier(0.34, 1.56, 0.64, 1.0),
        }
    }
}

/// The global Kirakira tokens. Change them with [`Kira::global_mut`] after [`crate::init`].
#[derive(Clone, Debug)]
pub struct Kira {
    pub light: Palette,
    pub dark: Palette,
    pub curves: Curves,
}

impl Default for Kira {
    fn default() -> Self {
        Self {
            light: Palette::light(),
            dark: Palette::dark(),
            curves: Curves::default(),
        }
    }
}

impl Global for Kira {}

impl Kira {
    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    pub fn global_mut(cx: &mut App) -> &mut Self {
        cx.default_global::<Self>()
    }
}

/// Reads the Kirakira tokens for the current light or dark mode.
pub trait ActiveKira {
    /// The decorative palette for the current mode.
    fn kira(&self) -> Palette;
    /// The Kirakira motion curves.
    fn curves(&self) -> Curves;
}

impl ActiveKira for App {
    fn kira(&self) -> Palette {
        let dark = self.theme().is_dark();
        match self.try_global::<Kira>() {
            Some(kira) if dark => kira.dark,
            Some(kira) => kira.light,
            None if dark => Palette::dark(),
            None => Palette::light(),
        }
    }

    fn curves(&self) -> Curves {
        self.try_global::<Kira>()
            .map(|kira| kira.curves.clone())
            .unwrap_or_default()
    }
}

/// Installs the Kirakira light and dark themes into GPUI Component and switches to `mode`.
///
/// Stock GPUI Component pieces pick the theme up too, so they sit beside Kirakira's.
pub fn apply(mode: ThemeMode, window: Option<&mut Window>, cx: &mut App) {
    let registry = ThemeRegistry::global_mut(cx);
    registry
        .load_themes_from_str(THEME_JSON)
        .expect("the bundled Kirakira theme parses");
    let light = Rc::clone(&registry.themes()[LIGHT_THEME]);
    let dark = Rc::clone(&registry.themes()[DARK_THEME]);
    let theme = Theme::global_mut(cx);
    theme.light_theme = light;
    theme.dark_theme = dark;
    Theme::change(mode, window, cx);
}
