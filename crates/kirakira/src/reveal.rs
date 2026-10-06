//! Reveal: an entrance for any block. Pop, rise, drop, swing, slide, fade or wipe it in.
//!
//! Each effect is the web keyframes, ported:
//!
//! - `Pop` grows from nothing past full size and back. `Rise` comes up 48 px with a stretch,
//!   `Drop` falls 48 px and bounces twice, both from the bottom edge.
//! - `SlideLeft` and `SlideRight` come in from 72 px to the side and overshoot a little.
//! - `Fade` only fades. `Wipe` uncovers it from left to right.
//! - `Swing` hangs from its top edge and swings to rest.
//!
//! Composition-safe: driven by a [`Clock`], so it plays on a
//! [`Timeline`](crate::timeline::Timeline). At the end the content is exactly as it was given.
//!
//! What differs from the web: GPUI can't rotate or unevenly scale arbitrary content.
//! [`motion::transform`](crate::motion::transform) moves and fades it exactly and scales it evenly
//! (the geometric mean of the keyframe's x and y scale, so a squash keeps its size). `Swing` can't
//! turn the content, so it swings like a pendulum that stays upright: the content moves the way
//! its centre would under the rotation. The `Wipe` clip is exact, and like the web's `backwards`
//! fill it only clips until the wipe has played. There is no `once = false`
//! (hide again on leaving the viewport): a clock only starts once.

use gpui_kit::base::StyledExt as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, AvailableSpace, Bounds, Element, ElementId, GlobalElementId,
    InspectorElementId, InteractiveElement as _, IntoElement, LayoutId, ParentElement, Pixels,
    RenderOnce, StyleRefinement, Styled, Window, canvas, div, size,
};

use crate::draw::{Inset, Side, clip};
use crate::motion::{
    Clock, Easing, Keyframes, Pose, Timing, Track, Trigger, delay_ms, ms, split_layout, transform,
};
use crate::theme::ActiveKira as _;

/// How the content arrives.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RevealEffect {
    /// Grow from nothing past full size and back.
    #[default]
    Pop,
    /// Come up from below with a stretch.
    Rise,
    /// Fall from above and bounce twice.
    Drop,
    /// Hang from the top edge and swing to rest.
    Swing,
    /// Come in from the right, moving left.
    SlideLeft,
    /// Come in from the left, moving right.
    SlideRight,
    /// Fade in.
    Fade,
    /// Uncover from left to right.
    Wipe,
}

/// The distance rise, drop and slide travel, `--kk-reveal-distance`.
const DISTANCE: f32 = 48.0;

impl RevealEffect {
    /// The default length of the entrance.
    pub fn duration_ms(self) -> u64 {
        match self {
            Self::Pop => 600,
            Self::Rise => 700,
            Self::Drop => 800,
            Self::Swing => 900,
            Self::SlideLeft | Self::SlideRight => 700,
            Self::Fade => 500,
            Self::Wipe => 700,
        }
    }

    /// The `transform-origin`, as fractions of the box.
    fn origin(self) -> (f32, f32) {
        match self {
            Self::Rise | Self::Drop => (0.5, 1.0),
            Self::Swing => (0.5, 0.0),
            _ => (0.5, 0.5),
        }
    }
}

/// An effect's keyframes: the pose, the opacity, and for `Swing` the angle in degrees.
pub struct RevealTracks {
    pub pose: Keyframes<Pose>,
    pub opacity: Keyframes<f32>,
    pub angle: Option<Keyframes<f32>>,
    /// For `Wipe`, the clip's right inset as a fraction of the width.
    pub wipe: Option<Keyframes<f32>>,
}

