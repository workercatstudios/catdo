use gpui_kit::component::{
    ActiveTheme, Selectable, Sizable, StyledExt, Theme, ThemeMode,
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
    }

    fn label(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Light => "Light",
            Self::Dark => "Dark",
        }
    }
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
        div()
            .v_flex()
            .gap_2()
            .mt_3()
            .pt_3()
            .border_t_1()
            .border_color(cx.theme().border)
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child("Appearance"),
            )
            .child(
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
                        if let Some(appearance) =
                            selected.first().and_then(|i| Appearance::ALL.get(*i))
                        {
                            this.set_appearance(*appearance, window, cx);
                        }
                    })),
            )
    }
}
