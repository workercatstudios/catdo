//! The motion core: Kirakira's CSS keyframes, ported.
//!
//! Every animation is a pure function of time, as it is on the web, where a `Timeline` can seek any
//! composition to any frame:
//!
//! - A [`Clock`] says how long an animation has been running. It starts when the element mounts or
//!   first scrolls into view ([`Trigger`]), or reads the time of the [`Timeline`](crate::timeline)
//!   it is inside.
//! - A [`Track`] is a CSS `@keyframes` rule: values at offsets, with the easing of each segment.
//!   [`Timing`] is the rest of the `animation` shorthand: duration, delay, iterations, direction.
//! - A [`Pose`] holds what CSS animates with `transform` and `opacity`. [`Transform`] applies one to
//!   any element: translate and opacity exactly, scale by scaling the rem size of the subtree.
//! - [`Pulse`] restarts a one-shot animation from an event, the way the web components flip
//!   between two identical keyframes on each click.
//!
//! Under `prefers-reduced-motion` (GPUI's [`App::reduce_motion`](gpui_kit::App::reduce_motion)),
//! clocks report entrances as finished and loops stop, so every component shows its final state.

mod clock;
mod pose;
mod pulse;
mod scope;
mod track;
mod transform;

use std::time::Duration;

pub use clock::{Clock, ScopeTime, Trigger, freeze_time, now, scope_time, with_scope_time};
pub use gpui_kit::base::{
    Easing, IterationCount, Keyframe, Keyframes, MotionPhase, PlaybackDirection, SignedDuration,
    Stagger, StaggerOrigin, Timing, TimingSample,
};
pub use pose::Pose;
pub use pulse::Pulse;
pub(crate) use scope::rescaled;
pub use scope::scoped;
pub use track::Track;
pub use transform::{Transform, split_layout, transform};

/// A CSS `cubic-bezier()` timing function that may overshoot.
///
/// gpui-base's `Easing::cubic_bezier` clamps its output to `0..=1`, which flattens curves such as
/// Kirakira's spring, `cubic-bezier(0.34, 1.56, 0.64, 1)`. This one returns the curve's real `y`,
/// past 1 or below 0, as CSS does. `x1` and `x2` are clamped to `0..=1`, as CSS requires.
pub fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32) -> Easing {
    let (x1, x2) = (x1.clamp(0.0, 1.0), x2.clamp(0.0, 1.0));
    let (cx, cy) = (3.0 * x1, 3.0 * y1);
    let (bx, by) = (3.0 * (x2 - x1) - cx, 3.0 * (y2 - y1) - cy);
    let (ax, ay) = (1.0 - cx - bx, 1.0 - cy - by);
    let sample_x = move |s: f32| ((ax * s + bx) * s + cx) * s;
    let sample_y = move |s: f32| ((ay * s + by) * s + cy) * s;
    let slope_x = move |s: f32| (3.0 * ax * s + 2.0 * bx) * s + cx;
    Easing::Custom(std::rc::Rc::new(move |t: f32| {
        let t = t.clamp(0.0, 1.0);
        // Solve x(s) = t: Newton first, bisection if the slope is flat.
        let mut s = t;
        for _ in 0..8 {
            let error = sample_x(s) - t;
            if error.abs() < 1e-6 {
                return sample_y(s);
            }
            let slope = slope_x(s);
            if slope.abs() < 1e-6 {
                break;
            }
            s = (s - error / slope).clamp(0.0, 1.0);
        }
        let (mut low, mut high) = (0.0_f32, 1.0_f32);
        s = t;
        for _ in 0..32 {
            let x = sample_x(s);
            if (x - t).abs() < 1e-6 {
                break;
            }
            if x < t {
                low = s;
            } else {
                high = s;
            }
            s = (low + high) / 2.0;
        }
        sample_y(s)
    }))
}

/// Milliseconds as a [`Duration`]. Kirakira's timing props are milliseconds, as on the web.
pub const fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

/// A delay for [`Timing::delay`], in milliseconds.
pub fn delay_ms(millis: u64) -> SignedDuration {
    SignedDuration::positive(ms(millis))
}

/// The id of the `index`th keyed child of `id`, for per-letter or per-particle state.
pub fn child_id(id: &gpui_kit::ElementId, index: usize) -> gpui_kit::ElementId {
    (id.clone(), gpui_kit::SharedString::from(index.to_string())).into()
}

/// Cheap, stable pseudo-random numbers in `[0, 1)` from an index: the same every frame, so output
/// stays deterministic. Golden-ratio seeds (`0.618034`, `0.754878`, `0.414214`) scatter neighbours.
pub fn hash(index: usize, seed: f32) -> f32 {
    ((index as f32 + 1.0) * seed).fract()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cubic_bezier_overshoots_like_css() {
        let spring = cubic_bezier(0.34, 1.56, 0.64, 1.0);
        let peak = (1..100)
            .map(|i| spring.sample(i as f32 / 100.0))
            .fold(0.0_f32, f32::max);
        assert!(peak > 1.09, "spring peaks at {peak}");
        assert_eq!(spring.sample(0.0), 0.0);
        assert!((spring.sample(1.0) - 1.0).abs() < 1e-5);
        // Matches CSS `ease` where nothing overshoots.
        let ease = cubic_bezier(0.25, 0.1, 0.25, 1.0);
        assert!((ease.sample(0.5) - 0.802).abs() < 2e-3);
    }
}
