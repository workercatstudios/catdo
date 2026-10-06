//! Slider: a knob that grows on hover, stretches while held and wobbles when let go.
//!
//! Replaces `gpui_kit::component::slider`, rebuilt on gpui-base with GPUI Component's structure,
//! sizes, colours and hover ring. `SliderState`, `SliderEvent`, `SliderScale` and `SliderValue`
//! are re-exported.
//!
//! The thumb is a still hit target; its knob does the moving. The knob grows to 1.15 on hover
//! (0.16 s ease-out). Held, it stretches to 1.25 × 0.82 (0.15 s ease-out); let go, it wobbles back
//! with each rebound about half the last: 1.25 × 0.82 → 0.9 × 1.1 → 1.05 × 0.96 → 0.98 × 1.02 → 1
//! in 0.4 s. With [`Slider::show_value`] a bubble above the thumb shows the value while it is
//! dragged, popping 0.4 → 1.12 → 0.96 → 1 in 0.3 s (opaque by 30 %) and shrinking away in 0.12 s.
//! Under reduced motion the knob doesn't grow, stretch or wobble, and the bubble appears and goes
//! at once.
//!
//! All of it, GPUI Component's hover ring included, is timed by
//! [`motion::now`](crate::motion::now), so screenshots can pin any frame.
//!
//! Differences: the knob is an owned shape, so the uneven stretch is exact, but it stays a pill
//! (rounded to its shorter side) where CSS would scale the circle into an ellipse. GPUI
//! Component's slider takes no keyboard focus and no keys, so the web's keyboard focus (which
//! grows the knob and shows the bubble) has nothing to hang on: the bubble shows only while the
//! pointer drags.

use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui_kit::base::{Slider as BaseSlider, SliderIndicator, SliderThumb, SliderTrack};
use gpui_kit::component::{ActiveTheme as _, AxisExt as _, StyledExt as _, ThemeStyled as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    App, Axis, Background, Corners, DefiniteLength, ElementId, Entity, EntityId, FontWeight, Hsla,
    InteractiveElement as _, IntoElement, MouseButton, ParentElement as _, Pixels, RenderOnce,
    SharedString, SpringConfig, SpringState, StatefulInteractiveElement as _, StyleRefinement,
    Styled, Window, div, px, relative,
};

pub use gpui_kit::base::slider::{SliderEvent, SliderScale, SliderState, SliderValue};

use crate::glide::glide;
use crate::motion::{Easing, Keyframes, Pose, Pulse, Track, ms, now, transform};

const THUMB_RING_WIDTH: Pixels = px(3.);
const THUMB_RING_OPACITY: f32 = 0.5;
const GROW: Duration = ms(160);
const HOLD: Duration = ms(150);
const RELEASE: Duration = ms(400);
const BUBBLE_IN: Duration = ms(300);
const BUBBLE_OUT: Duration = ms(120);

/// `kk-pop-slider-hold`: from rest to the stretch.
fn hold_track() -> Keyframes<Pose> {
    Track::new(Easing::EaseOut)
        .at(0.0, Pose::new())
        .at(1.0, Pose::new().scale_xy(1.25, 0.82))
        .build()
}

/// `kk-pop-slider-release-a`/`-b`: the wobble back.
fn release_track() -> Keyframes<Pose> {
    Track::new(Easing::EaseInOut)
        .at(0.0, Pose::new().scale_xy(1.25, 0.82))
        .at(0.3, Pose::new().scale_xy(0.9, 1.1))
        .at(0.55, Pose::new().scale_xy(1.05, 0.96))
        .at(0.78, Pose::new().scale_xy(0.98, 1.02))
        .at(1.0, Pose::new())
        .build()
}

/// `kk-pop-slider-value`: the bubble's scale.
fn bubble_scale() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at(0.0, 0.4)
        .at(0.5, 1.12)
        .at(0.75, 0.96)
        .at(1.0, 1.0)
        .build()
}

/// `kk-pop-slider-value`: the bubble's opacity, done by 30 %.
fn bubble_opacity() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at(0.0, 0.0)
        .at(0.3, 1.0)
        .at(1.0, 1.0)
        .build()
}

