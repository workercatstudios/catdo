//! Checkbox: a box that squashes when pressed, rebounds when checked and draws its tick.
//!
//! Replaces `gpui_kit::component::checkbox`, rebuilt on gpui-base with GPUI Component's structure,
//! sizes and colours. Every other item of that module is re-exported.
//!
//! Pressing squashes the box to 0.85 (0.1 s). Checking rebounds it from there, 0.85 → 1.08 → 0.97
//! → 1 in 0.34 s, and 0.08 s later the tick draws itself in over 0.25 s. Unchecking fades the mark
//! out in 0.12 s while the fill fades back to empty (0.15 s) and the box springs back to full size
//! in 0.3 s. [`Checkbox::indeterminate`] does the same with a dash. Nothing plays on first render.
//! Under reduced motion nothing squashes or draws; the fill and the mark change at once.
//!
//! Differences from the web version: GPUI can't dash an SVG stroke per frame, so the mark is a
//! stroked path in a `canvas`, cut at the drawn length, with round caps and joins painted as
//! discs. The box is an owned shape, so it scales exactly, but its 1 px border stays 1 px. The box
//! keeps GPUI Component's look (theme radius capped at 4 px, 16 px at the default size) rather than
//! the web's 30 % rounding.
//!
//! Accessibility: a plain-text [`label`](Checkbox::label) is also the control's accessible name. A
//! rich-text label (a `TextView`) isn't: its text can't be read back out of it, so give the control
//! an [`accessibility_label`](Checkbox::accessibility_label) too.

use std::rc::Rc;
use std::time::Duration;

use gpui_kit::base::{Checkbox as BaseCheckbox, CheckboxIndicator, CheckboxState};
use gpui_kit::component::text::Text;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{
    ActiveTheme as _, Disableable, FocusableExt, RoleOverride, Selectable, Sizable, Size,
    StyledExt as _, ThemeStyled as _, v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, Background, Bounds, ElementId, Hsla, InteractiveElement, Interactivity,
    IntoElement, MouseButton, ParentElement, PathBuilder, Pixels, RenderOnce, SharedString,
    StatefulInteractiveElement, StyleRefinement, Styled, Window, canvas, div, point, px, relative,
    rems,
};

#[allow(unused_imports)]
pub use gpui_kit::component::checkbox::*;

use crate::button::{Press, Pressed};
use crate::glide::glide;
use crate::motion::{Easing, Keyframes, Pulse, Track, ms};
use crate::theme::ActiveKira as _;

/// The rebound after checking, from the pressed 0.85.
fn check_track() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at(0.0, 0.85)
        .at(0.4, 1.08)
        .at(0.72, 0.97)
        .at(1.0, 1.0)
        .build()
}

/// Unchecking only springs back: the web's `scale 0.3s spring` transition from 0.85.
fn release_track(spring: Easing) -> Keyframes<f32> {
    Track::new(spring).at(0.0, 0.85).at(1.0, 1.0).build()
}

const CHECK: Duration = ms(340);
const RELEASE: Duration = ms(300);
const MARK_DELAY: Duration = ms(80);
const MARK_DRAW: Duration = ms(250);
const MARK_OUT: Duration = ms(120);
const FILL: Duration = ms(150);

/// The mark a checkbox draws, in the web component's 24-unit box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Mark {
    Check,
    Dash,
}

impl Mark {
    fn of(state: CheckboxState) -> Self {
        match state {
            CheckboxState::Indeterminate => Self::Dash,
            _ => Self::Check,
        }
    }

    /// `M5 12.5l4.5 4.5L19 7` and `M6.5 12h11`.
    pub(crate) fn points(self) -> &'static [(f32, f32)] {
        match self {
            Self::Check => &[(5.0, 12.5), (9.5, 17.0), (19.0, 7.0)],
            Self::Dash => &[(6.5, 12.0), (17.5, 12.0)],
        }
    }
}

