//! A CSS `transition` on one number, timed by Kirakira's clock.
//!
//! gpui-base's `transition` reads the executor's clock, which screenshot mode can't pin. The form
//! controls glide on [`motion::now`](crate::motion::now) instead, so every frame of a hover or a
//! fill can be captured exactly. Like a CSS transition, a new target starts from wherever the value
//! is, and the duration and curve can differ per direction (the caller passes the ones for the
//! target it asks for).

use std::time::{Duration, Instant};

use gpui_kit::{App, ElementId, Window};

use crate::motion::Easing;

struct GlideState {
    from: f32,
    target: f32,
    started: Instant,
    duration: Duration,
    easing: Easing,
}

impl GlideState {
    fn sample(&self, now: Instant) -> (f32, bool) {
        let elapsed = now.saturating_duration_since(self.started);
        if self.duration.is_zero() || elapsed >= self.duration {
            return (self.target, false);
        }
        let progress = elapsed.as_secs_f32() / self.duration.as_secs_f32();
        let eased = self.easing.sample(progress);
        (self.from + (self.target - self.from) * eased, true)
    }
}

/// The value of a transition towards `target`, keyed by `id`.
///
/// The first render adopts `target` without moving. Each new target starts a `duration`-long
/// glide on `easing` from the current value. Under reduced motion the value jumps.
pub(crate) fn glide(
    id: impl Into<ElementId>,
    target: f32,
    duration: Duration,
    easing: Easing,
    window: &mut Window,
    cx: &mut App,
) -> f32 {
    let reduced = cx.reduce_motion();
    glide_inner(id, target, duration, easing, reduced, window, cx)
}

/// [`glide`] that also runs under reduced motion, for a change that should still ease there (the
/// caller passes the calmer duration and curve).
pub(crate) fn glide_always(
    id: impl Into<ElementId>,
    target: f32,
    duration: Duration,
    easing: Easing,
    window: &mut Window,
    cx: &mut App,
) -> f32 {
    glide_inner(id, target, duration, easing, false, window, cx)
}

fn glide_inner(
    id: impl Into<ElementId>,
    target: f32,
    duration: Duration,
    easing: Easing,
    reduced: bool,
    window: &mut Window,
    cx: &mut App,
) -> f32 {
    let now = crate::motion::now();
    let state = window.use_keyed_state(id.into(), cx, |_, _| GlideState {
        from: target,
        target,
        started: now,
        duration: Duration::ZERO,
        easing: Easing::Linear,
    });
    let (value, running) = state.read(cx).sample(now);
    if state.read(cx).target != target {
        state.update(cx, |state, _| {
            state.from = value;
            state.target = target;
            state.started = now;
            state.duration = if reduced { Duration::ZERO } else { duration };
            state.easing = easing;
        });
        if reduced {
            return target;
        }
        window.request_animation_frame();
        return value;
    }
    if reduced {
        return target;
    }
    if running {
        window.request_animation_frame();
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_glide_eases_from_where_it_was() {
        let start = Instant::now();
        let state = GlideState {
            from: 0.0,
            target: 10.0,
            started: start,
            duration: Duration::from_millis(100),
            easing: Easing::Linear,
        };
        assert_eq!(state.sample(start), (0.0, true));
        let (mid, running) = state.sample(start + Duration::from_millis(50));
        assert!((mid - 5.0).abs() < 1e-4 && running);
        assert_eq!(
            state.sample(start + Duration::from_millis(100)),
            (10.0, false)
        );
    }
}