/// GPUI Component's `spring_control`, which its slider's hover ring springs on: gpui-base's
/// `Spring::new(180 ms)`, critically damped, settled within 0.001. gpui-base's `spring` reads
/// GPUI's clock, and its `Spring` keeps these private, so they're restated here.
const RING_RESPONSE: Duration = ms(180);
const RING_DAMPING: f32 = 1.0;
const RING_EPSILON: f32 = 0.001;

/// The ring's spring, as gpui-base builds it from a response and a damping ratio.
fn ring_config() -> SpringConfig {
    let frequency = std::f32::consts::TAU / RING_RESPONSE.as_secs_f32();
    SpringConfig::new(frequency * frequency, 2.0 * RING_DAMPING * frequency, 1.0)
}

/// A damped spring as a pure function of time: where it is `elapsed` after it set off from `from`
/// towards `target`, and whether it has settled.
fn spring_at(from: SpringState, target: f32, elapsed: Duration) -> (f32, bool) {
    let config = ring_config();
    let state = config.step(from, target, elapsed.as_secs_f32());
    if config.is_settled(state, target, RING_EPSILON) {
        (target, true)
    } else {
        (state.position, false)
    }
}

/// Where a [`ring_spring`] set off from, and when.
struct SpringAnchor {
    from: SpringState,
    target: f32,
    at: Option<Instant>,
}

impl SpringAnchor {
    fn state(&self, now: Instant) -> SpringState {
        let Some(at) = self.at else {
            return SpringState {
                position: self.target,
                velocity: 0.0,
            };
        };
        let elapsed = now.saturating_duration_since(at).as_secs_f32();
        ring_config().step(self.from, self.target, elapsed)
    }
}

/// gpui-base's `spring` with [`RING_RESPONSE`], on Kirakira's clock. A new target sets off from
/// where the spring is, keeping its velocity. Each frame is a function of the time since the last
/// retarget, so a pinned time gives the same frame every run. Under reduced motion it jumps.
fn ring_spring(id: ElementId, target: f32, window: &mut Window, cx: &mut App) -> f32 {
    let now = now();
    let state = window.use_keyed_state(id, cx, |_, _| SpringAnchor {
        from: SpringState {
            position: target,
            velocity: 0.0,
        },
        target,
        at: None,
    });
    let reduced = cx.reduce_motion();
    if state.read(cx).target != target {
        let from = state.read(cx).state(now);
        state.update(cx, |anchor, _| {
            anchor.from = from;
            anchor.target = target;
            anchor.at = (!reduced).then_some(now);
        });
    }
    let anchor = state.read(cx);
    let Some(at) = anchor.at.filter(|_| !reduced) else {
        return target;
    };
    let (value, settled) = spring_at(anchor.from, target, now.saturating_duration_since(at));
    if !settled {
        window.request_animation_frame();
    }
    value
}

fn progress(elapsed: Duration, duration: Duration) -> f32 {
    (elapsed.as_secs_f32() / duration.as_secs_f32()).min(1.0)
}

/// The knob's stretch: while held, from the press; after, the wobble from the release.
pub(crate) fn knob_stretch(
    held: bool,
    since_press: Option<Duration>,
    since_release: Option<Duration>,
) -> (f32, f32) {
    let pose = match (held, since_press, since_release) {
        (true, Some(elapsed), _) => hold_track().sample(progress(elapsed, HOLD)),
        (false, _, Some(elapsed)) if elapsed < RELEASE => {
            release_track().sample(progress(elapsed, RELEASE))
        }
        _ => Pose::new(),
    };
    (pose.sx, pose.sy)
}

/// The bubble's scale and opacity, or `None` when it's hidden.
pub(crate) fn bubble_frame(
    held: bool,
    since_press: Option<Duration>,
    since_release: Option<Duration>,
    reduced: bool,
) -> Option<(f32, f32)> {
    if reduced {
        return held.then_some((1.0, 1.0));
    }
    match (held, since_press, since_release) {
        (true, Some(elapsed), _) => {
            let t = progress(elapsed, BUBBLE_IN);
            Some((bubble_scale().sample(t), bubble_opacity().sample(t)))
        }
        (true, None, _) => Some((1.0, 1.0)),
        (false, _, Some(elapsed)) if elapsed < BUBBLE_OUT => {
            // Out on `transition: opacity 0.12s ease-in, scale 0.12s ease-in`.
            let t = Easing::EaseIn.sample(progress(elapsed, BUBBLE_OUT));
            Some((1.0 + (0.4 - 1.0) * t, 1.0 - t))
        }
        _ => None,
    }
}

