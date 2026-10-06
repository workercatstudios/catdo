//! Radio: a ring that squashes when pressed and a dot that pops in.
//!
//! Replaces `gpui_kit::component::radio`, rebuilt on gpui-base with GPUI Component's structure,
//! sizes and spacing. Every other item of that module is re-exported.
//!
//! Pressing squashes the ring to 1.1 × 0.86 (0.1 s); letting go springs it back (0.3 s). The chosen
//! dot pops 0 → 1.25 → 0.95 → 1 in 0.3 s and the old one shrinks away in 0.12 s. The ring's colour
//! fades in 0.15 s. Nothing plays until a radio's value first changes. Under reduced motion the
//! ring doesn't squash and the dot appears and goes at once.
//!
//! Differences: the web radio's look, a primary ring around a dot (57 % of the ring's inside),
//! replaces GPUI Component's filled circle with a tick, because the dot is what pops. The ring and
//! the dot are owned shapes, so the uneven squash is exact; the 1 px border stays 1 px.
//!
//! Accessibility: a plain-text [`label`](Radio::label) is also the control's accessible name. A
//! rich-text label (a `TextView`) isn't: its text can't be read back out of it, so give the control
//! an [`accessibility_label`](Radio::accessibility_label) too.

use std::rc::Rc;
use std::time::Duration;

use gpui_kit::base::{Radio as BaseRadio, RadioGroup as BaseRadioGroup};
use gpui_kit::component::text::Text;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{
    ActiveTheme as _, AxisExt as _, FocusableExt, Sizable, Size, StyledExt as _, ThemeStyled as _,
    h_flex, v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, Axis, ElementId, InteractiveElement, Interactivity, IntoElement, MouseButton,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, StyleRefinement, Styled,
    Window, div, relative, rems,
};

#[allow(unused_imports)]
pub use gpui_kit::component::radio::*;

use crate::button::Press;
use crate::checkbox::BoxMotion;
use crate::glide::glide;
use crate::motion::{Easing, Keyframes, Pulse, Track, ms};
use crate::theme::ActiveKira as _;

/// The dot popping in.
fn dot_in() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at(0.0, 0.0)
        .at(0.5, 1.25)
        .at(0.75, 0.95)
        .at(1.0, 1.0)
        .build()
}

/// The old dot shrinking away, on ease-in.
fn dot_out() -> Keyframes<f32> {
    Track::new(Easing::EaseIn).at(0.0, 1.0).at(1.0, 0.0).build()
}

const DOT_IN: Duration = ms(300);
const DOT_OUT: Duration = ms(120);
const RELEASE: Duration = ms(300);

/// The dot's scale `elapsed` after the radio's last change, or at rest.
pub(crate) fn dot_scale(checked: bool, elapsed: Option<Duration>) -> f32 {
    match elapsed {
        Some(elapsed) if checked && elapsed < DOT_IN => {
            dot_in().sample(elapsed.as_secs_f32() / DOT_IN.as_secs_f32())
        }
        Some(elapsed) if !checked && elapsed < DOT_OUT => {
            dot_out().sample(elapsed.as_secs_f32() / DOT_OUT.as_secs_f32())
        }
        _ if checked => 1.0,
        _ => 0.0,
    }
}

/// The ring's squash for a press amount: 0 is fully pressed (1.1 × 0.86), 1 is at rest. The
/// spring back overshoots past 1, so the ring briefly narrows and stretches.
pub(crate) fn squash(rest: f32) -> (f32, f32) {
    (1.1 + (1.0 - 1.1) * rest, 0.86 + (1.0 - 0.86) * rest)
}

/// A radio button. It doesn't manage a group; use [`RadioGroup`] or set `checked` yourself.
#[derive(IntoElement)]
pub struct Radio {
    base: BaseRadio,
    style: StyleRefinement,
    id: ElementId,
    label: Option<Text>,
    accessibility_label: Option<SharedString>,
    children: Vec<AnyElement>,
    checked: bool,
    disabled: bool,
    tab_stop: bool,
    tab_index: isize,
    size: Size,
    on_click: Option<Rc<dyn Fn(&bool, &mut Window, &mut App) + 'static>>,
    tooltip: Option<SharedString>,
    position_in_set: Option<usize>,
    size_of_set: Option<usize>,
    focus_ring_enabled: bool,
}

impl Radio {
    pub fn new(id: impl Into<ElementId>) -> Self {
        let id = id.into();
        Self {
            base: BaseRadio::new(id.clone()),
            id,
            style: StyleRefinement::default(),
            label: None,
            accessibility_label: None,
            children: Vec::new(),
            checked: false,
            disabled: false,
            tab_index: 0,
            tab_stop: true,
            size: Size::default(),
            on_click: None,
            tooltip: None,
            position_in_set: None,
            size_of_set: None,
            focus_ring_enabled: true,
        }
    }

    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// The text beside the control. A plain string is also its accessible name; a rich-text label
    /// isn't (its text can't be read back), so pair one with
    /// [`accessibility_label`](Self::accessibility_label).
    pub fn label(mut self, label: impl Into<Text>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// The name a screen reader announces, when the visible label is not it, or is rich text.
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn tab_index(mut self, tab_index: isize) -> Self {
        self.tab_index = tab_index;
        self
    }

    pub fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.tab_stop = tab_stop;
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
}

impl Sizable for Radio {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl FocusableExt for Radio {
    fn focus_ring(mut self, enabled: bool) -> Self {
        self.focus_ring_enabled = enabled;
        self
    }

