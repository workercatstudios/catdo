//! Switch: a thumb that ducks while it travels and pops as it lands.
//!
//! Replaces `gpui_kit::component::switch`. The thumb slides on a strong ease-out (0.25 s) and pops
//! separately: pressing shrinks it to 0.85; a toggle ducks it to 0.78 while it travels (20 %) and
//! pops it to 1.12 as it lands (55 %), 0.97, then 1, over 0.4 s. Nothing plays on first render.
//! Under reduced motion the thumb jumps across and keeps its size; the colours still change.
//!
//! As in GPUI Component, the track and thumb are pills when the theme's radius is 4 px or more and
//! take the theme's radius below that.
//!
//! A plain-string label is also the switch's accessible name. A rich-text label
//! ([`TextView`](gpui_kit::component::text::TextView)) isn't: GPUI Component keeps a text view's
//! source private, so name such a switch with [`Switch::accessibility_label`].

use std::rc::Rc;

use gpui_kit::base::StyledExt as _;
use gpui_kit::base::{Switch as BaseSwitch, SwitchThumb, SwitchTrack};
use gpui_kit::component::text::Text;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{ActiveTheme as _, Disableable, Side, Sizable, Size};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    App, Background, ElementId, Hsla, InteractiveElement as _, IntoElement, MouseButton,
    ParentElement as _, RenderOnce, SharedString, StatefulInteractiveElement as _, StyleRefinement,
    Styled, Window, div, px,
};

use crate::button::{Press, Pressed};
use crate::motion::{Easing, Keyframes, Pulse, Track, ms};
use crate::state_motion::glide;
use crate::theme::ActiveKira as _;

/// The landing pop, from the pressed 0.85.
fn landing() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at(0.0, 0.85)
        .at(0.2, 0.78)
        .at(0.55, 1.12)
        .at(0.8, 0.97)
        .at(1.0, 1.0)
        .build()
}

/// A switch, toggled on or off. Controlled: `on_click` reports the next value.
#[derive(IntoElement)]
pub struct Switch {
    id: ElementId,
    style: StyleRefinement,
    checked: bool,
    disabled: bool,
    label: Option<Text>,
    accessibility_label: Option<SharedString>,
    label_side: Side,
    on_click: Option<Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
    size: Size,
    color: Option<Hsla>,
    tooltip: Option<SharedString>,
}

impl Switch {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            checked: false,
            disabled: false,
            label: None,
            accessibility_label: None,
            label_side: Side::Right,
            on_click: None,
            size: Size::Medium,
            color: None,
            tooltip: None,
        }
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// The visible label. A plain string also names the switch for screen readers; give a
    /// rich-text label an [`accessibility_label`](Self::accessibility_label) too.
    pub fn label(mut self, label: impl Into<Text>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// The name a screen reader announces, when the visible label is not it.
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    /// Which side of the switch the label sits on. Right by default.
    pub fn label_side(mut self, side: Side) -> Self {
        self.label_side = side;
        self
    }

    /// Alias for [`Self::on_change`].
    pub fn on_click(self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_change(handler)
    }

    /// Handle a requested checked value. The owner writes it back and re-renders.
    pub fn on_change(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// The track colour when checked. Defaults to the theme's primary.
    pub fn color(mut self, color: impl Into<Hsla>) -> Self {
        self.color = Some(color.into());
        self
    }

    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }
}

impl Styled for Switch {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Sizable for Switch {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Disableable for Switch {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

struct Seen(bool);

impl RenderOnce for Switch {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let checked = self.checked;
        let id = self.id.clone();
        let pulse = Pulse::new((id.clone(), "kk-pop"), window, cx);
        let pressed = window.use_keyed_state((id.clone(), "kk-pressed"), cx, |_, _| Pressed(false));
        // Armed by a new value, so switches that render on don't all pop on load.
        let seen = window.use_keyed_state((id.clone(), "kk-seen"), cx, |_, _| Seen(checked));
        if seen.read(cx).0 != checked {
            seen.update(cx, |seen, _| seen.0 = checked);
            pulse.fire(cx);
        }
        let pulse = Pulse::new((id.clone(), "kk-pop"), window, cx);
        let held = !self.disabled && pressed.read(cx).0;
        let thumb_scale = Press {
            sink: 0.85,
            pop: landing(),
            duration: ms(400),
        }
        .scale(&id, held, &pulse, window, cx);