/// The first `progress` (0..=1) of a polyline by length: what `stroke-dashoffset` reveals.
pub(crate) fn partial_polyline(points: &[(f32, f32)], progress: f32) -> Vec<(f32, f32)> {
    let segment = |a: (f32, f32), b: (f32, f32)| ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
    let total: f32 = points.windows(2).map(|w| segment(w[0], w[1])).sum();
    let mut left = total * progress.clamp(0.0, 1.0);
    let mut out = Vec::new();
    let Some(&first) = points.first() else {
        return out;
    };
    if left <= 0.0 {
        return out;
    }
    out.push(first);
    for pair in points.windows(2) {
        let length = segment(pair[0], pair[1]);
        if left >= length {
            out.push(pair[1]);
            left -= length;
        } else {
            let t = left / length;
            out.push((
                pair[0].0 + (pair[1].0 - pair[0].0) * t,
                pair[0].1 + (pair[1].1 - pair[0].1) * t,
            ));
            break;
        }
    }
    out
}

/// `kk-pop-checkbox-tick`'s opacity: hidden at 0 %, shown from 1 %. (Its `stroke-dashoffset`
/// 1 → 0 is the drawn length, eased ease-in-out in [`mark_frame`].)
fn tick_opacity() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at(0.0, 0.0)
        .at(0.01, 1.0)
        .at(1.0, 1.0)
        .build()
}

/// `kk-pop-checkbox-out`: the mark fades on ease-in.
fn mark_out() -> Keyframes<f32> {
    Track::new(Easing::EaseIn).at(0.0, 1.0).at(1.0, 0.0).build()
}

/// How far the mark is drawn and how opaque it is, `elapsed` after the last change.
pub(crate) fn mark_frame(on: bool, elapsed: Option<Duration>) -> (f32, f32) {
    match elapsed {
        None => (1.0, if on { 1.0 } else { 0.0 }),
        Some(elapsed) if on => {
            // `both` fill: the first keyframe holds through the delay.
            let t = elapsed.saturating_sub(MARK_DELAY).as_secs_f32() / MARK_DRAW.as_secs_f32();
            let t = t.min(1.0);
            (Easing::EaseInOut.sample(t), tick_opacity().sample(t))
        }
        Some(elapsed) => {
            let t = elapsed.as_secs_f32() / MARK_OUT.as_secs_f32();
            (1.0, mark_out().sample(t.min(1.0)))
        }
    }
}

/// Paints a polyline of `points` (in a 24-unit box mapped onto `bounds`) with round caps and
/// joins, cut at `progress` of its length.
pub(crate) fn paint_mark(
    bounds: Bounds<Pixels>,
    points: &[(f32, f32)],
    progress: f32,
    stroke: f32,
    color: Hsla,
    window: &mut Window,
) {
    let unit = bounds.size.width.as_f32() / 24.0;
    let width = stroke * unit;
    let at = |(x, y): (f32, f32)| {
        point(
            bounds.origin.x + px(x * unit),
            bounds.origin.y + px(y * unit),
        )
    };
    let drawn = partial_polyline(points, progress);
    for pair in drawn.windows(2) {
        let mut path = PathBuilder::stroke(px(width));
        path.move_to(at(pair[0]));
        path.line_to(at(pair[1]));
        if let Ok(path) = path.build() {
            window.paint_path(path, color);
        }
    }
    // Round caps and joins: a disc at every vertex drawn so far.
    for &vertex in &drawn {
        let centre = at(vertex);
        let radius = width / 2.0;
        let mut disc = PathBuilder::fill();
        let steps = 16;
        let ring: Vec<_> = (0..steps)
            .map(|i| {
                let angle = i as f32 / steps as f32 * std::f32::consts::TAU;
                point(
                    centre.x + px(radius * angle.cos()),
                    centre.y + px(radius * angle.sin()),
                )
            })
            .collect();
        disc.add_polygon(&ring, true);
        if let Ok(path) = disc.build() {
            window.paint_path(path, color);
        }
    }
}

/// A checkbox. Controlled: `on_click` reports the requested value.
///
/// The same builder as GPUI Component's `Checkbox`, plus [`Checkbox::indeterminate`].
#[derive(IntoElement)]
pub struct Checkbox {
    id: ElementId,
    base: BaseCheckbox,
    style: StyleRefinement,
    label: Option<Text>,
    accessibility_label: Option<SharedString>,
    children: Vec<AnyElement>,
    checked: bool,
    indeterminate: bool,
    disabled: bool,
    size: Size,
    tab_stop: bool,
    tab_index: isize,
    on_click: Option<Rc<dyn Fn(&bool, &mut Window, &mut App) + 'static>>,
    tooltip: Option<SharedString>,
    role: RoleOverride,
    focus_ring_enabled: bool,
}