/// The keyframes of `effect`. `out` and `snap` are the Kirakira curves.
pub fn tracks(effect: RevealEffect, out: Easing, snap: Easing) -> RevealTracks {
    let ease = Easing::EaseInOut;
    let still = || {
        Track::new(Easing::Linear)
            .at(0.0, Pose::new())
            .at(1.0, Pose::new())
    };
    let fade_in = |ease: Easing, at: f32| {
        Track::new(ease)
            .at(0.0, 0.0)
            .at(at, 1.0)
            .at(1.0, 1.0)
            .build()
    };
    let mut angle = None;
    let mut wipe = None;
    let (pose, opacity) = match effect {
        RevealEffect::Pop => (
            Track::new(ease.clone())
                .at(0.0, Pose::new().scale(0.0))
                .at(0.5, Pose::new().scale_xy(1.2, 1.25))
                .at(0.75, Pose::new().scale_xy(0.9, 0.95))
                .at(1.0, Pose::new())
                .build(),
            fade_in(ease, 0.2),
        ),
        RevealEffect::Rise => (
            Track::new(out.clone())
                .at(0.0, Pose::new().y(DISTANCE))
                .at(0.6, Pose::new().y(DISTANCE * -0.12).scale_xy(0.97, 1.05))
                .at(0.8, Pose::new().scale_xy(1.02, 0.98))
                .at(1.0, Pose::new())
                .build(),
            fade_in(out, 0.3),
        ),
        RevealEffect::Drop => {
            let fall = Easing::cubic_bezier(0.6, 0.0, 1.0, 0.7).expect("valid curve");
            let land = Easing::cubic_bezier(0.0, 0.4, 0.4, 1.0).expect("valid curve");
            (
                Track::new(ease.clone())
                    .at_ease(0.0, Pose::new().y(-DISTANCE), fall.clone())
                    .at_ease(0.4, Pose::new().scale_xy(1.05, 0.94), land.clone())
                    .at_ease(0.58, Pose::new().y(DISTANCE * -0.3), fall.clone())
                    .at_ease(0.74, Pose::new().scale_xy(1.02, 0.98), land)
                    .at_ease(0.87, Pose::new().y(DISTANCE * -0.1), fall.clone())
                    .at(1.0, Pose::new())
                    .build(),
                // The 0% keyframe's timing function covers the opacity too.
                Track::new(ease)
                    .at_ease(0.0, 0.0, fall)
                    .at(0.15, 1.0)
                    .at(1.0, 1.0)
                    .build(),
            )
        }
        RevealEffect::Swing => {
            angle = Some(
                Track::new(ease.clone())
                    .at(0.0, 20.0)
                    .at(0.4, -10.0)
                    .at(0.65, 5.0)
                    .at(0.85, -2.5)
                    .at(1.0, 0.0)
                    .build(),
            );
            (still().build(), fade_in(ease, 0.25))
        }
        RevealEffect::SlideLeft | RevealEffect::SlideRight => {
            let from = if effect == RevealEffect::SlideLeft {
                1.0
            } else {
                -1.0
            };
            (
                Track::new(out.clone())
                    .at(0.0, Pose::new().x(DISTANCE * from * 1.5))
                    .at(0.7, Pose::new().x(DISTANCE * from * -0.08))
                    .at(1.0, Pose::new())
                    .build(),
                fade_in(out, 0.4),
            )
        }
        RevealEffect::Fade => (still().build(), fade_in(Easing::EaseOut, 1.0)),
        RevealEffect::Wipe => {
            wipe = Some(Track::new(snap).at(0.0, 1.0).at(1.0, -0.5).build());
            (
                still().build(),
                Track::new(Easing::Linear).at(0.0, 1.0).at(1.0, 1.0).build(),
            )
        }
    };
    RevealTracks {
        pose,
        opacity,
        angle,
        wipe,
    }
}

/// Where the centre of a box `height` tall goes when it turns `degrees` clockwise about the middle
/// of its top edge: the pendulum offset `Swing` moves the content by.
pub fn swing_offset(degrees: f32, height: f32) -> (f32, f32) {
    let (sin, cos) = degrees.to_radians().sin_cos();
    let arm = height / 2.0;
    (-arm * sin, arm * (cos - 1.0))
}

#[derive(Default)]
struct Measured(Option<Pixels>);