        let (track_w, track_h) = match self.size {
            Size::XSmall | Size::Small => (px(28.), px(16.)),
            _ => (px(36.), px(20.)),
        };
        let thumb = match self.size {
            Size::XSmall | Size::Small => px(12.),
            _ => px(16.),
        };
        let inset = px(2.);
        let travel = track_w - thumb - inset * 2.;
        let thumb_x = glide(
            (id.clone(), "kk-thumb"),
            if checked { travel } else { px(0.) },
            ms(250),
            cx.curves().out,
            window,
            cx,
        );
        let theme = cx.theme();
        let radius = if theme.radius >= px(4.) {
            track_h
        } else {
            theme.radius
        };
        let checked_bg = self
            .color
            .map(Background::from)
            .unwrap_or(theme.tokens.primary.into());
        let unchecked_bg: Background = theme.tokens.switch.into();
        let disabled_bg = if checked { checked_bg } else { unchecked_bg }.opacity(0.5);
        // Kirakira's dark theme draws an unchecked thumb in the ink colour, so it shows on the
        // dark track; elsewhere the thumb is the theme's.
        let thumb_bg: Background = if theme.is_dark() && !checked {
            theme.foreground.into()
        } else {
            theme.tokens.switch_thumb.into()
        };
        let disabled_label = theme.muted_foreground;

        // The thumb is an owned shape, so it scales exactly: resize it around its centre.
        let scaled = thumb * thumb_scale;
        let shift = (thumb - scaled) * 0.5;

        let accessibility_label = self.accessibility_label.clone().or_else(|| {
            // GPUI Component reads a rich label's text with `Text::get_text`, which is
            // crate-private and comes back empty for `markdown(..)` and `html(..)` views anyway;
            // their source isn't reachable from here, so those need `accessibility_label`.
            self.label.as_ref().and_then(|label| match label {
                Text::String(text) => Some(text.clone()),
                Text::TextView(_) => None,
            })
        });
        let on_click = self.on_click.clone();
        let down = pressed.clone();
        let up = pressed.clone();
        let up_out = pressed;
        let tooltip = self.tooltip.clone();

        div().refine_style(&self.style).child(
            BaseSwitch::new(id.clone())
                .checked(checked)
                .disabled(self.disabled)
                .styles(|styles| {
                    styles.disabled(|style| style.text_color(disabled_label).cursor_not_allowed())
                })
                .when_some(accessibility_label, |this, label| {
                    this.accessibility_label(label)
                })
                .when_some(on_click, |this, on_click| {
                    this.on_change(move |next, _, window, cx| on_click(&next, window, cx))
                })
                .flex()
                .gap_2()
                .items_start()
                .when(self.label_side.is_left(), |this| this.flex_row_reverse())
                .when(!self.disabled, |this| {
                    this.on_mouse_down(MouseButton::Left, move |_, _, cx| {
                        down.update(cx, |pressed, cx| {
                            pressed.0 = true;
                            cx.notify();
                        })
                    })
                    .on_mouse_up(MouseButton::Left, move |_, _, cx| {
                        up.update(cx, |pressed, cx| {
                            pressed.0 = false;
                            cx.notify();
                        })
                    })
                    .on_mouse_up_out(MouseButton::Left, move |_, _, cx| {
                        up_out.update(cx, |pressed, cx| {
                            pressed.0 = false;
                            cx.notify();
                        })
                    })
                })
                .child(
                    SwitchTrack::new((id.clone(), "track"))
                        .checked(checked)
                        .disabled(self.disabled)
                        .relative()
                        .flex_none()
                        .w(track_w)
                        .h(track_h)
                        .rounded(radius)
                        .border(inset)
                        .border_color(theme.transparent)
                        .when(!checked, |this| this.bg(unchecked_bg))
                        .styles(|styles| {
                            styles
                                .checked(|style| style.bg(checked_bg))
                                .disabled(|style| style.bg(disabled_bg))
                        })
                        .when_some(tooltip, |this, tooltip| {
                            this.tooltip(move |window, cx| {
                                Tooltip::new(tooltip.clone()).build(window, cx)
                            })
                        })
                        .child(
                            SwitchThumb::new(checked)
                                .absolute()
                                .rounded(radius)
                                .size(scaled)
                                .left(thumb_x + shift)
                                .top(shift)
                                .bg(thumb_bg),
                        ),
                )
                .when_some(self.label, |this, label| {
                    this.child(div().line_height(track_h).child(label).map(
                        |this| match self.size {
                            Size::XSmall | Size::Small => this.text_sm(),
                            _ => this.text_base(),
                        },
                    ))
                }),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::parity::assert_number_track;

    #[test]
    fn landing_matches_the_web() {
        assert_number_track("pop-switch", "kk-pop-switch-on", "sx", &super::landing());
        assert_number_track("pop-switch", "kk-pop-switch-off", "sx", &super::landing());
    }
}