/// The id of one thumb's piece of state.
fn keyed(name: &'static str, slider: EntityId, channel: &'static str) -> ElementId {
    ElementId::NamedChild(Arc::new((name, slider).into()), channel.into())
}

/// Pointer state of one thumb.
#[derive(Default)]
struct ThumbInteraction {
    hovered: bool,
    pressed: bool,
}

/// One thumb's state and pulses.
#[derive(Clone)]
struct Thumb {
    interaction: Entity<ThumbInteraction>,
    press: Pulse,
    release: Pulse,
}

impl Thumb {
    fn new(id: EntityId, start: bool, window: &mut Window, cx: &mut App) -> Self {
        let channel = if start { "start" } else { "end" };
        let key = |name: &'static str| {
            ElementId::NamedChild(Arc::new(("kk-slider-thumb", id).into()), name.into())
        };
        Self {
            interaction: window.use_keyed_state(
                ElementId::NamedChild(Arc::new(key(channel)), "interaction".into()),
                cx,
                |_, _| ThumbInteraction::default(),
            ),
            press: Pulse::new((key(channel), "kk-press"), window, cx),
            release: Pulse::new((key(channel), "kk-release"), window, cx),
        }
    }

    fn set_pressed(&self, pressed: bool, cx: &mut App) {
        let was = self.interaction.read(cx).pressed;
        if was == pressed {
            return;
        }
        self.interaction.update(cx, |interaction, cx| {
            interaction.pressed = pressed;
            cx.notify();
        });
        if pressed {
            self.press.fire(cx);
        } else {
            self.release.fire(cx);
        }
    }

    fn set_hovered(&self, hovered: bool, cx: &mut App) {
        self.interaction.update(cx, |interaction, cx| {
            if interaction.hovered != hovered {
                interaction.hovered = hovered;
                cx.notify();
            }
        });
    }
}

/// A slider bound to a [`SliderState`].
#[derive(IntoElement)]
pub struct Slider {
    state: Entity<SliderState>,
    axis: Axis,
    style: StyleRefinement,
    disabled: bool,
    reverse: bool,
    show_value: bool,
    format_value: Option<Rc<dyn Fn(f32) -> SharedString>>,
}

impl Slider {
    pub fn new(state: &Entity<SliderState>) -> Self {
        Self {
            axis: Axis::Horizontal,
            state: state.clone(),
            style: StyleRefinement::default(),
            disabled: false,
            reverse: false,
            show_value: false,
            format_value: None,
        }
    }

    pub fn horizontal(mut self) -> Self {
        self.axis = Axis::Horizontal;
        self
    }

    pub fn vertical(mut self) -> Self {
        self.axis = Axis::Vertical;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Fill from the thumb to the max end instead. Single-value sliders only.
    pub fn reverse(mut self) -> Self {
        self.reverse = true;
        self
    }

    /// Show each thumb's value in a bubble above it while it is dragged. Not in GPUI Component.
    pub fn show_value(mut self, show: bool) -> Self {
        self.show_value = show;
        self
    }

    /// Format the value in the bubble, e.g. `|v| format!("{v} px").into()`.
    pub fn format_value(mut self, format: impl Fn(f32) -> SharedString + 'static) -> Self {
        self.format_value = Some(Rc::new(format));
        self
    }
}

impl Styled for Slider {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Slider {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let axis = self.axis;
        let state = self.state.read(cx);
        let value = state.value();
        let is_range = value.is_range();
        let percentage = state.percentage();
        let (bar_start, bar_end) = if self.reverse && !is_range {
            (relative(percentage.end), relative(0.))
        } else {
            (relative(percentage.start), relative(1. - percentage.end))
        };
        let rem_size = window.rem_size();
        let reduced = cx.reduce_motion();