impl Checkbox {
    pub fn new(id: impl Into<ElementId>) -> Self {
        let id = id.into();
        Self {
            base: BaseCheckbox::new(id.clone()),
            id,
            style: StyleRefinement::default(),
            label: None,
            accessibility_label: None,
            children: Vec::new(),
            checked: false,
            indeterminate: false,
            disabled: false,
            size: Size::default(),
            tab_stop: true,
            tab_index: 0,
            on_click: None,
            tooltip: None,
            role: RoleOverride::default(),
            focus_ring_enabled: true,
        }
    }

    pub fn role(mut self, role: impl Into<RoleOverride>) -> Self {
        self.role = role.into();
        self
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

    /// Show the mixed state: a dash. Takes precedence over `checked`; activating it requests
    /// `true`. Not in GPUI Component, which has no mixed checkbox.
    pub fn indeterminate(mut self, indeterminate: bool) -> Self {
        self.indeterminate = indeterminate;
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

    pub fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.tab_stop = tab_stop;
        self
    }

    pub fn tab_index(mut self, tab_index: isize) -> Self {
        self.tab_index = tab_index;
        self
    }

    fn state(&self) -> CheckboxState {
        if self.indeterminate {
            CheckboxState::Indeterminate
        } else if self.checked {
            CheckboxState::Checked
        } else {
            CheckboxState::Unchecked
        }
    }
}

impl InteractiveElement for Checkbox {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for Checkbox {}

impl Styled for Checkbox {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Disableable for Checkbox {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl FocusableExt for Checkbox {
    fn focus_ring(mut self, enabled: bool) -> Self {
        self.focus_ring_enabled = enabled;
        self
    }

    fn is_focus_ring_enabled(&self) -> bool {
        self.focus_ring_enabled
    }
}

impl Selectable for Checkbox {
    fn selected(self, selected: bool) -> Self {
        self.checked(selected)
    }

    fn is_selected(&self) -> bool {
        self.checked
    }
}

impl ParentElement for Checkbox {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Sizable for Checkbox {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

struct Seen {
    state: CheckboxState,
    /// The last mark shown, kept while unchecking so the right one fades out.
    mark: Mark,
}

/// The pieces of a pressable box shared by the checkbox and the radio: the press state, and the
/// scale and fill for this frame.
pub(crate) struct BoxMotion {
    pub pressed: gpui_kit::Entity<Pressed>,
    pub pop: Pulse,
}

impl BoxMotion {
    pub fn new(id: &ElementId, window: &mut Window, cx: &mut App) -> Self {
        Self {
            pressed: window.use_keyed_state((id.clone(), "kk-pressed"), cx, |_, _| Pressed(false)),
            pop: Pulse::new((id.clone(), "kk-pop"), window, cx),
        }
    }

    /// Adds the press listeners to an interactive root. A release fires the pop, so a click the
    /// owner ignores still springs back.
    pub fn listen<E: InteractiveElement>(&self, element: E) -> E {
        let down = self.pressed.clone();
        let up = self.pressed.clone();
        let up_out = self.pressed.clone();
        let pop = self.pop.clone();
        element
            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                down.update(cx, |pressed, cx| {
                    pressed.0 = true;
                    cx.notify();
                })
            })
            .on_mouse_up(MouseButton::Left, move |_, _, cx| {
                let was = up.read(cx).0;
                up.update(cx, |pressed, cx| {
                    pressed.0 = false;
                    cx.notify();
                });
                if was {
                    pop.fire(cx);
                }
            })
            .on_mouse_up_out(MouseButton::Left, move |_, _, cx| {
                up_out.update(cx, |pressed, cx| {
                    pressed.0 = false;
                    cx.notify();
                })
            })
    }
}

impl RenderOnce for Checkbox {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state();
        let on = state != CheckboxState::Unchecked;
        let id = self.id.clone();
        let has_content = self.label.is_some() || !self.children.is_empty();
        let indicator_size = rems(match self.size {
            Size::XSmall => 0.75,
            Size::Small => 0.875,
            Size::Large => 1.125,
            _ => 1.,
        });

