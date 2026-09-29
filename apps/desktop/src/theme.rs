use gpui_kit::component::{
    ActiveTheme, Selectable, StyledExt, Theme, ThemeMode,
    button::{Button, ButtonGroup},
};
use gpui_kit::{App, Context, IntoElement, ParentElement, Styled, Window, WindowAppearance, div};
use serde::{Deserialize, Serialize};

use crate::app::CatDo;

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

    pub(crate) fn apply(self, window: &mut Window, cx: &mut App) {
        Theme::change(self.resolve(window.appearance()), Some(window), cx);
        apply_palette(cx);
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

/// Keep native Kit controls and application surfaces on the same palette.
fn apply_palette(cx: &mut App) {
    use gpui_kit::{px, rgb};
    let theme = Theme::global_mut(cx);
    let dark = theme.is_dark();
    let color = |light, dark_color| rgb(if dark { dark_color } else { light }).into();
    let canvas = color(0xFFFFFF, 0x1B1B1A);
    let sidebar = color(0xF6F6F4, 0x171716);
    let surface = color(0xF6F6F4, 0x232322);
    let foreground = color(0x232323, 0xEDEDEA);
    let muted = color(0x6B6B67, 0xA3A39F);
    let border = color(0xE7E7E3, 0x333331);
    let primary = color(0x262626, 0xEDEDEA);
    let primary_foreground = color(0xFFFFFF, 0x1B1B1A);
    let selected = color(0xECECE8, 0x2C2C2A);
    theme.background = canvas;
    theme.foreground = foreground;
    theme.muted = surface;
    theme.muted_foreground = muted;
    theme.border = border;
    theme.input = color(0x8F8F8B, 0x6F6F6B);
    theme.popover = canvas;
    theme.popover_foreground = foreground;
    theme.danger = color(0xAD3F3C, 0xE7988B);
    theme.primary = primary;
    theme.primary_foreground = primary_foreground;
    theme.primary_hover = color(0x111111, 0xFFFFFF);
    theme.primary_active = color(0x000000, 0xD6D6D2);
    theme.accent = selected;
    theme.accent_foreground = foreground;
    theme.secondary = surface;
    theme.secondary_foreground = foreground;
    theme.secondary_hover = selected;
    theme.secondary_active = selected;
    theme.ring = primary;
    theme.caret = primary;
    theme.selection = selected;
    theme.link = primary;
    theme.link_hover = theme.primary_hover;
    theme.link_active = theme.primary_active;
    theme.sidebar = sidebar;
    theme.sidebar_foreground = foreground;
    theme.sidebar_border = border;
    theme.sidebar_accent = selected;
    theme.sidebar_accent_foreground = foreground;
    theme.sidebar_primary = primary;
    theme.sidebar_primary_foreground = primary_foreground;
    theme.colors.list = canvas;
    theme.list_hover = surface;
    theme.list_active = selected;
    theme.list_active_border = border;
    theme.list_head = surface;
    theme.button = canvas;
    theme.button_foreground = foreground;
    theme.button_hover = surface;
    theme.button_active = selected;
    theme.button_primary = primary;
    theme.button_primary_foreground = primary_foreground;
    theme.button_primary_hover = theme.primary_hover;
    theme.button_primary_active = theme.primary_active;
    theme.button_secondary = surface;
    theme.button_secondary_foreground = foreground;
    theme.button_secondary_hover = selected;
    theme.button_secondary_active = selected;
    theme.title_bar = sidebar;
    theme.title_bar_border = border;
    theme.group_box = surface;
    theme.group_box_foreground = foreground;
    theme.tab = surface;
    theme.tab_foreground = muted;
    theme.tab_active = selected;
    theme.tab_active_foreground = primary;
    theme.tab_bar = sidebar;
    theme.tab_bar_segmented = surface;
    theme.radius = px(8.);
    theme.radius_lg = px(12.);
    theme.tokens = theme.colors.into();
    Theme::sync_base(cx);
}

/// Colour with a job: each view, date kind, and project carries its own hue,
/// while actions and selection stay neutral ink.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Accent {
    Today,
    Inbox,
    Upcoming,
    Calendar,
    Done,
    Scheduled,
    Repeat,
    Project(usize),
}

const PROJECT_LIGHT: [u32; 8] = [
    0x3B6FB6, 0x6B5BB5, 0x2F8A7D, 0xB1843D, 0xB5486A, 0xC2622F, 0x3E7A4F, 0x5A6B7A,
];
const PROJECT_DARK: [u32; 8] = [
    0x8FB4E8, 0xB3A6E8, 0x86CDBF, 0xD9A860, 0xE7A0B9, 0xEBA37A, 0x9ACB9F, 0x9FB0BF,
];

/// Stable colour slot for a project, using the same hash as the web and
/// Android clients so a project looks the same everywhere.
pub fn project_color_index(id: uuid::Uuid) -> usize {
    let hash = id.to_string().bytes().fold(0u32, |hash, byte| {
        hash.wrapping_mul(31).wrapping_add(byte as u32)
    });
    (hash % PROJECT_LIGHT.len() as u32) as usize
}

pub fn accent(accent: Accent, cx: &App) -> gpui_kit::Hsla {
    use gpui_kit::rgb;
    let dark = cx.theme().is_dark();
    let (light, dark_color) = match accent {
        Accent::Today => (0x946A2A, 0xD9A860),
        Accent::Inbox | Accent::Scheduled => (0x3B6FB6, 0x8FB4E8),
        Accent::Upcoming => (0x6B5BB5, 0xB3A6E8),
        Accent::Calendar | Accent::Repeat => (0x25736A, 0x86CDBF),
        Accent::Done => (0x3E7A4F, 0x9ACB9F),
        Accent::Project(index) => (
            PROJECT_LIGHT[index % PROJECT_LIGHT.len()],
            PROJECT_DARK[index % PROJECT_DARK.len()],
        ),
    };
    rgb(if dark { dark_color } else { light }).into()
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
                    Button::new(appearance.label())
                        .label(appearance.label())
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