/// Plays an entrance on its children.
///
/// ```ignore
/// Reveal::new("card").effect(RevealEffect::Drop).delay(300).child(card)
/// ```
#[derive(IntoElement)]
pub struct Reveal {
    id: ElementId,
    effect: RevealEffect,
    delay: u64,
    duration: Option<u64>,
    trigger: Trigger,
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl Reveal {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            effect: RevealEffect::default(),
            delay: 200,
            duration: None,
            trigger: Trigger::InView,
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }

    pub fn effect(mut self, effect: RevealEffect) -> Self {
        self.effect = effect;
        self
    }

    /// Milliseconds before the entrance starts. 200 by default.
    pub fn delay(mut self, delay_ms: u64) -> Self {
        self.delay = delay_ms;
        self
    }

    /// Milliseconds for the entrance. Each effect has its own default; see
    /// [`RevealEffect::duration_ms`].
    pub fn duration(mut self, duration_ms: u64) -> Self {
        self.duration = Some(duration_ms);
        self
    }

    /// When the entrance starts. [`Trigger::InView`] by default.
    pub fn trigger(mut self, trigger: Trigger) -> Self {
        self.trigger = trigger;
        self
    }
}

impl Styled for Reveal {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Reveal {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Reveal {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let clock = Clock::new(self.id.clone(), self.trigger, window, cx);
        let duration = self.duration.unwrap_or(self.effect.duration_ms());
        clock.animate(Some(ms(self.delay + duration)), window);
        let curves = cx.curves();
        let tracks = tracks(self.effect, curves.out, curves.snap);
        let timing = Timing::new(ms(duration)).delay(delay_ms(self.delay));
        let sample = timing.sample(clock.elapsed());
        let progress = sample.directed_progress;
        let mut pose = tracks.pose.sample(progress);
        pose.opacity = tracks.opacity.sample(progress);

        // Swing needs the content's height to place the pendulum; measure it as it lays out.
        let measured = (self.effect == RevealEffect::Swing).then(|| {
            window.use_keyed_state((self.id.clone(), "kk-size"), cx, |_, _| Measured::default())
        });
        if let (Some(angle), Some(measured)) = (&tracks.angle, &measured) {
            let height = measured.read(cx).0.map_or(0.0, f32::from);
            let (x, y) = swing_offset(angle.sample(progress), height);
            pose.x += x;
            pose.y += y;
        }

