use gpui_kit::component::{ActiveTheme, Selectable, StyledExt, Theme, ThemeMode};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    App, Context, Hsla, IntoElement, ParentElement, Styled, Window, WindowAppearance, div, rgb,
};
use kirakira::button::{Button, ButtonGroup, ButtonVariants as _};
use serde::{Deserialize, Serialize};

use crate::app::CatDo;

/// Kirakira's typeface. Static Latin weights, because GPUI can't pick weights from a
/// variable font; other scripts fall back to the system font.
pub(crate) const FONT: &str = "M PLUS 1";
/// Set once the font is registered, so a failed load keeps the system font.
static FONT_LOADED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub(crate) fn load_fonts(cx: &mut App) {
    let fonts = [
        include_bytes!("../../../assets/fonts/MPLUS1-400.ttf").as_slice(),
        include_bytes!("../../../assets/fonts/MPLUS1-500.ttf").as_slice(),
        include_bytes!("../../../assets/fonts/MPLUS1-700.ttf").as_slice(),
        include_bytes!("../../../assets/fonts/MPLUS1-800.ttf").as_slice(),
    ];
    match cx
        .text_system()
        .add_fonts(fonts.into_iter().map(std::borrow::Cow::Borrowed).collect())
    {
        Ok(()) => FONT_LOADED.store(true, std::sync::atomic::Ordering::Relaxed),
        Err(error) => eprintln!("Could not load the M PLUS 1 font: {error:#}"),
    }
}

/// A device preference: System keeps following live OS appearance changes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Appearance {
    #[default]
    System,
    Light,
    Dark,
}

impl Appearance {
    const ALL: [Self; 3] = [Self::System, Self::Light, Self::Dark];

    pub(crate) fn resolve(self, system: WindowAppearance) -> ThemeMode {
        match self {
            Self::System => system.into(),
            Self::Light => ThemeMode::Light,
            Self::Dark => ThemeMode::Dark,
        }
    }

    /// Installs Kirakira's light or dark theme (warm paper and cocoa ink, or a navy night, with
    /// a pink primary) into GPUI Kit, so every control and surface follows it.
    pub(crate) fn apply(self, window: &mut Window, cx: &mut App) {
        kirakira::theme::apply(self.resolve(window.appearance()), Some(window), cx);
        let theme = Theme::global_mut(cx);
        // GPUI Component draws a selected ghost button (navigation, the appearance toggle) with
        // `secondary_active`, Kirakira's teal plate. CatDo selects with Kirakira's blush accent,
        // as the web and Android clients do.
        theme.secondary_active = theme.accent;
        if FONT_LOADED.load(std::sync::atomic::Ordering::Relaxed) {
            theme.font_family = FONT.into();
        }
        // CatDo's clients share a sidebar a step warmer than the page in light mode, so the
        // 240px rail reads as its own region.
        if !theme.is_dark() {
            theme.sidebar = rgb(0xFBF6F1).into();
            theme.title_bar = theme.sidebar;
        }
        theme.tokens = theme.colors.into();
        Theme::sync_base(cx);
        window.refresh();
    }

    fn label(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Light => "Light",
            Self::Dark => "Dark",
        }
    }
}

/// Colour with a job: each view, date kind, and project carries its own Kirakira hue, while
/// actions and selection use the theme's pink primary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Accent {
    Today,
    Inbox,
    Upcoming,
    Calendar,
    /// A finished check: the pink primary.
    Done,
    Scheduled,
    Repeat,
    /// Late deadlines and errors.
    Overdue,
    Project(usize),
}

/// Project slots p0..p7 on Kirakira's hues, shared with the web and Android clients: pink, sky,
/// teal, orange, lilac, yellow, lime and navy. Light yellow and lime are deepened a little so a dot
/// still shows on paper; dark navy is lightened so it shows on the navy page.
const PROJECT_LIGHT: [u32; 8] = [
    0xEC5F8F, 0x5AA9E6, 0x4FB0AA, 0xF4A35F, 0xB79AD1, 0xE0B531, 0xE8775F, 0x2D3F63,
];
const PROJECT_DARK: [u32; 8] = [
    0xFF7AA5, 0x5AA9E6, 0x4FB0AA, 0xF4A35F, 0xB79AD1, 0xF7D35C, 0xFF9B85, 0x8FA6D6,
];

