//! Motion that follows a state change, for the display pieces of the base kit (accordion,
//! collapsible, tabs, card).
//!
//! Everything here reads [`motion::now`](crate::motion::now), so the gallery's shot mode pins
//! exact frames of it, the way it does for [`Pulse`](crate::motion::Pulse):
//!
//! - [`since_change`]: how long ago a value last changed, for one-shot keyframes that play on a
//!   change (a panel opening, a pill landing).
//! - [`glide`]: a CSS `transition` toward a target value, retargeted from wherever it is.
//! - [`Reveal`]: a clipped, measured height that can run past the content's own height, for
//!   panels that open with an overshoot.

use std::time::{Duration, Instant};

use gpui_kit::base::Interpolate;
use gpui_kit::{
    AnyElement, App, AvailableSpace, Bounds, ContentMask, Element, ElementId, GlobalElementId,
    InspectorElementId, IntoElement, LayoutId, Pixels, Style, Window, px, relative, size,
};

use crate::motion::{Easing, now};

struct Seen<T> {
    value: T,
    changed: Option<Instant>,
}

/// Time since `value` last changed under `id`, or `None` if it hasn't changed since the element
/// first rendered. Reduced motion is the caller's to handle: a reduced change may still fade.
pub(crate) fn since_change<T: PartialEq + Clone + 'static>(
    id: impl Into<ElementId>,
    value: T,
    window: &mut Window,
    cx: &mut App,
) -> Option<Duration> {
    let now = now();
    let state = window.use_keyed_state(id.into(), cx, |_, _| Seen {
        value: value.clone(),
        changed: None,
    });
    if state.read(cx).value != value {
        state.update(cx, |seen, _| {
            seen.value = value;
            seen.changed = Some(now);
        });
    }
    state
        .read(cx)
        .changed
        .map(|changed| now.saturating_duration_since(changed))
}

/// `elapsed` while it is under `duration`, asking for the next frame; `None` once it has passed.
pub(crate) fn running(
    elapsed: Option<Duration>,
    duration: Duration,
    window: &mut Window,
) -> Option<Duration> {
    let elapsed = elapsed.filter(|elapsed| *elapsed < duration)?;
    window.request_animation_frame();
    Some(elapsed)
}

/// Linear progress through `duration` at `elapsed`, `0..=1`.
pub(crate) fn progress(elapsed: Duration, duration: Duration) -> f32 {
    if duration.is_zero() {
        return 1.0;
    }
    (elapsed.as_secs_f32() / duration.as_secs_f32()).clamp(0.0, 1.0)
}

struct Glide<T> {
    from: T,
    target: T,
    easing: Easing,
    duration: Duration,
    started: Option<Instant>,
}

impl<T: Interpolate> Glide<T> {
    fn sample(&self, now: Instant) -> (T, bool) {
        let Some(started) = self.started else {
            return (self.target.clone(), false);
        };
        let t = progress(now.saturating_duration_since(started), self.duration);
        (
            self.from.interpolate(&self.target, self.easing.sample(t)),
            t < 1.0,
        )
    }
}

/// A CSS `transition` toward `target`: the first value is adopted as is; a new target starts from
/// the value shown at that moment and eases there over `duration` on `easing`. Under reduced
/// motion it jumps.
pub(crate) fn glide<T: Interpolate + PartialEq + 'static>(
    id: impl Into<ElementId>,
    target: T,
    duration: Duration,
    easing: Easing,
    window: &mut Window,
    cx: &mut App,
) -> T {
    let now = now();
    let state = window.use_keyed_state(id.into(), cx, |_, _| Glide {
        from: target.clone(),
        target: target.clone(),
        easing: easing.clone(),
        duration,
        started: None,
    });
    let reduced = cx.reduce_motion();
    if state.read(cx).target != target {
        let (current, _) = state.read(cx).sample(now);
        state.update(cx, |glide, _| {
            glide.from = current;
            glide.target = target.clone();
            glide.easing = easing;
            glide.duration = duration;
            glide.started = (!reduced).then_some(now);
        });
    }
    if reduced {
        return target;
    }
    let (value, moving) = state.read(cx).sample(now);
    if moving {
        window.request_animation_frame();
    }
    value
}

/// A clipped vertical reveal whose height is `natural × fraction + extra`, where `natural` is the
/// content's measured height. `extra` lets a panel run a few pixels past its content and settle,
/// which gpui-base's `MotionReveal` (a fraction only) can't.
pub(crate) struct Reveal {
    id: ElementId,
    fraction: f32,
    extra: Pixels,
    child: AnyElement,
}

impl Reveal {
    pub(crate) fn new(
        id: impl Into<ElementId>,
        fraction: f32,
        extra: Pixels,
        child: impl IntoElement,
    ) -> Self {
        Self {
            id: id.into(),
            fraction: fraction.max(0.0),
            extra,
            child: child.into_any_element(),
        }
    }
}

/// The revealed height for content `natural` tall.
pub(crate) fn reveal_height(natural: Pixels, fraction: f32, extra: Pixels) -> Pixels {
    (natural * fraction + extra).max(px(0.))
}

#[derive(Clone, Copy, Default)]
struct RevealState {
    natural: Option<Pixels>,
}

impl IntoElement for Reveal {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for Reveal {
    type RequestLayoutState = ();
    type PrepaintState = bool;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        global_id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let natural = window.with_element_state(
            global_id.expect("Reveal has an id"),
            |state: Option<RevealState>, _| {
                let state = state.unwrap_or_default();
                (state.natural, state)
            },
        );
        let mut style = Style::default();
        style.size.width = relative(1.0).into();
        // Until the content has been measured, hold it closed; the next frame has its height.
        let height = natural
            .map(|natural| reveal_height(natural, self.fraction, self.extra))
            .unwrap_or_default();
        style.size.height = height.into();
        (window.request_layout(style, None, cx), ())
    }

    fn prepaint(
        &mut self,
        global_id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> bool {
        let measured = self.child.layout_as_root(
            size(
                AvailableSpace::Definite(bounds.size.width),
                AvailableSpace::MinContent,
            ),
            window,
            cx,
        );
        let changed = window.with_element_state(
            global_id.expect("Reveal has an id"),
            |state: Option<RevealState>, _| {
                let mut state = state.unwrap_or_default();
                let changed = state.natural != Some(measured.height);
                state.natural = Some(measured.height);
                (changed, state)
            },
        );
        if changed {
            window.request_animation_frame();
        }
        let visible = bounds.size.height > px(0.);
        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            self.child.prepaint_at(bounds.origin, window, cx);
        });
        visible
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        visible: &mut bool,
        window: &mut Window,
        cx: &mut App,
    ) {
        if !*visible {
            return;
        }
        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            self.child.paint(window, cx);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reveal_height_runs_past_and_never_below_zero() {
        assert_eq!(reveal_height(px(100.), 1.0, px(6.)), px(106.));
        assert_eq!(reveal_height(px(100.), 0.5, px(0.)), px(50.));
        assert_eq!(reveal_height(px(100.), 0.0, px(-2.)), px(0.));
    }

    #[test]
    fn progress_clamps() {
        assert_eq!(
            progress(Duration::from_millis(50), Duration::from_millis(100)),
            0.5
        );
        assert_eq!(
            progress(Duration::from_millis(500), Duration::from_millis(100)),
            1.0
        );
        assert_eq!(progress(Duration::from_millis(5), Duration::ZERO), 1.0);
    }
}