        let bar_color = self
            .style
            .background
            .clone()
            .and_then(|bg| bg.color())
            .unwrap_or(cx.theme().tokens.slider_bar.into());
        let thumb_bg: Background = self
            .style
            .text
            .color
            .map(Into::into)
            .unwrap_or_else(|| cx.theme().tokens.slider_thumb.into());
        let corner_radii = self.style.corner_radii.clone();
        let default_radius = cx.theme().radius_full();
        let corner = |value: Option<gpui_kit::AbsoluteLength>| {
            value
                .map(|v| v.to_pixels(rem_size))
                .unwrap_or(default_radius)
        };
        let radius = Corners {
            top_left: corner(corner_radii.top_left),
            top_right: corner(corner_radii.top_right),
            bottom_left: corner(corner_radii.bottom_left),
            bottom_right: corner(corner_radii.bottom_right),
        };

        let ring_color = cx.theme().ring;
        let entity_id = self.state.entity_id();
        let start_thumb = is_range.then(|| Thumb::new(entity_id, true, window, cx));
        let end_thumb = Thumb::new(entity_id, false, window, cx);
        let primary = cx.theme().primary;
        let primary_foreground = cx.theme().primary_foreground;
        let format = self
            .format_value
            .clone()
            .unwrap_or_else(|| Rc::new(|v: f32| SharedString::from(v.to_string())));

        // Everything about one thumb that depends on time, sampled while `window` is free.
        struct ThumbFrame {
            thumb: Thumb,
            start: bool,
            position: DefiniteLength,
            ring_width: Pixels,
            ring_color: Hsla,
            knob: (f32, f32),
            bubble: Option<(f32, f32, SharedString)>,
        }
        let frame = |thumb: Thumb, start: bool, window: &mut Window, cx: &mut App| {
            let interaction = thumb.interaction.read(cx);
            let (hovered, held) = (interaction.hovered, interaction.pressed);
            let channel = if start { "start" } else { "end" };
            let ring = ring_spring(
                keyed("kk-slider-ring", entity_id, channel),
                if hovered || held { 1. } else { 0. },
                window,
                cx,
            );
            // The web drops the hover scale under reduced motion (`scale: none`).
            let grow = glide(
                keyed("kk-slider-grow", entity_id, channel),
                if (hovered || held) && !reduced {
                    1.15
                } else {
                    1.0
                },
                GROW,
                Easing::EaseOut,
                window,
                cx,
            );
            let (since_press, since_release) = (thumb.press.elapsed(), thumb.release.elapsed());
            let (sx, sy) = knob_stretch(held, since_press, since_release);
            thumb.press.animate(BUBBLE_IN, window);
            thumb.release.animate(RELEASE, window);
            let value = if start { value.start() } else { value.end() };
            let bubble = self
                .show_value
                .then(|| bubble_frame(held, since_press, since_release, reduced))
                .flatten()
                .map(|(scale, opacity)| (scale, opacity, format(value)));
            ThumbFrame {
                thumb,
                start,
                position: relative(if start {
                    percentage.start
                } else {
                    percentage.end
                }),
                ring_width: THUMB_RING_WIDTH * ring,
                ring_color: ring_color.alpha(THUMB_RING_OPACITY * ring),
                knob: (grow * sx, grow * sy),
                bubble,
            }
        };
        let start_frame = start_thumb.map(|thumb| frame(thumb, true, window, cx));
        let end_frame = frame(end_thumb.clone(), false, window, cx);