        // Armed by a new value, so boxes that render checked don't all pop on load.
        let seen = window.use_keyed_state((id.clone(), "kk-seen"), cx, |_, _| Seen {
            state,
            mark: Mark::of(state),
        });
        let motion = BoxMotion::new(&id, window, cx);
        if seen.read(cx).state != state {
            seen.update(cx, |seen, _| {
                seen.state = state;
                if on {
                    seen.mark = Mark::of(state);
                }
            });
            Pulse::new((id.clone(), "kk-mark"), window, cx).fire(cx);
            motion.pop.fire(cx);
        }
        let motion = BoxMotion::new(&id, window, cx);
        let mark = seen.read(cx).mark;
        let change = Pulse::new((id.clone(), "kk-mark"), window, cx);
        change.animate(MARK_DELAY + MARK_DRAW, window);
        let (drawn, mark_alpha) = mark_frame(on, change.running(MARK_DELAY + MARK_DRAW));

        let held = !self.disabled && motion.pressed.read(cx).0;
        let scale = Press {
            sink: 0.85,
            pop: if on {
                check_track()
            } else {
                release_track(cx.curves().spring)
            },
            duration: if on { CHECK } else { RELEASE },
        }
        .scale(&id, held, &motion.pop, window, cx);
        let fill = glide(
            (id.clone(), "kk-fill"),
            if on { 1.0 } else { 0.0 },
            FILL,
            Easing::EaseOut,
            window,
            cx,
        );

        let focus_handle = window
            .use_keyed_state(id.clone(), cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone();
        let is_focused = focus_handle.is_focused(window);

        let theme = cx.theme();
        let unchecked_border = theme.input;
        let checked_color = theme.primary;
        let disabled = self.disabled;
        let (box_border, box_bg) = if disabled {
            (unchecked_border.opacity(0.5), theme.input_background())
        } else {
            (unchecked_border, theme.input_background())
        };
        let fill_bg: Background = if disabled {
            checked_color.opacity(0.5).into()
        } else {
            theme.tokens.primary.into()
        };
        let fill_border = if disabled {
            checked_color.opacity(0.5)
        } else {
            checked_color
        };
        let mark_color = if disabled {
            theme.primary_foreground.opacity(0.5)
        } else {
            theme.primary_foreground
        };
        let radius = theme.radius.min(px(4.));
        let foreground = theme.foreground;
        let muted_foreground = theme.muted_foreground;
        let disabled_text_color = theme.muted_foreground;

        let accessibility_label = self.accessibility_label.clone().or_else(|| {
            self.label.as_ref().and_then(|label| match label {
                Text::String(text) => Some(text.clone()),
                Text::TextView(_) => None,
            })
        });
        let on_click = self.on_click.clone();
        let instance_style = self.style.clone();
        let tooltip = self.tooltip.clone();
        let children = self.children;

        // The box is an owned shape: resize it around its centre inside a fixed slot.
        let side = indicator_size.to_pixels(window.rem_size());
        let scaled = side * scale;
        let shift = (side - scaled) * 0.5;
        let corner = radius * scale;

        let root = self
            .base
            .role(self.role)
            .state(state)
            .disabled(disabled)
            .styles(|styles| {
                styles.disabled(|style| {
                    style
                        .text_color(disabled_text_color)
                        .refine_style(&instance_style)
                })
            })
            .tab_stop(self.tab_stop)
            .tab_index(self.tab_index)
            .track_focus(&focus_handle)
            .when_some(accessibility_label, |this, label| {
                this.accessibility_label(label)
            })
            .when_some(on_click, |this, on_click| {
                this.on_change(move |next, _, window, cx| {
                    window.prevent_default();
                    on_click(&(next == CheckboxState::Checked), window, cx);
                })
            })
            .h_flex()
            .gap_2()
            .items_start()
            .line_height(relative(1.))
            .text_color(foreground)
            .map(|this| match self.size {
                Size::XSmall => this.text_xs(),
                Size::Small => this.text_sm(),
                Size::Medium => this.text_base(),
                Size::Large => this.text_lg(),
                _ => this,
            })
            .rounded(theme.radius * 0.5)
            .when(is_focused && self.focus_ring_enabled, |this| {
                this.focus_ring_style(window, cx)
            })
            .refine_style(&self.style)
            .child(
                CheckboxIndicator::new()
                    .state(state)
                    .disabled(disabled)
                    .relative()
                    .size(indicator_size)
                    // Center on the first 1.25em line, including when the label wraps.
                    .when(has_content, |this| this.mt(indicator_size * 0.125))
                    .flex_shrink_0()
                    .child(
                        div()
                            .absolute()
                            .left(shift)
                            .top(shift)
                            .size(scaled)
                            .rounded(corner)
                            .border_1()
                            .border_color(box_border)
                            .bg(box_bg)
                            .child(
                                div()
                                    .absolute()
                                    .top(px(-1.))
                                    .left(px(-1.))
                                    .size(scaled)
                                    .rounded(corner)
                                    .border_1()
                                    .border_color(fill_border)
                                    .bg(fill_bg)
                                    .opacity(fill),
                            )
                            .when(mark_alpha > 0.0, |this| {
                                this.child(
                                    canvas(
                                        |_, _, _| {},
                                        move |bounds, _, window, _| {
                                            paint_mark(
                                                bounds,
                                                mark.points(),
                                                drawn,
                                                3.0,
                                                mark_color.opacity(mark_alpha),
                                                window,
                                            )
                                        },
                                    )
                                    .absolute()
                                    .size_full(),
                                )
                            }),
                    ),
            )
            .when(has_content, |this| {
                this.child(
                    v_flex()
                        .flex_1()
                        .overflow_hidden()
                        .line_height(relative(1.25))
                        .gap_1()
                        .when_some(self.label, |this, label| {
                            this.child(
                                div()
                                    .size_full()
                                    .text_color(foreground)
                                    .when(disabled, |this| this.text_color(muted_foreground))
                                    .child(label),
                            )
                        })
                        .children(children),
                )
            })
            .on_mouse_down(MouseButton::Left, |_, window, _| {
                // Like GPUI Component: pointer presses don't move focus.
                window.prevent_default();
            })
            .when_some(tooltip, |this, tooltip| {
                this.tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
            });
        if disabled { root } else { motion.listen(root) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_partial_polyline_stops_at_its_length() {
        let line = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)];
        assert!(partial_polyline(&line, 0.0).is_empty());
        assert_eq!(partial_polyline(&line, 0.25), vec![(0.0, 0.0), (5.0, 0.0)]);
        assert_eq!(
            partial_polyline(&line, 0.75),
            vec![(0.0, 0.0), (10.0, 0.0), (10.0, 5.0)]
        );
        assert_eq!(partial_polyline(&line, 1.0), line.to_vec());
    }