        let outer = split_layout(&mut self.style);
        let (origin_x, origin_y) = self.effect.origin();
        let content = div()
            .id(self.id.clone())
            .relative()
            .refine_style(&self.style)
            .children(self.children)
            .when_some(measured, |this, measured| {
                this.child(
                    canvas(
                        move |bounds, window, cx| {
                            let height = Some(bounds.size.height);
                            let changed = measured.update(cx, |measured, _| {
                                std::mem::replace(&mut measured.0, height) != height
                            });
                            if changed {
                                window.refresh();
                            }
                        },
                        |_, _, _, _| {},
                    )
                    .absolute()
                    .size_full(),
                )
            });
        let moved = transform((self.id.clone(), "kk-pose"), pose, content)
            .origin(origin_x, origin_y)
            .outer_style(outer);
        // Like the web's `backwards` fill, the wipe clips only until it has played: after that,
        // and under reduced motion, shadows and overflow show as they would without a Reveal.
        let wipe = tracks.wipe.filter(|_| !sample.finished && !clock.reduced());
        let element = match wipe {
            Some(wipe) => {
                let right = wipe.sample(progress);
                let open = Side::frac(-0.5);
                clip(Inset::new(open, Side::frac(right), open, open), moved).into_any_element()
            }
            None => moved.into_any_element(),
        };
        // Watched from outside the clip: before the wipe starts the clip is empty, and the
        // content inside it never counts as visible.
        observe_unclipped(&clock, element)
    }
}

/// Wires up an [`Trigger::InView`] `clock` on `child`'s own bounds, but from outside whatever
/// `child` clips: an entrance that starts fully clipped (a wipe, an iris) is still seen.
pub(crate) fn observe_unclipped(clock: &Clock, child: impl IntoElement) -> Unclipped {
    Unclipped {
        clock: (!clock.started()).then(|| clock.clone()),
        child: child.into_any_element(),
        observer: None,
    }
}

/// See [`observe_unclipped`]. Layout is the child's own.
pub(crate) struct Unclipped {
    clock: Option<Clock>,
    child: AnyElement,
    observer: Option<AnyElement>,
}

impl IntoElement for Unclipped {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for Unclipped {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.child.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.prepaint(window, cx);
        if let Some(clock) = &self.clock {
            // A box of the child's size, laid out on its own at the child's origin, so the
            // observer sees the child's bounds under this element's content mask.
            let mut observer = clock
                .observe(div().w(bounds.size.width).h(bounds.size.height))
                .into_any_element();
            let space = size(
                AvailableSpace::Definite(bounds.size.width),
                AvailableSpace::Definite(bounds.size.height),
            );
            observer.prepaint_as_root(bounds.origin, space, window, cx);
            self.observer = Some(observer);
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.paint(window, cx);
        if let Some(observer) = &mut self.observer {
            observer.paint(window, cx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all() -> [RevealEffect; 8] {
        [
            RevealEffect::Pop,
            RevealEffect::Rise,
            RevealEffect::Drop,
            RevealEffect::Swing,
            RevealEffect::SlideLeft,
            RevealEffect::SlideRight,
            RevealEffect::Fade,
            RevealEffect::Wipe,
        ]
    }

    #[test]
    fn every_effect_ends_at_rest_and_visible() {
        for effect in all() {
            let tracks = tracks(effect, Easing::EaseOut, Easing::EaseInOut);
            assert!(tracks.pose.sample(1.0).is_identity(), "{effect:?}");
            assert_eq!(tracks.opacity.sample(1.0), 1.0, "{effect:?}");
            if let Some(angle) = tracks.angle {
                assert_eq!(angle.sample(1.0), 0.0);
            }
            if let Some(wipe) = tracks.wipe {
                assert_eq!(wipe.sample(0.0), 1.0);
                assert_eq!(wipe.sample(1.0), -0.5);
            }
        }
    }

    // Rise, drop and the slides offset by `var(--kk-reveal-distance)` inside `calc()`, which the
    // parity parser can't read, and wipe animates `clip-path`; those are covered by the tests
    // below and the gallery shots instead.
    #[test]
    fn pop_swing_and_fade_match_the_web_keyframes() {
        use crate::parity::{assert_number_track, assert_pose_track};
        let pop = tracks(RevealEffect::Pop, Easing::EaseOut, Easing::EaseInOut);
        assert_pose_track("reveal", "kk-reveal-pop", &pop.pose, &[]);
        assert_number_track("reveal", "kk-reveal-pop", "opacity", &pop.opacity);
        let swing = tracks(RevealEffect::Swing, Easing::EaseOut, Easing::EaseInOut);
        let angle = swing.angle.expect("swing turns");
        assert_number_track("reveal", "kk-reveal-swing", "rotate", &angle);
        assert_number_track("reveal", "kk-reveal-swing", "opacity", &swing.opacity);
        let fade = tracks(RevealEffect::Fade, Easing::EaseOut, Easing::EaseInOut);
        assert_number_track("reveal", "kk-reveal-fade", "opacity", &fade.opacity);
    }

    #[test]
    fn slides_come_from_opposite_sides() {
        let left = tracks(RevealEffect::SlideLeft, Easing::Linear, Easing::Linear);
        let right = tracks(RevealEffect::SlideRight, Easing::Linear, Easing::Linear);
        assert_eq!(left.pose.sample(0.0).x, 72.0);
        assert_eq!(right.pose.sample(0.0).x, -72.0);
    }

    #[test]
    fn a_clockwise_swing_carries_the_centre_left() {
        let (x, y) = swing_offset(20.0, 100.0);
        assert!((x + 50.0 * 20f32.to_radians().sin()).abs() < 1e-4);
        assert!(x < 0.0 && y < 0.0);
        assert_eq!(swing_offset(0.0, 100.0), (0.0, 0.0));
    }
}
