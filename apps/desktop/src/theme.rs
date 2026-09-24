use gpui_kit::component::{
    Selectable, Sizable, StyledExt, Theme, ThemeMode,
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
    let canvas = color(0xFFFFFF, 0x1C201D);
    let sidebar = color(0xF7F7F5, 0x181B19);
    let surface = color(0xF7F7F5, 0x242925);
    let foreground = color(0x242824, 0xEDF0EA);
    let muted = color(0x696F66, 0xA2AAA0);
    let border = color(0xE8EAE5, 0x333B34);
    let primary = color(0x3E624E, 0xACCCB1);
    let primary_foreground = color(0xFFFFFF, 0x1C201D);
    let selected = color(0xE9EFE9, 0x2C3E30);
    theme.background = canvas;
    theme.foreground = foreground;
    theme.muted = surface;
    theme.muted_foreground = muted;
    theme.border = border;
    theme.input = color(0x8C978C, 0x718171);
    theme.popover = canvas;
    theme.popover_foreground = foreground;
    theme.danger = color(0xAD3F3C, 0xE7988B);
    theme.primary = primary;
    theme.primary_foreground = primary_foreground;
    theme.primary_hover = color(0x3D5949, 0xBBD5C3);
    theme.primary_active = color(0x344C3E, 0x96B8A1);
    theme.accent = selected;
    theme.accent_foreground = primary;
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
    theme.sidebar_accent_foreground = primary;
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
                .small()
                .compact()
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
