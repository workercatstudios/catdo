//! Label: a text label that nods when pressed.
//!
//! Replaces `gpui_kit::component::label`, wrapping GPUI Component's `Label`: secondary text,
//! masking and highlights work as before, and every other item of that module is re-exported.
//!
//! Pressing the label dips it 0.08 em in 0.08 s (ease-out); letting go springs it back on the spring
//! curve in 0.3 s, rising a hair past its line before it settles. Like the web version it is a
//! transition, so a quick tap still shows the whole nod. A [`Label::disabled`] label doesn't nod.
//! Under reduced motion nothing moves.
//!
//! Differences: a GPUI label isn't tied to a control, so it nods on its own press rather than
//! clicking a control for it. GPUI Component's `Label` has no id; this one keys its motion by its
//! text unless [`Label::id`] gives it one, so two labels with the same text in the same parent
//! should get ids.

use std::time::Duration;

use gpui_kit::component::label::Label as BaseLabel;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    App, ElementId, InteractiveElement as _, IntoElement, MouseButton, ParentElement as _,
    RenderOnce, SharedString, StyleRefinement, Styled, Window, div, px,
};

#[allow(unused_imports)]
pub use gpui_kit::component::label::*;

use crate::button::Pressed;
use crate::glide::glide;
use crate::motion::{Easing, ms, split_layout};
use crate::theme::ActiveKira as _;
use crate::vector::TextSetting;

/// How far the label dips, in em.
const DIP: f32 = 0.08;
const DOWN: Duration = ms(80);
const UP: Duration = ms(300);

/// A mouse listener that records whether the label is held.
fn set<E>(state: gpui_kit::Entity<Pressed>, value: bool) -> impl Fn(&E, &mut Window, &mut App) {
    move |_, _, cx| {
        state.update(cx, |pressed, cx| {
            pressed.0 = value;
            cx.notify();
        })
    }
}

/// A text label with optional secondary text, masking and highlights.
#[derive(IntoElement)]
pub struct Label {
    id: Option<ElementId>,
    text: SharedString,
    inner: BaseLabel,
    disabled: bool,
}

impl Label {
    pub fn new(label: impl Into<SharedString>) -> Self {
        let text: SharedString = label.into();
        Self {
            id: None,
            inner: BaseLabel::new(text.clone()),
            text,
            disabled: false,
        }
    }

    fn map(mut self, f: impl FnOnce(BaseLabel) -> BaseLabel) -> Self {
        self.inner = f(self.inner);
        self
    }

    /// Secondary text, shown after the label in the muted colour.
    pub fn secondary(self, secondary: impl Into<SharedString>) -> Self {
        self.map(|inner| inner.secondary(secondary))
    }

    pub fn masked(self, masked: bool) -> Self {
        self.map(|inner| inner.masked(masked))
    }

    pub fn highlights(self, text: impl Into<HighlightsMatch>) -> Self {
        self.map(|inner| inner.highlights(text))
    }

    /// The id the nod is keyed by. Defaults to the label's text.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// A label for a disabled control: dimmed, and it doesn't nod. Not in GPUI Component.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Styled for Label {
    fn style(&mut self) -> &mut StyleRefinement {
        self.inner.style()
    }
}

impl RenderOnce for Label {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self
            .id
            .unwrap_or_else(|| ElementId::Name(format!("kk-label-{}", self.text).into()));
        let pressed = window.use_keyed_state((id.clone(), "kk-pressed"), cx, |_, _| Pressed(false));
        let held = !self.disabled && pressed.read(cx).0;
        let font_size = TextSetting::new(&self.inner.style().text, window).face.size;
        let dip = glide(
            (id.clone(), "kk-dip"),
            if held { DIP } else { 0.0 },
            if held { DOWN } else { UP },
            if held {
                Easing::EaseOut
            } else {
                cx.curves().spring
            },
            window,
            cx,
        );
        let outer = split_layout(self.inner.style());
        let (down, up, up_out) = (pressed.clone(), pressed.clone(), pressed);

        div()
            .id(id)
            .map(|mut this| {
                *this.style() = outer;
                this
            })
            .when(self.disabled, |this| this.opacity(0.5).cursor_not_allowed())
            .when(!self.disabled, |this| {
                this.on_mouse_down(MouseButton::Left, set(down, true))
                    .on_mouse_up(MouseButton::Left, set(up, false))
                    .on_mouse_up_out(MouseButton::Left, set(up_out, false))
            })
            // A relative offset moves the label like `translate` without moving its neighbours.
            // It goes on a box inside, so the outer box keeps the position and insets it was
            // given. The box fills the outer one, so a label sized to it still is.
            .child(
                div()
                    .relative()
                    .h_full()
                    .when(dip != 0.0, |this| this.top(px(dip * f32::from(font_size))))
                    .child(self.inner),
            )
    }
}