        let disabled = self.disabled;
        let state_entity = self.state.clone();
        let render_thumb = move |frame: ThumbFrame| {
            let ThumbFrame {
                thumb,
                start,
                position,
                ring_width,
                ring_color,
                knob,
                bubble,
            } = frame;
            let side = px(16.);
            let (w, h) = (side * knob.0, side * knob.1);
            let id = keyed(
                "kk-slider-bubble",
                entity_id,
                if start { "start" } else { "end" },
            );
            let (hover, down, up, up_out) =
                (thumb.clone(), thumb.clone(), thumb.clone(), thumb.clone());
            SliderThumb::new(&state_entity)
                .axis(axis)
                .start(start)
                .disabled(disabled)
                .when(!disabled, |this| {
                    this.absolute()
                        .when(axis.is_horizontal(), |this| {
                            this.top(px(-5.)).left(position).ml(-px(8.))
                        })
                        .when(axis.is_vertical(), |this| {
                            this.bottom(position).left(px(-5.)).mb(-px(8.))
                        })
                        .flex_shrink_0()
                        .size_4()
                        .on_hover(move |entered, _, cx| hover.set_hovered(*entered, cx))
                        // The base thumb stops propagation on mouse down to own the drag, so the
                        // press is read in the capture phase.
                        .capture_any_mouse_down(move |_, _, cx| down.set_pressed(true, cx))
                        .capture_any_mouse_up(move |_, _, cx| up.set_pressed(false, cx))
                        .on_mouse_up_out(MouseButton::Left, move |_, _, cx| {
                            up_out.set_pressed(false, cx)
                        })
                        // The knob: an owned shape, stretched about its centre.
                        .child(
                            div()
                                .absolute()
                                .left((side - w) * 0.5)
                                .top((side - h) * 0.5)
                                .w(w)
                                .h(h)
                                .child(
                                    div()
                                        .flex_none()
                                        .absolute()
                                        .top(-ring_width)
                                        .left(-ring_width)
                                        .right(-ring_width)
                                        .bottom(-ring_width)
                                        .rounded(px(999.))
                                        .border(ring_width)
                                        .border_color(ring_color),
                                )
                                .child(
                                    div()
                                        .size_full()
                                        .p(px(1.))
                                        .rounded(px(999.))
                                        .bg(bar_color.opacity(0.5))
                                        .child(div().size_full().rounded(px(999.)).bg(thumb_bg)),
                                ),
                        )
                        .when_some(bubble, |this, (scale, opacity, text)| {
                            this.child(
                                div()
                                    .absolute()
                                    .left(side * 0.5 - px(100.))
                                    .w(px(200.))
                                    .bottom(side + px(10.))
                                    .flex()
                                    .justify_center()
                                    .items_end()
                                    .child(
                                        transform(
                                            id,
                                            Pose::new().scale(scale).opacity(opacity),
                                            div()
                                                .rounded_full()
                                                .bg(primary)
                                                .px_2()
                                                .py_0p5()
                                                .text_xs()
                                                .font_weight(FontWeight::BOLD)
                                                .whitespace_nowrap()
                                                .text_color(primary_foreground)
                                                .child(text),
                                        )
                                        .origin(0.5, 1.0),
                                    ),
                            )
                        })
                })
        };