    fn is_focus_ring_enabled(&self) -> bool {
        self.focus_ring_enabled
    }
}

impl Styled for Radio {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl InteractiveElement for Radio {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for Radio {}

impl ParentElement for Radio {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

struct Seen(bool);

impl RenderOnce for Radio {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let checked = self.checked;
        let disabled = self.disabled;
        let id = self.id.clone();
        let has_content = self.label.is_some() || !self.children.is_empty();
        let indicator_size = rems(match self.size {
            Size::XSmall => 0.75,
            Size::Small => 0.875,
            Size::Large => 1.125,
            _ => 1.,
        });

        let seen = window.use_keyed_state((id.clone(), "kk-seen"), cx, |_, _| Seen(checked));
        if seen.read(cx).0 != checked {
            seen.update(cx, |seen, _| seen.0 = checked);
            Pulse::new((id.clone(), "kk-dot"), window, cx).fire(cx);
        }
        let change = Pulse::new((id.clone(), "kk-dot"), window, cx);
        change.animate(DOT_IN, window);
        let dot = dot_scale(checked, change.running(DOT_IN));

        let motion = BoxMotion::new(&id, window, cx);
        let held = !disabled && motion.pressed.read(cx).0;
        let rest = Press {
            sink: 0.0,
            pop: Track::new(cx.curves().spring)
                .at(0.0, 0.0)
                .at(1.0, 1.0)
                .build(),
            duration: RELEASE,
        }
        .scale(&id, held, &motion.pop, window, cx);
        let (sx, sy) = squash(rest);
        let ring = glide(
            (id.clone(), "kk-ring"),
            if checked { 1.0 } else { 0.0 },
            ms(150),
            Easing::EaseOut,
            window,
            cx,
        );

        let focus_handle = window
            .use_keyed_state(id.clone(), cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone();
        let is_focused = focus_handle.is_focused(window);
        let accessibility_label = self.accessibility_label.clone().or_else(|| {
            self.label.as_ref().and_then(|label| match label {
                Text::String(text) => Some(text.clone()),
                Text::TextView(_) => None,
            })
        });

        let theme = cx.theme();
        let fade = |color: gpui_kit::Hsla| if disabled { color.opacity(0.5) } else { color };
        let border = fade(theme.input);
        let checked_border = fade(theme.primary);
        let dot_color = fade(theme.primary);
        let ring_bg = theme.input_background();
        let foreground = theme.foreground;
        let muted_foreground = theme.muted_foreground;
        let radius = theme.radius * 0.5;

        let side = indicator_size.to_pixels(window.rem_size());
        let (w, h) = (side * sx, side * sy);
        // The dot is 57 % of the ring's inside, and squashes with it.
        let inner = side - gpui_kit::px(2.);
        let (dot_w, dot_h) = (inner * 0.57 * sx * dot, inner * 0.57 * sy * dot);
        let tooltip = self.tooltip.clone();

        let root = self
            .base
            .id(id.clone())
            .checked(checked)
            .disabled(disabled)
            .track_focus(&focus_handle)
            .tab_stop(self.tab_stop)
            .tab_index(self.tab_index)
            .when_some(accessibility_label, |this, label| {
                this.accessibility_label(label)
            })
            .when_some(
                self.position_in_set.zip(self.size_of_set),
                |this, (position, size)| this.set_position(position, size),
            )
            .h_flex()
            .gap_x_2()
            .text_color(foreground)
            .items_start()
            .line_height(relative(1.))
            .rounded(radius)
            .when(is_focused && self.focus_ring_enabled, |this| {
                this.focus_ring_style(window, cx)
            })
            .map(|this| match self.size {
                Size::XSmall => this.text_xs(),
                Size::Small => this.text_sm(),
                Size::Medium => this.text_base(),
                Size::Large => this.text_lg(),
                _ => this,
            })
            .refine_style(&self.style)
            .child(
                div()
                    .relative()
                    .size(indicator_size)
                    // Center on the first 1.25em line, including when the label wraps.
                    .when(has_content, |this| this.mt(indicator_size * 0.125))
                    .flex_shrink_0()
                    .child(
                        div()
                            .absolute()
                            .left((side - w) * 0.5)
                            .top((side - h) * 0.5)
                            .w(w)
                            .h(h)
                            .rounded_full()
                            .border_1()
                            .border_color(border)
                            .bg(ring_bg)
                            .child(
                                div()
                                    .absolute()
                                    .top(gpui_kit::px(-1.))
                                    .left(gpui_kit::px(-1.))
                                    .w(w)
                                    .h(h)
                                    .rounded_full()
                                    .border_1()
                                    .border_color(checked_border)
                                    .opacity(ring),
                            )
                            .when(dot > 0.0, |this| {
                                this.child(
                                    div()
                                        .absolute()
                                        .left((w - gpui_kit::px(2.) - dot_w) * 0.5)
                                        .top((h - gpui_kit::px(2.) - dot_h) * 0.5)
                                        .w(dot_w)
                                        .h(dot_h)
                                        .rounded_full()
                                        .bg(dot_color),
                                )
                            }),
                    ),
            )
            .when(has_content, |this| {
                this.child(
                    v_flex()
                        .w_full()
                        .line_height(relative(1.25))
                        .gap_1()
                        .when_some(self.label, |this, label| {
                            this.child(
                                div()
                                    .size_full()
                                    .when(disabled, |this| this.text_color(muted_foreground))
                                    .child(label),
                            )
                        })
                        .children(self.children),
                )
            })
            .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
            .when_some(self.on_click.clone(), |this, on_click| {
                this.on_change(move |next, _, window, cx| {
                    window.prevent_default();
                    on_click(&next, window, cx);
                })
            })
            .when_some(tooltip, |this, tooltip| {
                this.tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
            });
        if disabled { root } else { motion.listen(root) }
    }
}

/// A group of radios, laid out vertically by default. Controlled: `on_click` reports the
/// requested index.
#[derive(IntoElement)]
pub struct RadioGroup {
    id: ElementId,
    style: StyleRefinement,
    radios: Vec<Radio>,
    layout: Axis,
    selected_index: Option<usize>,
    disabled: bool,
    on_click: Option<Rc<dyn Fn(&usize, &mut Window, &mut App) + 'static>>,
}

impl RadioGroup {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default().flex_1(),
            on_click: None,
            layout: Axis::Vertical,
            selected_index: None,
            disabled: false,
            radios: vec![],
        }
    }