/// Stable colour slot for a project, using the same hash as the web and
/// Android clients so a project looks the same everywhere.
pub fn project_color_index(id: uuid::Uuid) -> usize {
    let hash = id.to_string().bytes().fold(0u32, |hash, byte| {
        hash.wrapping_mul(31).wrapping_add(byte as u32)
    });
    (hash % PROJECT_LIGHT.len() as u32) as usize
}

/// The text-safe tone of a meaning: 4.5:1 on paper and on the navy page, so it can colour text,
/// icons and chips. The tones are shared with the web and Android clients.
pub fn accent(accent: Accent, cx: &App) -> Hsla {
    let dark = cx.theme().is_dark();
    let (light, dark_color) = match accent {
        Accent::Done => return cx.theme().primary,
        Accent::Today => (0xA35A12, 0xF7D35C),
        Accent::Inbox | Accent::Scheduled => (0x2F6FB0, 0x8CC4F0),
        Accent::Upcoming => (0x7A52A3, 0xC9B0E3),
        Accent::Calendar | Accent::Repeat => (0x2B7A75, 0x7FD0C9),
        Accent::Overdue => (0xC2333A, 0xFF8A8D),
        Accent::Project(index) => (
            PROJECT_LIGHT[index % PROJECT_LIGHT.len()],
            PROJECT_DARK[index % PROJECT_DARK.len()],
        ),
    };
    rgb(if dark { dark_color } else { light }).into()
}

/// A meaning's `--kk-*` hue: Kirakira yellow for Today, sky for Inbox and dates, lilac for
/// Upcoming, teal for the calendar and repeats, pink for done, red for late, and a project's slot.
fn pastel(accent: Accent, dark: bool) -> Hsla {
    match accent {
        Accent::Today => rgb(0xF7D35C),
        Accent::Inbox | Accent::Scheduled => rgb(0x5AA9E6),
        Accent::Upcoming => rgb(0xB79AD1),
        Accent::Calendar | Accent::Repeat => rgb(0x4FB0AA),
        Accent::Done => rgb(if dark { 0xFF7AA5 } else { 0xEC5F8F }),
        Accent::Overdue => rgb(if dark { 0xFF6B6F } else { 0xE5484D }),
        Accent::Project(index) => {
            rgb(if dark { PROJECT_DARK } else { PROJECT_LIGHT }[index % PROJECT_LIGHT.len()])
        }
    }
    .into()
}

/// The pastel tile behind a meaning's icon.
pub fn fill(accent: Accent, cx: &App) -> Hsla {
    let dark = cx.theme().is_dark();
    pastel(accent, dark).opacity(if dark { 0.2 } else { 0.18 })
}

/// The lighter wash behind text in a meaning's tone, which keeps that text at 4.5:1.
pub fn chip(accent: Accent, cx: &App) -> Hsla {
    let dark = cx.theme().is_dark();
    pastel(accent, dark).opacity(if dark { 0.18 } else { 0.1 })
}

impl CatDo {
    pub(crate) fn set_appearance(
        &mut self,
        appearance: Appearance,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Err(error) = self.store.set_preference("appearance", &appearance) {
            self.error(format!("Could not save appearance: {error}"), cx);
            return;
        }
        self.appearance = appearance;
        appearance.apply(window, cx);
        cx.notify();
    }

    pub(crate) fn render_theme_control(&self, cx: &Context<Self>) -> impl IntoElement {
        div().h_flex().py_2().child(
            ButtonGroup::new("appearance")
                .children(Appearance::ALL.map(|appearance| {
                    // The chosen segment is filled pink, readable in both themes.
                    Button::new(appearance.label())
                        .label(appearance.label())
                        .when(self.appearance == appearance, |button| button.primary())
                        .selected(self.appearance == appearance)
                        .tooltip(match appearance {
                            Appearance::System => "Follow your system’s light or dark theme",
                            Appearance::Light => "Always use the light theme",
                            Appearance::Dark => "Always use the dark theme",
                        })
                }))
                .on_click(cx.listener(|this, selected: &Vec<usize>, window, cx| {
                    if let Some(appearance) = selected.first().and_then(|i| Appearance::ALL.get(*i))
                    {
                        this.set_appearance(*appearance, window, cx);
                    }
                })),
        )
    }
}