        let (root_down, root_up, root_up_out) =
            (end_thumb.clone(), end_thumb.clone(), end_thumb.clone());
        BaseSlider::new(&self.state)
            .axis(axis)
            .disabled(disabled)
            .flex()
            .flex_1()
            .items_center()
            .justify_center()
            .when(axis.is_vertical(), |this| this.h(px(120.)))
            .when(axis.is_horizontal(), |this| this.w_full())
            .refine_style(&self.style)
            .bg(cx.theme().transparent)
            .text_color(cx.theme().foreground)
            .child(
                SliderTrack::new(&self.state)
                    .axis(axis)
                    .disabled(disabled)
                    // A press on the track drags a lone thumb too, so it stretches like a thumb
                    // press.
                    .when(!disabled && !is_range, |this| {
                        this.capture_any_mouse_down(move |_, _, cx| root_down.set_pressed(true, cx))
                    })
                    .when(!disabled, |this| {
                        this.capture_any_mouse_up(move |_, _, cx| root_up.set_pressed(false, cx))
                            .on_mouse_up_out(MouseButton::Left, move |_, _, cx| {
                                root_up_out.set_pressed(false, cx)
                            })
                    })
                    .flex()
                    .when(axis.is_horizontal(), |this| {
                        this.items_center().h_6().w_full()
                    })
                    .when(axis.is_vertical(), |this| {
                        this.justify_center().w_6().h_full()
                    })
                    .flex_shrink_0()
                    .child(
                        SliderIndicator::new(&self.state)
                            .relative()
                            .when(axis.is_horizontal(), |this| this.w_full().h_1p5())
                            .when(axis.is_vertical(), |this| this.h_full().w_1p5())
                            .bg(bar_color.opacity(0.2))
                            .active(|this| this.bg(bar_color.opacity(0.4)))
                            .corner_radii(radius)
                            .child(
                                div()
                                    .absolute()
                                    .when(axis.is_horizontal(), |this| {
                                        this.h_full().left(bar_start).right(bar_end)
                                    })
                                    .when(axis.is_vertical(), |this| {
                                        this.w_full().bottom(bar_start).top(bar_end)
                                    })
                                    .bg(bar_color)
                                    .rounded_full_style(cx),
                            )
                            .when_some(start_frame, |this, frame| this.child(render_thumb(frame)))
                            .child(render_thumb(end_frame)),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_match_the_web() {
        use crate::parity::{assert_number_track, assert_pose_track};
        assert_pose_track("pop-slider", "kk-pop-slider-hold", &hold_track(), &[]);
        assert_pose_track(
            "pop-slider",
            "kk-pop-slider-release-a",
            &release_track(),
            &[],
        );
        assert_pose_track(
            "pop-slider",
            "kk-pop-slider-release-b",
            &release_track(),
            &[],
        );
        assert_number_track("pop-slider", "kk-pop-slider-value", "sx", &bubble_scale());
        assert_number_track(
            "pop-slider",
            "kk-pop-slider-value",
            "opacity",
            &bubble_opacity(),
        );
    }

    #[test]
    fn the_knob_stretches_then_wobbles() {
        assert_eq!(knob_stretch(false, None, None), (1.0, 1.0));
        assert_eq!(knob_stretch(true, Some(ms(500)), None), (1.25, 0.82));
        let (sx, sy) = knob_stretch(false, Some(ms(900)), Some(ms(120)));
        assert!((sx - 0.9).abs() < 1e-4 && (sy - 1.1).abs() < 1e-4);
        assert_eq!(knob_stretch(false, None, Some(ms(400))), (1.0, 1.0));
    }

    #[test]
    fn the_ring_springs_out_without_overshooting_and_settles() {
        let rest = SpringState {
            position: 0.0,
            velocity: 0.0,
        };
        assert_eq!(spring_at(rest, 1.0, Duration::ZERO), (0.0, false));
        let mut last = 0.0;
        for step in 1..=40 {
            let (value, _) = spring_at(rest, 1.0, ms(step * 10));
            assert!(value >= last && value <= 1.0, "{value} at {} ms", step * 10);
            last = value;
        }
        // Critically damped over a 180 ms response: most of the way by then, settled soon after.
        let (at_response, _) = spring_at(rest, 1.0, RING_RESPONSE);
        assert!(at_response > 0.9, "{at_response}");
        assert_eq!(spring_at(rest, 1.0, ms(400)), (1.0, true));
    }

    #[test]
    fn a_retargeted_ring_keeps_its_velocity() {
        let anchor = SpringAnchor {
            from: SpringState {
                position: 0.0,
                velocity: 0.0,
            },
            target: 1.0,
            at: Some(Instant::now()),
        };
        let midway = anchor.state(anchor.at.unwrap() + ms(60));
        assert!(midway.position > 0.0 && midway.position < 1.0);
        assert!(midway.velocity > 0.0);
        // Sent back to 0, it carries on outwards for a moment before turning.
        let (next, _) = spring_at(midway, 0.0, ms(5));
        assert!(next > midway.position);
    }

    #[test]
    fn the_bubble_pops_in_and_shrinks_away() {
        assert_eq!(bubble_frame(false, None, None, false), None);
        assert_eq!(
            bubble_frame(true, Some(ms(0)), None, false),
            Some((0.4, 0.0))
        );
        assert_eq!(
            bubble_frame(true, Some(ms(150)), None, false),
            Some((1.12, 1.0))
        );
        let (scale, opacity) = bubble_frame(false, None, Some(ms(60)), false).unwrap();
        assert!(scale < 1.0 && scale > 0.4 && opacity < 1.0 && opacity > 0.0);
        assert_eq!(
            bubble_frame(true, Some(ms(10)), None, true),
            Some((1.0, 1.0))
        );
    }
}