    pub fn vertical(id: impl Into<ElementId>) -> Self {
        Self::new(id)
    }

    pub fn horizontal(id: impl Into<ElementId>) -> Self {
        Self::new(id).layout(Axis::Horizontal)
    }

    pub fn layout(mut self, layout: Axis) -> Self {
        self.layout = layout;
        self
    }

    /// Alias for [`Self::on_change`].
    pub fn on_click(self, handler: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_change(handler)
    }

    /// Handle a requested selected index. The owner writes it back and re-renders.
    pub fn on_change(mut self, handler: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    pub fn selected_index(mut self, index: Option<usize>) -> Self {
        self.selected_index = index;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn child(mut self, child: impl Into<Radio>) -> Self {
        self.radios.push(child.into());
        self
    }

    pub fn children(mut self, children: impl IntoIterator<Item = impl Into<Radio>>) -> Self {
        self.radios.extend(children.into_iter().map(Into::into));
        self
    }
}

impl Styled for RadioGroup {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl From<&'static str> for Radio {
    fn from(label: &'static str) -> Self {
        Self::new(label).label(label)
    }
}

impl From<SharedString> for Radio {
    fn from(label: SharedString) -> Self {
        Self::new(label.clone()).label(label)
    }
}

impl From<String> for Radio {
    fn from(label: String) -> Self {
        Self::new(SharedString::from(label.clone())).label(SharedString::from(label))
    }
}

impl RenderOnce for RadioGroup {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let on_click = self.on_click;
        let disabled = self.disabled;
        let selected_ix = self.selected_index;

        let base = if self.layout.is_vertical() {
            v_flex()
        } else {
            h_flex().w_full().flex_wrap()
        };

        let total = self.radios.len();
        BaseRadioGroup::new(self.id)
            .axis(self.layout)
            .refine_style(&self.style)
            .child(
                base.gap_3()
                    .children(self.radios.into_iter().enumerate().map(|(ix, mut radio)| {
                        let checked = selected_ix == Some(ix);
                        radio.id = ix.into();
                        radio.position_in_set = Some(ix + 1);
                        radio.size_of_set = Some(total);
                        radio.disabled(disabled).checked(checked).when_some(
                            on_click.clone(),
                            |this, on_click| {
                                this.on_click(move |_, window, cx| on_click(&ix, window, cx))
                            },
                        )
                    })),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_dot_pops_past_full_size() {
        assert_eq!(dot_scale(true, Some(ms(0))), 0.0);
        assert_eq!(dot_scale(true, Some(ms(150))), 1.25);
        assert_eq!(dot_scale(true, None), 1.0);
        assert_eq!(dot_scale(false, None), 0.0);
        let leaving = dot_scale(false, Some(ms(60)));
        assert!(leaving > 0.0 && leaving < 1.0);
    }

    #[test]
    fn tracks_match_the_web() {
        use crate::parity::assert_number_track;
        assert_number_track("pop-radio-group", "kk-pop-radio-group-in", "sx", &dot_in());
        assert_number_track(
            "pop-radio-group",
            "kk-pop-radio-group-out",
            "sx",
            &dot_out(),
        );
    }

    #[test]
    fn a_press_squashes_wide_and_short() {
        assert_eq!(squash(1.0), (1.0, 1.0));
        let (sx, sy) = squash(0.0);
        assert!((sx - 1.1).abs() < 1e-6 && (sy - 0.86).abs() < 1e-6);
    }
}