    #[test]
    fn the_tick_waits_then_draws() {
        assert_eq!(mark_frame(true, Some(ms(40))), (0.0, 0.0));
        let (drawn, alpha) = mark_frame(true, Some(ms(80 + 125)));
        assert!((drawn - 0.5).abs() < 1e-3 && alpha == 1.0);
        assert_eq!(mark_frame(true, Some(ms(400))), (1.0, 1.0));
        // At rest: drawn when on, hidden when off.
        assert_eq!(mark_frame(true, None), (1.0, 1.0));
        assert_eq!(mark_frame(false, None), (1.0, 0.0));
    }

    #[test]
    fn unchecking_fades_the_mark() {
        let (drawn, alpha) = mark_frame(false, Some(ms(60)));
        assert_eq!(drawn, 1.0);
        assert!(alpha > 0.0 && alpha < 1.0);
        assert_eq!(mark_frame(false, Some(ms(120))), (1.0, 0.0));
    }

    #[test]
    fn tracks_match_the_web() {
        use crate::parity::assert_number_track;
        let check = check_track();
        assert_number_track("pop-checkbox", "kk-pop-checkbox-check", "sx", &check);
        assert_number_track("pop-checkbox", "kk-pop-checkbox-mixed", "sx", &check);
        // The tick and the dash share the opacity keyframes; their dash offset is drawn length.
        assert_number_track(
            "pop-checkbox",
            "kk-pop-checkbox-tick",
            "opacity",
            &tick_opacity(),
        );
        assert_number_track(
            "pop-checkbox",
            "kk-pop-checkbox-dash",
            "opacity",
            &tick_opacity(),
        );
        assert_number_track(
            "pop-checkbox",
            "kk-pop-checkbox-out",
            "opacity",
            &mark_out(),
        );
    }

    #[test]
    fn the_rebound_starts_squashed() {
        let track = check_track();
        assert_eq!(track.sample(0.0), 0.85);
        assert_eq!(track.sample(0.4), 1.08);
        assert_eq!(track.sample(1.0), 1.0);
    }
}
